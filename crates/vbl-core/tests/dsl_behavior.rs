use std::sync::Arc;
use std::time::Duration;

use vbl_core::dsl::MacroProgram;
use vbl_core::executor::TaskKind;
use vbl_core::input::{KeyAction, MouseButton};
use vbl_core::macros::{MacroSource, ProfileRunner};
use vbl_core::profile::VblSettings;
use vbl_core::testing::{MockInputSink, RecordedInput};
use vbl_core::time::MockClock;

const VBL_PROGRAM: &str = r#"
{
  "macros": [
    { "on": "respawn", "do": [ {"tap":"escape"}, {"tap":"r"}, {"tap":"enter"} ] },

    { "on": "ult", "do": [ {"toggle":"skillEnabled"} ] },

    { "on": "X1.down", "when": {"GameOnGround": true, "X2Held": false},
      "do": [ {"tap":"space"}, {"hold":"$jumpset_key", "until":"X1.up"} ] },

    { "on": "X1.down", "when": {"any":[ {"X2Held":true}, {"not":"GameOnGround"} ]},
      "do": [ {"wait":25}, {"hold":"$jumpset_key", "until":"X1.up"} ] },

    { "on": "X2.down", "do": [ {"set_state":"X2Held", "value":true} ] },

    { "on": "X2Held.held", "while": {"GameOnGround": true},
      "do": [ {"tap":"shift"}, {"tap":"space"}, {"tap":"shift"} ] },

    { "on": "X2.up",
      "do": [
        {"set_state":"X2Held", "value":false},
        {"if": {"all":["skillEnabled","GameUltimateReady",{"not":"GameOnGround"}]},
         "then": [ {"if": {"eq":["$skill","normal"]},
                    "then": [ {"tap":"$skill_key"} ],
                    "else": [ {"wait":25} ] } ]},
        {"click":"left"}
      ] }
  ]
}
"#;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn kp(name: &str) -> RecordedInput {
    RecordedInput::Key {
        key: name.to_string(),
        action: KeyAction::Press,
    }
}

fn ku(name: &str) -> RecordedInput {
    RecordedInput::Key {
        key: name.to_string(),
        action: KeyAction::Release,
    }
}

fn setup(settings: VblSettings) -> (ProfileRunner<MacroProgram>, Arc<MockInputSink>) {
    let program = MacroProgram::from_json(VBL_PROGRAM).unwrap();
    program.validate(&settings).unwrap();
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let mut runner = ProfileRunner::new(clock, sink.clone(), program, settings);
    runner.set_armed(true);
    runner.set_focused(true);
    (runner, sink)
}

#[test]
fn program_parses_and_validates() {
    let program = MacroProgram::from_json(VBL_PROGRAM).unwrap();
    assert!(program.validate(&VblSettings::default()).is_ok());
    assert_eq!(program.loop_names(), vec!["X2Held".to_string()]);
}

#[test]
fn dsl_respawn_matches_builtin_trace() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.fire("respawn", TaskKind::Respawn).unwrap();
    runner.advance(ms(200));
    assert_eq!(
        sink.log(),
        vec![
            (ms(0), kp("escape")),
            (ms(35), ku("escape")),
            (ms(35), kp("r")),
            (ms(70), ku("r")),
            (ms(70), kp("enter")),
            (ms(105), ku("enter")),
        ]
    );
}

#[test]
fn dsl_toggle_flips_skill_enabled() {
    let (mut runner, sink) = setup(VblSettings::default());
    assert!(runner.state().skill_enabled);
    runner.fire("ult", TaskKind::Other).unwrap();
    assert!(!runner.state().skill_enabled);
    assert!(sink.log().is_empty());
}

#[test]
fn dsl_x1_grounded_taps_space_then_holds_until_cancel() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.set_on_ground(true);

    runner.fire("X1.down", TaskKind::X1).unwrap();
    runner.advance(ms(50));
    assert_eq!(
        sink.log(),
        vec![
            (ms(0), kp("space")),
            (ms(35), ku("space")),
            (ms(35), kp("e"))
        ]
    );

    runner.cancel(TaskKind::X1);
    assert_eq!(*sink.log().last().unwrap(), (ms(50), ku("e")));
}

#[test]
fn dsl_x1_airborne_waits_then_holds() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.fire("X1.down", TaskKind::X1).unwrap();
    runner.advance(ms(50));
    assert_eq!(sink.log(), vec![(ms(25), kp("e"))]);
}

#[test]
fn dsl_x2_held_loop_taps_shift_space_shift() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.set_on_ground(true);
    runner.fire("X2.down", TaskKind::Other).unwrap();
    assert!(runner.state().x2_held);
    assert!(runner.loop_active());

    let mut t = 0u64;
    for _ in 0..2 {
        runner.pump();
        t += 105;
        runner.advance(ms(t));
    }

    assert_eq!(
        sink.log()[..6].to_vec(),
        vec![
            (ms(0), kp("lshift")),
            (ms(35), ku("lshift")),
            (ms(35), kp("space")),
            (ms(70), ku("space")),
            (ms(70), kp("lshift")),
            (ms(105), ku("lshift")),
        ]
    );
    assert_eq!(sink.log().len(), 12);
}

#[test]
fn dsl_x2_up_normal_skill_when_ult_airborne_then_clicks() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.set_ultimate_ready(true);

    runner.fire("X2.up", TaskKind::X2Spike).unwrap();
    runner.advance(ms(100));

    assert_eq!(
        sink.log(),
        vec![
            (ms(0), kp("lctrl")),
            (ms(35), ku("lctrl")),
            (
                ms(35),
                RecordedInput::Mouse {
                    button: MouseButton::Left,
                    action: KeyAction::Press
                }
            ),
            (
                ms(70),
                RecordedInput::Mouse {
                    button: MouseButton::Left,
                    action: KeyAction::Release
                }
            ),
        ]
    );
}

#[test]
fn dsl_x2_up_plain_click_when_not_ult() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.fire("X2.up", TaskKind::X2Spike).unwrap();
    runner.advance(ms(100));
    assert_eq!(
        sink.log(),
        vec![
            (
                ms(0),
                RecordedInput::Mouse {
                    button: MouseButton::Left,
                    action: KeyAction::Press
                }
            ),
            (
                ms(35),
                RecordedInput::Mouse {
                    button: MouseButton::Left,
                    action: KeyAction::Release
                }
            ),
        ]
    );
}

#[test]
fn dsl_not_armed_injects_nothing() {
    let program = MacroProgram::from_json(VBL_PROGRAM).unwrap();
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let mut runner = ProfileRunner::new(clock, sink.clone(), program, VblSettings::default());
    runner.set_focused(true);
    runner.fire("respawn", TaskKind::Respawn).unwrap();
    runner.advance(ms(200));
    assert!(sink.log().is_empty());
}

#[test]
fn dsl_losing_focus_mid_hold_releases_everything() {
    let (mut runner, sink) = setup(VblSettings::default());
    runner.set_on_ground(true);
    runner.fire("X1.down", TaskKind::X1).unwrap();
    runner.advance(ms(50));
    runner.set_focused(false);
    assert_eq!(
        *sink.log().last().unwrap(),
        (ms(50), RecordedInput::ReleaseAll)
    );
}

#[test]
fn invalid_program_is_rejected_by_validate() {
    let bad = r#"{ "macros": [ { "on":"x", "do":[ {"tap":"not_a_key"} ] } ] }"#;
    let program = MacroProgram::from_json(bad).unwrap();
    assert!(program.validate(&VblSettings::default()).is_err());
}

#[test]
fn malformed_json_is_an_error() {
    assert!(MacroProgram::from_json("{ not json").is_err());
}

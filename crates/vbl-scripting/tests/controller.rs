use std::sync::Arc;
use std::time::Duration;

use vbl_core::executor::TaskKind;
use vbl_core::input::{Key, KeyAction, MouseButton};
use vbl_core::profile::VblSettings;
use vbl_core::testing::{MockInputSink, RecordedInput};
use vbl_core::time::MockClock;
use vbl_scripting::{ScriptController, ScriptHost};

const VBL_SCRIPT: &str = r#"
    vbl.on("respawn", function()
        vbl.tap("escape"); vbl.tap("r"); vbl.tap("enter")
    end)

    vbl.on("ult", function() vbl.toggle("skillEnabled") end)

    vbl.on("X1.down", function()
        if vbl.state.GameOnGround and not vbl.state.X2Held then
            vbl.tap("space")
            vbl.down(vbl.settings.jumpset_key)
        else
            vbl.wait(25)
            vbl.down(vbl.settings.jumpset_key)
        end
    end)

    vbl.on("X1.up", function() vbl.up(vbl.settings.jumpset_key) end)

    vbl.on("X2.down", function() vbl.set_state("X2Held", true) end)

    vbl.every("X2Held", function()
        if vbl.state.GameOnGround then
            vbl.tap("shift"); vbl.tap("space"); vbl.tap("shift")
        end
        vbl.wait(1)
    end)

    vbl.on("X2.up", function()
        vbl.set_state("X2Held", false)
        vbl.click("left")
    end)
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

fn setup() -> (ScriptController, Arc<MockInputSink>) {
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let host = ScriptHost::load(VBL_SCRIPT).unwrap();
    let mut ctrl = ScriptController::new(clock, sink.clone(), host, VblSettings::default());
    ctrl.set_armed(true);
    ctrl.set_focused(true);
    (ctrl, sink)
}

#[test]
fn scripted_respawn_matches_builtin_trace() {
    let (mut ctrl, sink) = setup();
    ctrl.fire("respawn", TaskKind::Respawn).unwrap();
    ctrl.advance(ms(200));

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
fn scripted_toggle_flips_skill_enabled_with_no_input() {
    let (mut ctrl, sink) = setup();
    assert!(ctrl.state().skill_enabled);
    ctrl.fire("ult", TaskKind::Other).unwrap();
    assert!(!ctrl.state().skill_enabled);
    ctrl.fire("ult", TaskKind::Other).unwrap();
    assert!(ctrl.state().skill_enabled);
    assert!(sink.log().is_empty());
}

#[test]
fn scripted_x1_grounded_taps_space_then_holds_jumpset_until_up() {
    let (mut ctrl, sink) = setup();
    ctrl.set_on_ground(true);

    ctrl.fire("X1.down", TaskKind::X1).unwrap();
    ctrl.advance(ms(50));

    assert_eq!(
        sink.log(),
        vec![
            (ms(0), kp("space")),
            (ms(35), ku("space")),
            (ms(35), kp("e")),
        ]
    );

    ctrl.fire("X1.up", TaskKind::X1).unwrap();
    ctrl.advance(ms(60));
    assert_eq!(*sink.log().last().unwrap(), (ms(50), ku("e")));
}

#[test]
fn scripted_x1_airborne_waits_then_holds_jumpset() {
    let (mut ctrl, sink) = setup();

    ctrl.fire("X1.down", TaskKind::X1).unwrap();
    ctrl.advance(ms(50));
    assert_eq!(sink.log(), vec![(ms(25), kp("e"))]);
}

#[test]
fn scripted_x2_held_loop_taps_shift_space_shift_per_iteration() {
    let (mut ctrl, sink) = setup();
    ctrl.set_on_ground(true);
    ctrl.fire("X2.down", TaskKind::Other).unwrap();
    assert!(ctrl.state().x2_held);

    let mut t = 0u64;
    for _ in 0..2 {
        ctrl.pump();
        t += 106;
        ctrl.advance(ms(t));
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
fn scripted_x2_up_clears_held_and_clicks() {
    let (mut ctrl, sink) = setup();
    ctrl.set_on_ground(true);
    ctrl.fire("X2.down", TaskKind::Other).unwrap();
    ctrl.fire("X2.up", TaskKind::X2Spike).unwrap();
    ctrl.advance(ms(50));

    assert!(!ctrl.state().x2_held);
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

    ctrl.pump();
    ctrl.advance(ms(200));
    assert_eq!(sink.log().len(), 2);
}

#[test]
fn not_armed_injects_nothing() {
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let host = ScriptHost::load(VBL_SCRIPT).unwrap();
    let mut ctrl = ScriptController::new(clock, sink.clone(), host, VblSettings::default());
    ctrl.set_focused(true);

    ctrl.fire("respawn", TaskKind::Respawn).unwrap();
    ctrl.advance(ms(200));
    assert!(sink.log().is_empty());
}

#[test]
fn losing_focus_mid_hold_releases_everything() {
    let (mut ctrl, sink) = setup();
    ctrl.set_on_ground(true);
    ctrl.fire("X1.down", TaskKind::X1).unwrap();
    ctrl.advance(ms(50));
    assert!(sink.log().iter().any(|(_, a)| *a == kp("e")));

    ctrl.set_focused(false);
    assert_eq!(
        *sink.log().last().unwrap(),
        (ms(50), RecordedInput::ReleaseAll)
    );
}

#[test]
fn log_lines_are_collected() {
    let host = ScriptHost::load(r#"vbl.on("hi", function() vbl.log("hello") end)"#).unwrap();
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let mut ctrl = ScriptController::new(clock, sink, host, VblSettings::default());
    ctrl.set_armed(true);
    ctrl.set_focused(true);

    ctrl.fire("hi", TaskKind::Other).unwrap();
    assert_eq!(ctrl.take_logs(), vec!["hello".to_string()]);
    assert!(ctrl.take_logs().is_empty());
}

#[test]
fn key_helper_normalizes_consistently() {
    assert_eq!(Key::parse("escape").unwrap().as_str(), "escape");
}

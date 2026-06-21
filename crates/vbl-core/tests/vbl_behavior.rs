use std::sync::Arc;
use std::time::Duration;

use vbl_core::input::{Key, KeyAction, MouseButton};
use vbl_core::profile::{MacroKeybinds, SkillMode, VblSettings};
use vbl_core::testing::{MockInputSink, RecordedInput};
use vbl_core::time::MockClock;
use vbl_core::Vbl;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn key(name: &str) -> RecordedInput {
    RecordedInput::Key {
        key: Key::parse(name).unwrap().as_str().to_string(),
        action: KeyAction::Press,
    }
}

fn key_up(name: &str) -> RecordedInput {
    RecordedInput::Key {
        key: Key::parse(name).unwrap().as_str().to_string(),
        action: KeyAction::Release,
    }
}

fn click_down() -> RecordedInput {
    RecordedInput::Mouse {
        button: MouseButton::Left,
        action: KeyAction::Press,
    }
}

fn click_up() -> RecordedInput {
    RecordedInput::Mouse {
        button: MouseButton::Left,
        action: KeyAction::Release,
    }
}

fn setup(skill: SkillMode) -> (Vbl, Arc<MockClock>, Arc<MockInputSink>) {
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let settings = VblSettings {
        skill,
        ..Default::default()
    };
    let mut vbl = Vbl::new(clock.clone(), sink.clone(), settings);
    vbl.set_armed(true);
    vbl.set_focused(true);
    (vbl, clock, sink)
}

#[test]
fn respawn_taps_esc_r_enter() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    vbl.key_down(&Key::parse("f1").unwrap());
    vbl.advance(ms(200));

    assert_eq!(
        sink.log(),
        vec![
            (ms(0), key("escape")),
            (ms(35), key_up("escape")),
            (ms(35), key("r")),
            (ms(70), key_up("r")),
            (ms(70), key("enter")),
            (ms(105), key_up("enter")),
        ]
    );
}

#[test]
fn toggle_ultimate_flips_skill_enabled() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    assert!(vbl.state().skill_enabled);

    vbl.key_down(&Key::parse("f2").unwrap());
    assert!(!vbl.state().skill_enabled);

    vbl.key_down(&Key::parse("f2").unwrap());
    assert!(vbl.state().skill_enabled);

    assert!(sink.actions().is_empty());
}

#[test]
fn x1_grounded_jumps_then_holds_jumpset() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    vbl.set_on_ground(true);

    vbl.press_x1();
    vbl.advance(ms(100));

    assert_eq!(
        sink.log(),
        vec![
            (ms(0), key("space")),
            (ms(35), key_up("space")),
            (ms(35), key("e")),
        ]
    );

    vbl.press_x1_up();
    assert_eq!(sink.log().last().unwrap(), &(ms(100), key_up("e")));
}

#[test]
fn x1_airborne_waits_25_then_holds_jumpset() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);

    vbl.press_x1();
    vbl.advance(ms(50));

    assert_eq!(sink.log(), vec![(ms(25), key("e"))]);
}

#[test]
fn x1_airborne_boomjump_ult_ready_waits_100() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Boomjump);
    vbl.set_ultimate_ready(true);

    vbl.press_x1();
    vbl.advance(ms(150));

    assert_eq!(sink.log(), vec![(ms(100), key("e"))]);
}

#[test]
fn x2_loop_grounded_normal_cycles_shift_space_shift() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    vbl.set_on_ground(true);

    vbl.press_x2();
    vbl.advance(ms(105));

    assert_eq!(
        sink.log(),
        vec![
            (ms(0), key("shift")),
            (ms(35), key_up("shift")),
            (ms(35), key("space")),
            (ms(70), key_up("space")),
            (ms(70), key("shift")),
            (ms(105), key_up("shift")),
        ]
    );
}

#[test]
fn x2_loop_grounded_boomjump_ult_cycles_shift_skill_shift() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Boomjump);
    vbl.set_on_ground(true);
    vbl.set_ultimate_ready(true);

    vbl.press_x2();
    vbl.advance(ms(130));

    assert_eq!(
        sink.log(),
        vec![
            (ms(0), key("shift")),
            (ms(35), key_up("shift")),
            (ms(35), key("lctrl")),
            (ms(70), key_up("lctrl")),
            (ms(95), key("shift")),
            (ms(130), key_up("shift")),
        ]
    );
}

#[test]
fn x2_up_spike_normal_ult_ready_airborne() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    vbl.set_ultimate_ready(true);

    vbl.press_x2();
    vbl.advance(ms(10));
    vbl.press_x2_up();
    vbl.advance(ms(200));

    assert_eq!(
        sink.log(),
        vec![
            (ms(10), key("lctrl")),
            (ms(45), key_up("lctrl")),
            (ms(45), click_down()),
            (ms(80), click_up()),
        ]
    );
}

#[test]
fn x2_up_spike_boomjump_ult_ready_airborne_waits_then_clicks() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Boomjump);
    vbl.set_ultimate_ready(true);

    vbl.press_x2();
    vbl.press_x2_up();
    vbl.advance(ms(200));

    assert_eq!(
        sink.log(),
        vec![(ms(25), click_down()), (ms(60), click_up())]
    );
}

#[test]
fn x2_up_spike_plain_click_when_not_ult_ready() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);

    vbl.press_x2();
    vbl.press_x2_up();
    vbl.advance(ms(100));

    assert_eq!(
        sink.log(),
        vec![(ms(0), click_down()), (ms(35), click_up())]
    );
}

#[test]
fn x2_up_spike_suppressed_while_x1_held() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    vbl.set_on_ground(true);
    vbl.set_ultimate_ready(true);

    vbl.press_x1();
    vbl.advance(ms(50));
    vbl.press_x2();
    vbl.press_x2_up();
    vbl.advance(ms(300));

    assert!(
        !sink.actions().iter().any(|a| matches!(
            a,
            RecordedInput::Mouse {
                button: MouseButton::Left,
                ..
            }
        )),
        "spike must be suppressed while X1 is held"
    );
}

#[test]
fn gate_loss_releases_everything_and_clears_state() {
    let (mut vbl, _clock, sink) = setup(SkillMode::Normal);
    vbl.set_on_ground(true);

    vbl.press_x1();
    vbl.advance(ms(50));
    assert!(vbl.state().x1_held);

    vbl.set_focused(false);

    assert_eq!(sink.log().last().unwrap().1, RecordedInput::ReleaseAll);
    assert!(!vbl.state().x1_held);
    assert!(!vbl.state().target_focused);
}

#[test]
fn nothing_happens_when_not_armed() {
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let mut vbl = Vbl::new(clock.clone(), sink.clone(), VblSettings::default());

    vbl.press_x1();
    vbl.key_down(&Key::parse("f1").unwrap());
    vbl.advance(ms(200));

    assert!(sink.actions().is_empty());
    assert!(!vbl.state().x1_held);
}

#[test]
fn respawn_keybind_is_remappable() {
    let clock = Arc::new(MockClock::new());
    let sink = Arc::new(MockInputSink::new(clock.clone()));
    let settings = VblSettings {
        macro_keys: MacroKeybinds {
            respawn_key: "f4".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut vbl = Vbl::new(clock.clone(), sink.clone(), settings);
    vbl.set_armed(true);
    vbl.set_focused(true);

    vbl.key_down(&Key::parse("f1").unwrap());
    vbl.advance(ms(200));
    assert!(sink.actions().is_empty());

    vbl.key_down(&Key::parse("f4").unwrap());
    vbl.advance(ms(200));
    assert_eq!(sink.actions().first(), Some(&key("escape")));
}

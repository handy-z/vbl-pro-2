use std::time::Duration;

use vbl_core::executor::Step;
use vbl_core::input::{Key, MouseButton};
use vbl_core::profile::{SkillMode, VblSettings};
use vbl_core::state::{EngineState, StateKey};
use vbl_scripting::{ScriptAction, ScriptContext, ScriptHost};

fn key(s: &str) -> Key {
    Key::parse(s).unwrap()
}

fn ctx<'a>(state: &'a EngineState, settings: &'a VblSettings) -> ScriptContext<'a> {
    ScriptContext { state, settings }
}

#[test]
fn respawn_lowers_to_three_taps_with_default_hold() {
    let host = ScriptHost::load(
        r#"
        vbl.on("respawn", function()
            vbl.tap("escape")
            vbl.tap("r")
            vbl.tap("enter")
        end)
    "#,
    )
    .unwrap();

    let state = EngineState::default();
    let settings = VblSettings::default();
    let out = host.trigger("respawn", &ctx(&state, &settings)).unwrap();

    assert_eq!(
        out.steps(),
        vec![
            Step::Tap(key("escape"), Duration::from_millis(35)),
            Step::Tap(key("r"), Duration::from_millis(35)),
            Step::Tap(key("enter"), Duration::from_millis(35)),
        ]
    );
}

#[test]
fn explicit_hold_ms_overrides_default() {
    let host = ScriptHost::load(r#"vbl.on("t", function() vbl.tap("space", 100) end)"#).unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    let out = host.trigger("t", &ctx(&state, &settings)).unwrap();
    assert_eq!(
        out.steps(),
        vec![Step::Tap(key("space"), Duration::from_millis(100))]
    );
}

#[test]
fn all_action_primitives_map_correctly() {
    let host = ScriptHost::load(
        r#"
        vbl.on("x", function()
            vbl.down("e")
            vbl.wait(25)
            vbl.click("left", 35)
            vbl.up("e")
            vbl.release_all()
        end)
    "#,
    )
    .unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    let out = host.trigger("x", &ctx(&state, &settings)).unwrap();
    assert_eq!(
        out.steps(),
        vec![
            Step::Press(key("e")),
            Step::Wait(Duration::from_millis(25)),
            Step::Click(MouseButton::Left, Duration::from_millis(35)),
            Step::Release(key("e")),
            Step::ReleaseAll,
        ]
    );
}

#[test]
fn handler_reads_state_and_settings_to_branch() {
    let host = ScriptHost::load(
        r#"
        vbl.on("X1.down", function()
            if vbl.state.GameOnGround and not vbl.state.X2Held then
                vbl.tap("space")
                vbl.down(vbl.settings.jumpset_key)
            else
                vbl.wait(25)
                vbl.down(vbl.settings.jumpset_key)
            end
        end)
    "#,
    )
    .unwrap();
    let settings = VblSettings::default();

    let grounded = EngineState {
        game_on_ground: true,
        ..EngineState::default()
    };
    let out = host.trigger("X1.down", &ctx(&grounded, &settings)).unwrap();
    assert_eq!(
        out.steps(),
        vec![
            Step::Tap(key("space"), Duration::from_millis(35)),
            Step::Press(key("e"))
        ]
    );

    let airborne = EngineState::default();
    let out = host.trigger("X1.down", &ctx(&airborne, &settings)).unwrap();
    assert_eq!(
        out.steps(),
        vec![Step::Wait(Duration::from_millis(25)), Step::Press(key("e"))]
    );
}

#[test]
fn skill_mode_setting_is_visible() {
    let host =
        ScriptHost::load(r#"vbl.on("t", function() vbl.log(vbl.settings.skill) end)"#).unwrap();
    let state = EngineState::default();
    let settings = VblSettings {
        skill: SkillMode::Boomjump,
        ..VblSettings::default()
    };
    let out = host.trigger("t", &ctx(&state, &settings)).unwrap();
    assert_eq!(out.actions, vec![ScriptAction::Log("boomjump".to_string())]);
}

#[test]
fn toggle_and_set_state_map_to_actions() {
    let host = ScriptHost::load(
        r#"
        vbl.on("ult", function() vbl.toggle("skillEnabled") end)
        vbl.on("ground", function() vbl.set_state("GameOnGround", true) end)
    "#,
    )
    .unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());

    let out = host.trigger("ult", &ctx(&state, &settings)).unwrap();
    assert_eq!(
        out.actions,
        vec![ScriptAction::Toggle(StateKey::SkillEnabled)]
    );

    let out = host.trigger("ground", &ctx(&state, &settings)).unwrap();
    assert_eq!(
        out.actions,
        vec![ScriptAction::SetState(StateKey::GameOnGround, true)]
    );
}

#[test]
fn unknown_trigger_returns_empty() {
    let host = ScriptHost::load(r#"vbl.on("known", function() vbl.tap("e") end)"#).unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    let out = host.trigger("nope", &ctx(&state, &settings)).unwrap();
    assert!(out.is_empty());
}

#[test]
fn every_registers_a_loop_handler() {
    let host = ScriptHost::load(
        r#"
        vbl.every("X2Held", function()
            vbl.tap("shift")
            vbl.tap("space")
            vbl.tap("shift")
        end)
    "#,
    )
    .unwrap();
    assert_eq!(host.loops().unwrap(), vec!["X2Held".to_string()]);
    assert!(host.triggers().unwrap().is_empty());

    let (state, settings) = (EngineState::default(), VblSettings::default());
    let out = host
        .trigger_loop("X2Held", &ctx(&state, &settings))
        .unwrap();
    assert_eq!(out.steps().len(), 3);
}

#[test]
fn triggers_lists_registered_names_sorted() {
    let host = ScriptHost::load(
        r#"
        vbl.on("b", function() end)
        vbl.on("a", function() end)
    "#,
    )
    .unwrap();
    assert_eq!(
        host.triggers().unwrap(),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn buffer_is_reset_between_triggers() {
    let host = ScriptHost::load(r#"vbl.on("t", function() vbl.tap("e") end)"#).unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    let first = host.trigger("t", &ctx(&state, &settings)).unwrap();
    let second = host.trigger("t", &ctx(&state, &settings)).unwrap();
    assert_eq!(first, second);
    assert_eq!(second.steps().len(), 1);
}

#[test]
fn io_is_unavailable_in_the_sandbox() {
    let err = ScriptHost::load(r#"io.write("escape!")"#);
    assert!(err.is_err(), "io must not be reachable from scripts");
}

#[test]
fn os_is_unavailable_in_the_sandbox() {
    let host = ScriptHost::load(r#"vbl.on("t", function() return os.clock() end)"#).unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    assert!(host.trigger("t", &ctx(&state, &settings)).is_err());
}

#[test]
fn runaway_loop_hits_the_instruction_budget() {
    let host = ScriptHost::load_with_budget(
        r#"vbl.on("spin", function() while true do end end)"#,
        50_000,
    )
    .unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    let err = host.trigger("spin", &ctx(&state, &settings));
    assert!(err.is_err(), "runaway loop must abort via the budget");
}

#[test]
fn unknown_key_is_a_script_error() {
    let host = ScriptHost::load(r#"vbl.on("t", function() vbl.tap("not_a_key") end)"#).unwrap();
    let (state, settings) = (EngineState::default(), VblSettings::default());
    assert!(host.trigger("t", &ctx(&state, &settings)).is_err());
}

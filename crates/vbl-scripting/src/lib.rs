use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use mlua::{Function, Lua, Table, VmState};
use vbl_core::executor::Step;
use vbl_core::input::{Key, MouseButton};
use vbl_core::macros::{MacroContext, MacroSource, ProfileRunner};
use vbl_core::profile::{SkillMode, VblSettings};
use vbl_core::state::StateKey;

pub use vbl_core::macros::{
    MacroAction, MacroAction as ScriptAction, MacroError, Outcome, Outcome as ScriptOutcome,
};

pub type ScriptContext<'a> = MacroContext<'a>;

pub type ScriptController = ProfileRunner<ScriptHost>;

pub const DEFAULT_INSTRUCTION_BUDGET: u64 = 1_000_000;

const REG_ON: &str = "vbl_on";
const REG_EVERY: &str = "vbl_every";

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("luau error: {0}")]
    Lua(#[from] mlua::Error),
}

type Result<T> = std::result::Result<T, ScriptError>;

#[derive(Default)]
struct HostData {
    actions: Vec<MacroAction>,
    default_tap_ms: u64,
}

pub struct ScriptHost {
    lua: Lua,
    instr: Rc<Cell<u64>>,
    budget: u64,
}

impl ScriptHost {
    pub fn load(source: &str) -> Result<ScriptHost> {
        Self::load_with_budget(source, DEFAULT_INSTRUCTION_BUDGET)
    }

    pub fn load_with_budget(source: &str, budget: u64) -> Result<ScriptHost> {
        let lua = Lua::new();
        lua.set_app_data(HostData::default());

        let instr = Rc::new(Cell::new(0u64));
        let instr_cb = instr.clone();
        lua.set_interrupt(move |_| {
            let n = instr_cb.get() + 1;
            instr_cb.set(n);
            if n > budget {
                return Err(mlua::Error::RuntimeError(
                    "script instruction budget exceeded".to_string(),
                ));
            }
            Ok(VmState::Continue)
        });

        strip_unsafe_globals(&lua)?;
        install_host_api(&lua)?;

        lua.load(source).set_name("vbl-script").exec()?;

        Ok(ScriptHost { lua, instr, budget })
    }

    pub fn triggers(&self) -> Result<Vec<String>> {
        self.registered(REG_ON)
    }

    pub fn loops(&self) -> Result<Vec<String>> {
        self.registered(REG_EVERY)
    }

    pub fn trigger(&self, name: &str, ctx: &ScriptContext) -> Result<Outcome> {
        self.dispatch(REG_ON, name, ctx)
    }

    pub fn trigger_loop(&self, name: &str, ctx: &ScriptContext) -> Result<Outcome> {
        self.dispatch(REG_EVERY, name, ctx)
    }

    fn dispatch(&self, registry: &str, name: &str, ctx: &ScriptContext) -> Result<Outcome> {
        self.sync_context(ctx)?;
        self.instr.set(0);

        let handlers: Table = self.lua.named_registry_value(registry)?;
        let handler: Option<Function> = handlers.get(name)?;
        if let Some(f) = handler {
            f.call::<()>(())?;
        }

        let actions = self
            .lua
            .app_data_mut::<HostData>()
            .map(|mut d| std::mem::take(&mut d.actions))
            .unwrap_or_default();
        Ok(Outcome { actions })
    }

    fn registered(&self, registry: &str) -> Result<Vec<String>> {
        let handlers: Table = self.lua.named_registry_value(registry)?;
        let mut names = Vec::new();
        for pair in handlers.pairs::<String, Function>() {
            let (name, _) = pair?;
            names.push(name);
        }
        names.sort();
        Ok(names)
    }

    fn sync_context(&self, ctx: &ScriptContext) -> Result<()> {
        if let Some(mut data) = self.lua.app_data_mut::<HostData>() {
            data.actions.clear();
            data.default_tap_ms = ctx.settings.tap_ms;
        }

        let vbl: Table = self.lua.globals().get("vbl")?;

        let state: Table = vbl.get("state")?;
        for key in StateKey::ALL {
            state.set(key.name(), ctx.state.get(key))?;
        }
        state.set("armed", ctx.state.armed)?;
        state.set("targetFocused", ctx.state.target_focused)?;

        let settings: Table = vbl.get("settings")?;
        let mk = &ctx.settings.macro_keys;
        settings.set("skill", skill_name(ctx.settings))?;
        settings.set("jumpset_key", mk.jumpset_key.clone())?;
        settings.set("skill_key", mk.skill_key.clone())?;
        settings.set("toggle_ultimate_key", mk.toggle_ultimate_key.clone())?;
        settings.set("respawn_key", mk.respawn_key.clone())?;
        settings.set("tap_ms", ctx.settings.tap_ms)?;

        Ok(())
    }

    pub fn budget(&self) -> u64 {
        self.budget
    }
}

impl MacroSource for ScriptHost {
    fn on(&self, trigger: &str, ctx: &MacroContext) -> std::result::Result<Outcome, MacroError> {
        self.trigger(trigger, ctx).map_err(to_macro_error)
    }

    fn every(&self, name: &str, ctx: &MacroContext) -> std::result::Result<Outcome, MacroError> {
        self.trigger_loop(name, ctx).map_err(to_macro_error)
    }

    fn loop_names(&self) -> Vec<String> {
        self.loops().unwrap_or_default()
    }
}

fn to_macro_error(err: ScriptError) -> MacroError {
    MacroError(err.to_string())
}

fn skill_name(settings: &VblSettings) -> &'static str {
    match settings.skill {
        SkillMode::Normal => "normal",
        SkillMode::Boomjump => "boomjump",
    }
}

fn strip_unsafe_globals(lua: &Lua) -> Result<()> {
    let globals = lua.globals();
    for name in [
        "io",
        "os",
        "package",
        "require",
        "dofile",
        "loadfile",
        "loadstring",
        "load",
        "debug",
        "getfenv",
        "setfenv",
        "newproxy",
        "collectgarbage",
    ] {
        globals.set(name, mlua::Value::Nil)?;
    }
    Ok(())
}

fn install_host_api(lua: &Lua) -> Result<()> {
    lua.set_named_registry_value(REG_ON, lua.create_table()?)?;
    lua.set_named_registry_value(REG_EVERY, lua.create_table()?)?;

    let vbl = lua.create_table()?;
    vbl.set("state", lua.create_table()?)?;
    vbl.set("settings", lua.create_table()?)?;

    vbl.set(
        "on",
        lua.create_function(|lua, (name, func): (String, Function)| {
            let handlers: Table = lua.named_registry_value(REG_ON)?;
            handlers.set(name, func)
        })?,
    )?;
    vbl.set(
        "every",
        lua.create_function(|lua, (name, func): (String, Function)| {
            let handlers: Table = lua.named_registry_value(REG_EVERY)?;
            handlers.set(name, func)
        })?,
    )?;

    vbl.set(
        "tap",
        lua.create_function(|lua, (key, hold_ms): (String, Option<u64>)| {
            let key = parse_key(&key)?;
            let hold = hold_ms.unwrap_or_else(|| default_tap_ms(lua));
            push(lua, MacroAction::Step(Step::Tap(key, ms(hold))));
            Ok(())
        })?,
    )?;
    vbl.set(
        "down",
        lua.create_function(|lua, key: String| {
            push(lua, MacroAction::Step(Step::Press(parse_key(&key)?)));
            Ok(())
        })?,
    )?;
    vbl.set(
        "up",
        lua.create_function(|lua, key: String| {
            push(lua, MacroAction::Step(Step::Release(parse_key(&key)?)));
            Ok(())
        })?,
    )?;
    vbl.set(
        "click",
        lua.create_function(|lua, (button, hold_ms): (Option<String>, Option<u64>)| {
            let button = parse_button(button.as_deref().unwrap_or("left"))?;
            let hold = hold_ms.unwrap_or_else(|| default_tap_ms(lua));
            push(lua, MacroAction::Step(Step::Click(button, ms(hold))));
            Ok(())
        })?,
    )?;
    vbl.set(
        "wait",
        lua.create_function(|lua, ms_arg: u64| {
            push(lua, MacroAction::Step(Step::Wait(ms(ms_arg))));
            Ok(())
        })?,
    )?;
    vbl.set(
        "release_all",
        lua.create_function(|lua, ()| {
            push(lua, MacroAction::Step(Step::ReleaseAll));
            Ok(())
        })?,
    )?;
    vbl.set(
        "toggle",
        lua.create_function(|lua, name: String| {
            push(lua, MacroAction::Toggle(parse_state(&name)?));
            Ok(())
        })?,
    )?;
    vbl.set(
        "set_state",
        lua.create_function(|lua, (name, value): (String, bool)| {
            push(lua, MacroAction::SetState(parse_state(&name)?, value));
            Ok(())
        })?,
    )?;
    vbl.set(
        "log",
        lua.create_function(|lua, msg: String| {
            push(lua, MacroAction::Log(msg));
            Ok(())
        })?,
    )?;

    lua.globals().set("vbl", vbl)?;
    Ok(())
}

fn push(lua: &Lua, action: MacroAction) {
    if let Some(mut data) = lua.app_data_mut::<HostData>() {
        data.actions.push(action);
    }
}

fn default_tap_ms(lua: &Lua) -> u64 {
    lua.app_data_ref::<HostData>()
        .map(|d| d.default_tap_ms)
        .unwrap_or(35)
}

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

fn parse_key(raw: &str) -> mlua::Result<Key> {
    Key::parse(raw).ok_or_else(|| mlua::Error::RuntimeError(format!("unknown key '{raw}'")))
}

fn parse_button(raw: &str) -> mlua::Result<MouseButton> {
    match raw.to_ascii_lowercase().as_str() {
        "left" => Ok(MouseButton::Left),
        "right" => Ok(MouseButton::Right),
        "middle" => Ok(MouseButton::Middle),
        "x1" => Ok(MouseButton::X1),
        "x2" => Ok(MouseButton::X2),
        other => Err(mlua::Error::RuntimeError(format!(
            "unknown mouse button '{other}'"
        ))),
    }
}

fn parse_state(raw: &str) -> mlua::Result<StateKey> {
    StateKey::from_name(raw)
        .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown state '{raw}'")))
}

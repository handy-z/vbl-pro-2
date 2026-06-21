use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::input::{Key, MouseButton};
use crate::macros::{MacroAction, MacroContext, MacroError, MacroSource, Outcome};
use crate::profile::{SkillMode, VblSettings};
use crate::state::StateKey;

const LOOP_SUFFIX: &str = ".held";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MacroProgram {
    #[serde(default)]
    pub macros: Vec<MacroRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroRule {
    pub on: String,
    #[serde(default)]
    pub when: Option<Condition>,
    #[serde(default, rename = "while")]
    pub while_guard: Option<Condition>,
    #[serde(default, rename = "do")]
    pub actions: Vec<DslAction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Flag(String),

    Ops(Box<CondOps>),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CondOps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<Condition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<Condition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<Condition>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<(String, String)>,

    #[serde(flatten)]
    pub flags: HashMap<String, bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DslAction {
    Tap {
        tap: String,
        #[serde(default)]
        hold_ms: Option<u64>,
    },
    Hold {
        hold: String,
        #[serde(default)]
        until: Option<String>,
    },
    Click {
        click: String,
        #[serde(default)]
        hold_ms: Option<u64>,
    },
    Down {
        down: String,
    },
    Up {
        up: String,
    },
    ReleaseAll {
        release_all: bool,
    },
    Wait {
        wait: u64,
    },
    Toggle {
        toggle: String,
    },
    SetState {
        set_state: String,
        value: bool,
    },
    Log {
        log: String,
    },
    If {
        #[serde(rename = "if")]
        cond: Condition,
        #[serde(default)]
        then: Vec<DslAction>,
        #[serde(default, rename = "else")]
        otherwise: Vec<DslAction>,
    },
}

impl MacroProgram {
    pub fn from_json(src: &str) -> Result<MacroProgram, MacroError> {
        serde_json::from_str(src).map_err(|e| MacroError(format!("invalid macro JSON: {e}")))
    }

    pub fn validate(&self, settings: &VblSettings) -> Result<(), MacroError> {
        for rule in &self.macros {
            check_actions(&rule.actions, settings)?;
        }
        Ok(())
    }

    fn run_rules<'a>(
        &'a self,
        ctx: &MacroContext,
        pick: impl Fn(&'a MacroRule) -> bool,
    ) -> Result<Outcome, MacroError> {
        let mut actions = Vec::new();
        for rule in self.macros.iter().filter(|r| pick(r)) {
            if rule.when.as_ref().is_none_or(|c| c.eval(ctx)) {
                eval_actions(&rule.actions, ctx, &mut actions)?;
            }
        }
        Ok(Outcome { actions })
    }
}

impl MacroSource for MacroProgram {
    fn on(&self, trigger: &str, ctx: &MacroContext) -> Result<Outcome, MacroError> {
        self.run_rules(ctx, |r| !is_loop(&r.on) && r.on == trigger)
    }

    fn every(&self, name: &str, ctx: &MacroContext) -> Result<Outcome, MacroError> {
        let trigger = format!("{name}{LOOP_SUFFIX}");
        self.run_rules(ctx, |r| {
            r.on == trigger && r.while_guard.as_ref().is_none_or(|c| c.eval(ctx))
        })
    }

    fn loop_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .macros
            .iter()
            .filter_map(|r| r.on.strip_suffix(LOOP_SUFFIX).map(str::to_string))
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

fn is_loop(trigger: &str) -> bool {
    trigger.ends_with(LOOP_SUFFIX)
}

impl Condition {
    fn eval(&self, ctx: &MacroContext) -> bool {
        match self {
            Condition::Flag(name) => state_value(name, ctx),
            Condition::Ops(ops) => ops.eval(ctx),
        }
    }
}

impl CondOps {
    fn eval(&self, ctx: &MacroContext) -> bool {
        let mut result = true;
        for (name, want) in &self.flags {
            result &= state_value(name, ctx) == *want;
        }
        if let Some(all) = &self.all {
            result &= all.iter().all(|c| c.eval(ctx));
        }
        if let Some(any) = &self.any {
            result &= any.iter().any(|c| c.eval(ctx));
        }
        if let Some(not) = &self.not {
            result &= !not.eval(ctx);
        }
        if let Some((lhs, rhs)) = &self.eq {
            result &= resolve_value(lhs, ctx) == resolve_value(rhs, ctx);
        }
        result
    }
}

fn state_value(name: &str, ctx: &MacroContext) -> bool {
    StateKey::from_name(name).is_some_and(|k| ctx.state.get(k))
}

fn resolve_value(token: &str, ctx: &MacroContext) -> String {
    match token {
        "$skill" => skill_name(ctx.settings).to_string(),
        "$jumpset_key" => ctx.settings.macro_keys.jumpset_key.clone(),
        "$skill_key" => ctx.settings.macro_keys.skill_key.clone(),
        "$respawn_key" => ctx.settings.macro_keys.respawn_key.clone(),
        "$toggle_ultimate_key" => ctx.settings.macro_keys.toggle_ultimate_key.clone(),
        "$tap_ms" => ctx.settings.tap_ms.to_string(),
        literal => literal.to_string(),
    }
}

fn skill_name(settings: &VblSettings) -> &'static str {
    match settings.skill {
        SkillMode::Normal => "normal",
        SkillMode::Boomjump => "boomjump",
    }
}

fn eval_actions(
    actions: &[DslAction],
    ctx: &MacroContext,
    out: &mut Vec<MacroAction>,
) -> Result<(), MacroError> {
    for action in actions {
        match action {
            DslAction::Tap { tap, hold_ms } => {
                out.push(MacroAction::Step(crate::executor::Step::Tap(
                    resolve_key(tap, ctx)?,
                    hold(*hold_ms, ctx),
                )));
            }
            DslAction::Hold { hold, .. } => {
                out.push(MacroAction::Step(crate::executor::Step::Hold(resolve_key(
                    hold, ctx,
                )?)));
            }
            DslAction::Click { click, hold_ms } => {
                out.push(MacroAction::Step(crate::executor::Step::Click(
                    parse_button(click)?,
                    hold(*hold_ms, ctx),
                )));
            }
            DslAction::Down { down } => {
                out.push(MacroAction::Step(crate::executor::Step::Press(
                    resolve_key(down, ctx)?,
                )));
            }
            DslAction::Up { up } => {
                out.push(MacroAction::Step(crate::executor::Step::Release(
                    resolve_key(up, ctx)?,
                )));
            }
            DslAction::ReleaseAll { release_all } => {
                if *release_all {
                    out.push(MacroAction::Step(crate::executor::Step::ReleaseAll));
                }
            }
            DslAction::Wait { wait } => {
                out.push(MacroAction::Step(crate::executor::Step::Wait(
                    Duration::from_millis(*wait),
                )));
            }
            DslAction::Toggle { toggle } => out.push(MacroAction::Toggle(parse_state(toggle)?)),
            DslAction::SetState { set_state, value } => {
                out.push(MacroAction::SetState(parse_state(set_state)?, *value))
            }
            DslAction::Log { log } => out.push(MacroAction::Log(log.clone())),
            DslAction::If {
                cond,
                then,
                otherwise,
            } => {
                let branch = if cond.eval(ctx) { then } else { otherwise };
                eval_actions(branch, ctx, out)?;
            }
        }
    }
    Ok(())
}

fn check_actions(actions: &[DslAction], settings: &VblSettings) -> Result<(), MacroError> {
    let state = crate::state::EngineState::default();
    let ctx = MacroContext {
        state: &state,
        settings,
    };
    fn walk(actions: &[DslAction], ctx: &MacroContext) -> Result<(), MacroError> {
        for action in actions {
            match action {
                DslAction::Tap { tap, .. } => {
                    resolve_key(tap, ctx)?;
                }
                DslAction::Hold { hold, .. } => {
                    resolve_key(hold, ctx)?;
                }
                DslAction::Down { down } => {
                    resolve_key(down, ctx)?;
                }
                DslAction::Up { up } => {
                    resolve_key(up, ctx)?;
                }
                DslAction::Click { click, .. } => {
                    parse_button(click)?;
                }
                DslAction::Toggle { toggle } => {
                    parse_state(toggle)?;
                }
                DslAction::SetState { set_state, .. } => {
                    parse_state(set_state)?;
                }
                DslAction::If {
                    then, otherwise, ..
                } => {
                    walk(then, ctx)?;
                    walk(otherwise, ctx)?;
                }
                DslAction::ReleaseAll { .. } | DslAction::Wait { .. } | DslAction::Log { .. } => {}
            }
        }
        Ok(())
    }
    walk(actions, &ctx)
}

fn hold(hold_ms: Option<u64>, ctx: &MacroContext) -> Duration {
    Duration::from_millis(hold_ms.unwrap_or(ctx.settings.tap_ms))
}

fn resolve_key(token: &str, ctx: &MacroContext) -> Result<Key, MacroError> {
    let raw = match token {
        "$jumpset_key" => ctx.settings.macro_keys.jumpset_key.as_str(),
        "$skill_key" => ctx.settings.macro_keys.skill_key.as_str(),
        "$respawn_key" => ctx.settings.macro_keys.respawn_key.as_str(),
        "$toggle_ultimate_key" => ctx.settings.macro_keys.toggle_ultimate_key.as_str(),
        literal => literal,
    };
    Key::parse(raw).ok_or_else(|| MacroError(format!("unknown key '{raw}'")))
}

fn parse_button(raw: &str) -> Result<MouseButton, MacroError> {
    match raw.to_ascii_lowercase().as_str() {
        "left" => Ok(MouseButton::Left),
        "right" => Ok(MouseButton::Right),
        "middle" => Ok(MouseButton::Middle),
        "x1" => Ok(MouseButton::X1),
        "x2" => Ok(MouseButton::X2),
        other => Err(MacroError(format!("unknown mouse button '{other}'"))),
    }
}

fn parse_state(raw: &str) -> Result<StateKey, MacroError> {
    StateKey::from_name(raw).ok_or_else(|| MacroError(format!("unknown state '{raw}'")))
}

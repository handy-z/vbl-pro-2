use std::sync::Arc;

use crate::event::EngineEvent;
use crate::executor::{Engine, SeqGen, Step, TaskKind};
use crate::profile::VblSettings;
use crate::state::{EngineState, StateKey};
use crate::time::Clock;
use crate::traits::InputSink;

#[derive(Clone, Debug, PartialEq)]
pub enum MacroAction {
    Step(Step),

    Toggle(StateKey),

    SetState(StateKey, bool),

    Log(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub actions: Vec<MacroAction>,
}

impl Outcome {
    pub fn steps(&self) -> Vec<Step> {
        self.actions
            .iter()
            .filter_map(|a| match a {
                MacroAction::Step(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

pub struct MacroContext<'a> {
    pub state: &'a EngineState,
    pub settings: &'a VblSettings,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct MacroError(pub String);

pub trait MacroSource {
    fn on(&self, trigger: &str, ctx: &MacroContext) -> Result<Outcome, MacroError>;

    fn every(&self, name: &str, ctx: &MacroContext) -> Result<Outcome, MacroError>;

    fn loop_names(&self) -> Vec<String>;
}

fn once(seq: Vec<Step>) -> SeqGen {
    let mut slot = Some(seq);
    Box::new(move |_state, _allowed| slot.take())
}

pub struct ProfileRunner<S: MacroSource> {
    engine: Engine,
    source: S,
    settings: VblSettings,
    logs: Vec<String>,
    errors: Vec<String>,
}

impl<S: MacroSource> ProfileRunner<S> {
    pub fn new(
        clock: Arc<dyn Clock>,
        sink: Arc<dyn InputSink>,
        source: S,
        settings: VblSettings,
    ) -> Self {
        let engine = Engine::new(clock, sink, settings.macro_keys.enabled);
        Self {
            engine,
            source,
            settings,
            logs: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn fire(&mut self, trigger: &str, kind: TaskKind) -> Result<(), MacroError> {
        if !self.engine.macro_allowed() {
            return Ok(());
        }
        let outcome = {
            let ctx = MacroContext {
                state: self.engine.state(),
                settings: &self.settings,
            };
            match self.source.on(trigger, &ctx) {
                Ok(o) => o,
                Err(e) => {
                    self.errors.push(e.0);
                    return Ok(());
                }
            }
        };
        self.apply(outcome, kind);
        Ok(())
    }

    pub fn pump(&mut self) {
        if !self.engine.macro_allowed() {
            return;
        }
        for name in self.source.loop_names() {
            let Some(gate) = StateKey::from_name(&name) else {
                continue;
            };
            if !self.engine.state().get(gate) || self.engine.has_task(TaskKind::X2Loop) {
                continue;
            }
            let outcome = {
                let ctx = MacroContext {
                    state: self.engine.state(),
                    settings: &self.settings,
                };
                match self.source.every(&name, &ctx) {
                    Ok(o) => o,
                    Err(e) => {
                        self.errors.push(e.0);
                        continue;
                    }
                }
            };
            self.apply(outcome, TaskKind::X2Loop);
        }
    }

    pub fn loop_active(&self) -> bool {
        self.source
            .loop_names()
            .iter()
            .filter_map(|n| StateKey::from_name(n))
            .any(|k| self.engine.state().get(k))
    }

    pub fn cancel(&mut self, kind: TaskKind) {
        self.engine.cancel_kind(kind, true);
    }

    fn apply(&mut self, outcome: Outcome, kind: TaskKind) {
        let mut steps = Vec::new();
        for action in outcome.actions {
            match action {
                MacroAction::Step(step) => steps.push(step),
                MacroAction::Toggle(key) => {
                    let next = !self.engine.state().get(key);
                    self.engine.set_state_flag(key, next);
                }
                MacroAction::SetState(key, value) => self.engine.set_state_flag(key, value),
                MacroAction::Log(message) => self.logs.push(message),
            }
        }
        if !steps.is_empty() {
            self.engine.start_task(kind, once(steps));
        }
    }

    pub fn state(&self) -> EngineState {
        *self.engine.state()
    }

    pub fn settings(&self) -> &VblSettings {
        &self.settings
    }

    pub fn apply_settings(&mut self, settings: VblSettings) {
        self.engine.set_macro_enabled(settings.macro_keys.enabled);
        self.settings = settings;
    }

    pub fn now(&self) -> std::time::Duration {
        self.engine.now()
    }

    pub fn next_wake(&self) -> Option<std::time::Duration> {
        self.engine.next_wake()
    }

    pub fn advance(&mut self, to: std::time::Duration) {
        self.engine.advance(to);
    }

    pub fn has_task(&self, kind: TaskKind) -> bool {
        self.engine.has_task(kind)
    }

    pub fn take_events(&mut self) -> Vec<EngineEvent> {
        self.engine.take_events()
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    pub fn set_armed(&mut self, value: bool) {
        self.engine.set_armed(value);
    }

    pub fn set_focused(&mut self, value: bool) {
        self.engine.set_focused(value);
    }

    pub fn set_on_ground(&mut self, value: bool) {
        self.engine.set_on_ground(value);
    }

    pub fn set_ultimate_ready(&mut self, value: bool) {
        self.engine.set_ultimate_ready(value);
    }

    pub fn set_state_flag(&mut self, key: StateKey, value: bool) {
        self.engine.set_state_flag(key, value);
    }
}

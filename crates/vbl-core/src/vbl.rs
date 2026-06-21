use std::sync::Arc;
use std::time::Duration;

use crate::executor::{Engine, SeqGen, Step, TaskKind};
use crate::input::{Key, KeyCombo, MouseButton};
use crate::profile::{SkillMode, VblSettings};
use crate::state::{EngineState, StateKey};
use crate::time::Clock;
use crate::traits::InputSink;

const BOOMJUMP_X1_WAIT_MS: u64 = 100;
const DEFAULT_X1_WAIT_MS: u64 = 25;
const BOOMJUMP_SKILL_GAP_MS: u64 = 25;
const BOOMJUMP_SPIKE_WAIT_MS: u64 = 25;
const LOOP_IDLE_MS: u64 = 1;

fn once(seq: Vec<Step>) -> SeqGen {
    let mut slot = Some(seq);
    Box::new(move |_state, _allowed| slot.take())
}

pub struct Vbl {
    engine: Engine,
    settings: VblSettings,
    jumpset: Key,
    skill: Key,
    shift: Key,
    space: Key,
    esc: Key,
    r: Key,
    enter: Key,
    respawn: Key,
    toggle: Key,
    tap: Duration,
}

impl Vbl {
    pub fn new(clock: Arc<dyn Clock>, sink: Arc<dyn InputSink>, settings: VblSettings) -> Self {
        let macro_enabled = settings.macro_keys.enabled;
        let engine = Engine::new(clock, sink, macro_enabled);

        let jumpset = Key::parse(&settings.macro_keys.jumpset_key)
            .unwrap_or_else(|| Key::parse("e").unwrap());
        let skill = Key::parse(&settings.macro_keys.skill_key)
            .unwrap_or_else(|| Key::parse("lctrl").unwrap());
        let respawn = KeyCombo::parse(&settings.macro_keys.respawn_key)
            .map(|c| c.key)
            .unwrap_or_else(|| Key::parse("f1").unwrap());
        let toggle = KeyCombo::parse(&settings.macro_keys.toggle_ultimate_key)
            .map(|c| c.key)
            .unwrap_or_else(|| Key::parse("f2").unwrap());

        let tap = Duration::from_millis(settings.tap_ms);

        Self {
            engine,
            settings,
            jumpset,
            skill,
            shift: Key::parse("shift").unwrap(),
            space: Key::parse("space").unwrap(),
            esc: Key::parse("escape").unwrap(),
            r: Key::parse("r").unwrap(),
            enter: Key::parse("enter").unwrap(),
            respawn,
            toggle,
            tap,
        }
    }

    pub fn apply_settings(&mut self, settings: VblSettings) {
        self.jumpset = Key::parse(&settings.macro_keys.jumpset_key)
            .unwrap_or_else(|| Key::parse("e").unwrap());
        self.skill = Key::parse(&settings.macro_keys.skill_key)
            .unwrap_or_else(|| Key::parse("lctrl").unwrap());
        self.respawn = KeyCombo::parse(&settings.macro_keys.respawn_key)
            .map(|c| c.key)
            .unwrap_or_else(|| Key::parse("f1").unwrap());
        self.toggle = KeyCombo::parse(&settings.macro_keys.toggle_ultimate_key)
            .map(|c| c.key)
            .unwrap_or_else(|| Key::parse("f2").unwrap());
        self.tap = Duration::from_millis(settings.tap_ms);
        self.engine.set_macro_enabled(settings.macro_keys.enabled);
        self.settings = settings;
    }

    pub fn settings(&self) -> &VblSettings {
        &self.settings
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn state(&self) -> EngineState {
        *self.engine.state()
    }

    pub fn now(&self) -> Duration {
        self.engine.now()
    }

    pub fn next_wake(&self) -> Option<Duration> {
        self.engine.next_wake()
    }

    pub fn advance(&mut self, to: Duration) {
        self.engine.advance(to);
    }

    pub fn take_events(&mut self) -> Vec<crate::event::EngineEvent> {
        self.engine.take_events()
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

    pub fn set_macro_enabled(&mut self, value: bool) {
        self.engine.set_macro_enabled(value);
    }

    fn boomjump(&self) -> bool {
        self.settings.skill == SkillMode::Boomjump
    }

    pub fn press_x1(&mut self) {
        if !self.engine.macro_allowed() {
            return;
        }
        self.engine.set_state_flag(StateKey::X1Held, true);

        let st = *self.engine.state();
        let seq = if !st.x2_held && st.game_on_ground {
            vec![
                Step::Tap(self.space.clone(), self.tap),
                Step::Hold(self.jumpset.clone()),
            ]
        } else {
            let ms = if st.skill_enabled && st.game_ultimate_ready && self.boomjump() {
                BOOMJUMP_X1_WAIT_MS
            } else {
                DEFAULT_X1_WAIT_MS
            };
            vec![
                Step::Wait(Duration::from_millis(ms)),
                Step::Hold(self.jumpset.clone()),
            ]
        };
        self.engine.start_task(TaskKind::X1, once(seq));
    }

    pub fn press_x1_up(&mut self) {
        self.engine.set_state_flag(StateKey::X1Held, false);
        self.engine.cancel_kind(TaskKind::X1, true);
    }

    pub fn press_x2(&mut self) {
        if !self.engine.macro_allowed() {
            return;
        }
        self.engine.set_state_flag(StateKey::X2Held, true);
        if self.engine.has_task(TaskKind::X2Loop) {
            return;
        }

        let shift = self.shift.clone();
        let space = self.space.clone();
        let skill = self.skill.clone();
        let tap = self.tap;
        let boomjump = self.boomjump();
        let idle = Duration::from_millis(LOOP_IDLE_MS);

        let gen: SeqGen = Box::new(move |state: &EngineState, allowed: bool| {
            if !(allowed && state.x2_held) {
                return None;
            }
            if !state.game_on_ground {
                return Some(vec![Step::Wait(idle)]);
            }
            let mut seq = if state.skill_enabled && state.game_ultimate_ready && boomjump {
                vec![
                    Step::Tap(shift.clone(), tap),
                    Step::Tap(skill.clone(), tap),
                    Step::Wait(Duration::from_millis(BOOMJUMP_SKILL_GAP_MS)),
                    Step::Tap(shift.clone(), tap),
                ]
            } else {
                vec![
                    Step::Tap(shift.clone(), tap),
                    Step::Tap(space.clone(), tap),
                    Step::Tap(shift.clone(), tap),
                ]
            };
            seq.push(Step::Wait(idle));
            Some(seq)
        });
        self.engine.start_task(TaskKind::X2Loop, gen);
    }

    pub fn press_x2_up(&mut self) {
        self.engine.set_state_flag(StateKey::X2Held, false);

        if !self.engine.macro_allowed() || self.engine.state().x1_held {
            return;
        }

        let st = *self.engine.state();
        let mut seq: Vec<Step> = Vec::new();
        if st.skill_enabled && st.game_ultimate_ready && !st.game_on_ground {
            match self.settings.skill {
                SkillMode::Normal => seq.push(Step::Tap(self.skill.clone(), self.tap)),
                SkillMode::Boomjump => {
                    seq.push(Step::Wait(Duration::from_millis(BOOMJUMP_SPIKE_WAIT_MS)))
                }
            }
        }
        seq.push(Step::Click(MouseButton::Left, self.tap));
        self.engine.start_task(TaskKind::X2Spike, once(seq));
    }

    pub fn key_down(&mut self, key: &Key) {
        if !self.engine.macro_allowed() {
            return;
        }
        if *key == self.respawn {
            let seq = vec![
                Step::Tap(self.esc.clone(), self.tap),
                Step::Tap(self.r.clone(), self.tap),
                Step::Tap(self.enter.clone(), self.tap),
            ];
            self.engine.start_task(TaskKind::Respawn, once(seq));
        } else if *key == self.toggle {
            let next = !self.engine.state().skill_enabled;
            self.engine.set_state_flag(StateKey::SkillEnabled, next);
        }
    }
}

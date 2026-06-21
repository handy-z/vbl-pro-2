use std::sync::Arc;
use std::time::Duration;

use crate::event::EngineEvent;
use crate::input::{Key, KeyAction, MouseButton};
use crate::state::{EngineState, StateKey};
use crate::time::Clock;
use crate::traits::InputSink;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Tap(Key, Duration),

    Click(MouseButton, Duration),

    Wait(Duration),

    Hold(Key),

    Press(Key),

    Release(Key),

    ReleaseAll,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskKind {
    X1,
    X2Loop,
    X2Spike,
    Respawn,
    Other,
}

pub type SeqGen = Box<dyn FnMut(&EngineState, bool) -> Option<Vec<Step>> + Send>;

enum Pending {
    TapRelease(Key),
    ClickUp(MouseButton),
    WaitDone,
}

struct Task {
    id: u64,
    kind: TaskKind,
    next_seq: SeqGen,
    current: Vec<Step>,
    cursor: usize,
    pending: Option<Pending>,
    wake_at: Option<Duration>,
    held_key: Option<Key>,
    holding: bool,
}

pub struct Engine {
    clock: Arc<dyn Clock>,
    sink: Arc<dyn InputSink>,
    state: EngineState,
    macro_enabled: bool,
    tasks: Vec<Task>,
    next_id: u64,
    events: Vec<EngineEvent>,
}

impl Engine {
    pub fn new(clock: Arc<dyn Clock>, sink: Arc<dyn InputSink>, macro_enabled: bool) -> Self {
        Self {
            clock,
            sink,
            state: EngineState::default(),
            macro_enabled,
            tasks: Vec::new(),
            next_id: 0,
            events: Vec::new(),
        }
    }

    pub fn state(&self) -> &EngineState {
        &self.state
    }

    pub fn now(&self) -> Duration {
        self.clock.now()
    }

    pub fn macro_allowed(&self) -> bool {
        self.state.macro_allowed(self.macro_enabled)
    }

    pub fn macro_enabled(&self) -> bool {
        self.macro_enabled
    }

    pub fn has_task(&self, kind: TaskKind) -> bool {
        self.tasks.iter().any(|t| t.kind == kind)
    }

    pub fn next_wake(&self) -> Option<Duration> {
        self.tasks.iter().filter_map(|t| t.wake_at).min()
    }

    pub fn active_task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn take_events(&mut self) -> Vec<EngineEvent> {
        std::mem::take(&mut self.events)
    }

    fn emit(&mut self, key: StateKey, value: bool) {
        self.events.push(EngineEvent::StateChanged { key, value });
    }

    pub fn set_state_flag(&mut self, key: StateKey, value: bool) {
        if self.state.get(key) != value {
            self.state.set(key, value);
            self.emit(key, value);
        }
    }

    pub fn set_armed(&mut self, value: bool) {
        let was = self.macro_allowed();
        self.state.armed = value;
        self.gate_check(was);
    }

    pub fn set_focused(&mut self, value: bool) {
        let was = self.macro_allowed();
        if self.state.target_focused != value {
            self.state.target_focused = value;
            self.emit(StateKey::RobloxFocused, value);
        }
        self.gate_check(was);
    }

    pub fn set_macro_enabled(&mut self, value: bool) {
        let was = self.macro_allowed();
        self.macro_enabled = value;
        self.gate_check(was);
    }

    pub fn set_on_ground(&mut self, value: bool) {
        self.set_state_flag(StateKey::GameOnGround, value);
    }

    pub fn set_ultimate_ready(&mut self, value: bool) {
        self.set_state_flag(StateKey::GameUltimateReady, value);
    }

    fn gate_check(&mut self, was_allowed: bool) {
        if was_allowed && !self.macro_allowed() {
            self.cancel_all(false);
            self.sink.release_all();
            self.set_state_flag(StateKey::X1Held, false);
            self.set_state_flag(StateKey::X2Held, false);
        }
    }

    pub fn start_task(&mut self, kind: TaskKind, next_seq: SeqGen) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(Task {
            id,
            kind,
            next_seq,
            current: Vec::new(),
            cursor: 0,
            pending: None,
            wake_at: None,
            held_key: None,
            holding: false,
        });
        self.drive(id);
        id
    }

    pub fn cancel_kind(&mut self, kind: TaskKind, emit_releases: bool) {
        let mut i = 0;
        while i < self.tasks.len() {
            if self.tasks[i].kind == kind {
                let task = self.tasks.swap_remove(i);
                self.finish_cancel(task, emit_releases);
            } else {
                i += 1;
            }
        }
    }

    fn cancel_all(&mut self, emit_releases: bool) {
        let tasks = std::mem::take(&mut self.tasks);
        for task in tasks {
            self.finish_cancel(task, emit_releases);
        }
    }

    fn finish_cancel(&self, task: Task, emit_releases: bool) {
        if emit_releases {
            match &task.pending {
                Some(Pending::TapRelease(k)) => self.sink.key(k, KeyAction::Release),
                Some(Pending::ClickUp(b)) => self.sink.mouse_button(*b, KeyAction::Release),
                _ => {}
            }
            if let Some(k) = &task.held_key {
                self.sink.key(k, KeyAction::Release);
            }
        }
    }

    pub fn advance(&mut self, to: Duration) {
        loop {
            let mut best: Option<(Duration, u64)> = None;
            for t in &self.tasks {
                if let Some(w) = t.wake_at {
                    if w <= to {
                        let cand = (w, t.id);
                        if best.is_none_or(|b| cand < b) {
                            best = Some(cand);
                        }
                    }
                }
            }
            let Some((wake, id)) = best else { break };
            self.clock.sleep_until(wake);
            self.resume(id);
        }
        self.clock.sleep_until(to);
    }

    fn drive(&mut self, id: u64) {
        let Some(pos) = self.tasks.iter().position(|t| t.id == id) else {
            return;
        };
        let mut task = self.tasks.swap_remove(pos);
        let sink = self.sink.clone();
        let now = self.clock.now();
        if self.run_steps(&mut task, &sink, now) {
            self.tasks.push(task);
        }
    }

    fn resume(&mut self, id: u64) {
        let Some(pos) = self.tasks.iter().position(|t| t.id == id) else {
            return;
        };
        let mut task = self.tasks.swap_remove(pos);
        let sink = self.sink.clone();

        match task.pending.take() {
            Some(Pending::TapRelease(k)) => sink.key(&k, KeyAction::Release),
            Some(Pending::ClickUp(b)) => sink.mouse_button(b, KeyAction::Release),
            Some(Pending::WaitDone) | None => {}
        }
        task.wake_at = None;
        task.cursor += 1;

        let now = self.clock.now();
        if self.run_steps(&mut task, &sink, now) {
            self.tasks.push(task);
        }
    }

    fn run_steps(&self, task: &mut Task, sink: &Arc<dyn InputSink>, now: Duration) -> bool {
        if task.holding || task.pending.is_some() {
            return true;
        }

        loop {
            if task.cursor >= task.current.len() {
                let allowed = self.macro_allowed();
                match (task.next_seq)(&self.state, allowed) {
                    Some(seq) if !seq.is_empty() => {
                        task.current = seq;
                        task.cursor = 0;
                    }
                    _ => return false,
                }
            }

            match task.current[task.cursor].clone() {
                Step::Tap(key, hold) => {
                    if !self.macro_allowed() {
                        self.abort_cleanup(task, sink);
                        return false;
                    }
                    sink.key(&key, KeyAction::Press);
                    task.pending = Some(Pending::TapRelease(key));
                    task.wake_at = Some(now + hold);
                    return true;
                }
                Step::Click(button, hold) => {
                    if !self.macro_allowed() {
                        self.abort_cleanup(task, sink);
                        return false;
                    }
                    sink.mouse_button(button, KeyAction::Press);
                    task.pending = Some(Pending::ClickUp(button));
                    task.wake_at = Some(now + hold);
                    return true;
                }
                Step::Wait(d) => {
                    task.pending = Some(Pending::WaitDone);
                    task.wake_at = Some(now + d);
                    return true;
                }
                Step::Hold(key) => {
                    if !self.macro_allowed() {
                        self.abort_cleanup(task, sink);
                        return false;
                    }
                    sink.key(&key, KeyAction::Press);
                    task.held_key = Some(key);
                    task.holding = true;
                    task.cursor += 1;
                    return true;
                }

                Step::Press(key) => {
                    if !self.macro_allowed() {
                        self.abort_cleanup(task, sink);
                        return false;
                    }
                    sink.key(&key, KeyAction::Press);
                    task.cursor += 1;
                }
                Step::Release(key) => {
                    sink.key(&key, KeyAction::Release);
                    task.cursor += 1;
                }
                Step::ReleaseAll => {
                    sink.release_all();
                    task.cursor += 1;
                }
            }
        }
    }

    fn abort_cleanup(&self, task: &Task, sink: &Arc<dyn InputSink>) {
        if let Some(k) = &task.held_key {
            sink.key(k, KeyAction::Release);
        }
    }
}

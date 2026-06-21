use std::sync::Arc;
use std::time::Duration;

use vbl_core::dsl::MacroProgram;
use vbl_core::event::EngineEvent;
use vbl_core::executor::TaskKind;
use vbl_core::input::Key;
use vbl_core::macros::{MacroSource, ProfileRunner};
use vbl_core::profile::VblSettings;
use vbl_core::state::EngineState;
use vbl_core::time::Clock;
use vbl_core::traits::InputSink;
use vbl_core::Vbl;
use vbl_scripting::ScriptHost;

trait Programmed {
    fn fire(&mut self, trigger: &str, kind: TaskKind);
    fn pump(&mut self);
    fn cancel(&mut self, kind: TaskKind);
    fn loop_active(&self) -> bool;
    fn set_armed(&mut self, value: bool);
    fn set_focused(&mut self, value: bool);
    fn set_on_ground(&mut self, value: bool);
    fn set_ultimate_ready(&mut self, value: bool);
    fn apply_settings(&mut self, settings: VblSettings);
    fn now(&self) -> Duration;
    fn next_wake(&self) -> Option<Duration>;
    fn advance(&mut self, to: Duration);
    fn take_events(&mut self) -> Vec<EngineEvent>;
    fn take_logs(&mut self) -> Vec<String>;
    fn take_errors(&mut self) -> Vec<String>;
    fn state(&self) -> EngineState;
}

impl<S: MacroSource> Programmed for ProfileRunner<S> {
    fn fire(&mut self, trigger: &str, kind: TaskKind) {
        let _ = ProfileRunner::fire(self, trigger, kind);
    }
    fn pump(&mut self) {
        ProfileRunner::pump(self);
    }
    fn cancel(&mut self, kind: TaskKind) {
        ProfileRunner::cancel(self, kind);
    }
    fn loop_active(&self) -> bool {
        ProfileRunner::loop_active(self)
    }
    fn set_armed(&mut self, value: bool) {
        ProfileRunner::set_armed(self, value);
    }
    fn set_focused(&mut self, value: bool) {
        ProfileRunner::set_focused(self, value);
    }
    fn set_on_ground(&mut self, value: bool) {
        ProfileRunner::set_on_ground(self, value);
    }
    fn set_ultimate_ready(&mut self, value: bool) {
        ProfileRunner::set_ultimate_ready(self, value);
    }
    fn apply_settings(&mut self, settings: VblSettings) {
        ProfileRunner::apply_settings(self, settings);
    }
    fn now(&self) -> Duration {
        ProfileRunner::now(self)
    }
    fn next_wake(&self) -> Option<Duration> {
        ProfileRunner::next_wake(self)
    }
    fn advance(&mut self, to: Duration) {
        ProfileRunner::advance(self, to);
    }
    fn take_events(&mut self) -> Vec<EngineEvent> {
        ProfileRunner::take_events(self)
    }
    fn take_logs(&mut self) -> Vec<String> {
        ProfileRunner::take_logs(self)
    }
    fn take_errors(&mut self) -> Vec<String> {
        ProfileRunner::take_errors(self)
    }
    fn state(&self) -> EngineState {
        ProfileRunner::state(self)
    }
}

enum Backend {
    Builtin(Box<Vbl>),
    Programmed {
        runner: Box<dyn Programmed>,
        label: &'static str,
    },
}

pub struct Driver(Backend);

fn nonempty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.trim().is_empty())
}

impl Driver {
    pub fn new(
        clock: Arc<dyn Clock>,
        sink: Arc<dyn InputSink>,
        settings: VblSettings,
    ) -> (Driver, Option<String>) {
        if let Some(src) = nonempty(&settings.script) {
            match ScriptHost::load(src) {
                Ok(host) => {
                    let runner = ProfileRunner::new(clock, sink, host, settings);
                    return (programmed(runner, "Luau script"), None);
                }
                Err(err) => {
                    let msg = format!("Script failed to load ({err}); using built-in profile");
                    return (builtin(Vbl::new(clock, sink, settings)), Some(msg));
                }
            }
        }
        if let Some(src) = nonempty(&settings.dsl) {
            match MacroProgram::from_json(src).and_then(|p| p.validate(&settings).map(|_| p)) {
                Ok(program) => {
                    let runner = ProfileRunner::new(clock, sink, program, settings);
                    return (programmed(runner, "DSL program"), None);
                }
                Err(err) => {
                    let msg = format!("DSL failed to load ({err}); using built-in profile");
                    return (builtin(Vbl::new(clock, sink, settings)), Some(msg));
                }
            }
        }
        (builtin(Vbl::new(clock, sink, settings)), None)
    }

    pub fn active_label(&self) -> Option<&'static str> {
        match &self.0 {
            Backend::Programmed { label, .. } => Some(label),
            Backend::Builtin(_) => None,
        }
    }

    pub fn press_x1(&mut self) {
        match &mut self.0 {
            Backend::Builtin(v) => v.press_x1(),
            Backend::Programmed { runner, .. } => runner.fire("X1.down", TaskKind::X1),
        }
    }

    pub fn press_x1_up(&mut self) {
        match &mut self.0 {
            Backend::Builtin(v) => v.press_x1_up(),
            Backend::Programmed { runner, .. } => {
                runner.cancel(TaskKind::X1);
                runner.fire("X1.up", TaskKind::X1);
            }
        }
    }

    pub fn press_x2(&mut self) {
        match &mut self.0 {
            Backend::Builtin(v) => v.press_x2(),
            Backend::Programmed { runner, .. } => runner.fire("X2.down", TaskKind::Other),
        }
    }

    pub fn press_x2_up(&mut self) {
        match &mut self.0 {
            Backend::Builtin(v) => v.press_x2_up(),
            Backend::Programmed { runner, .. } => runner.fire("X2.up", TaskKind::X2Spike),
        }
    }

    pub fn key_down(&mut self, key: &Key) {
        match &mut self.0 {
            Backend::Builtin(v) => v.key_down(key),
            Backend::Programmed { runner, .. } => {
                runner.fire(&format!("{}.down", key.as_str()), TaskKind::Other)
            }
        }
    }

    pub fn pump(&mut self) {
        if let Backend::Programmed { runner, .. } = &mut self.0 {
            runner.pump();
        }
    }

    pub fn loop_active(&self) -> bool {
        match &self.0 {
            Backend::Programmed { runner, .. } => runner.loop_active(),
            Backend::Builtin(_) => false,
        }
    }

    pub fn set_armed(&mut self, value: bool) {
        match &mut self.0 {
            Backend::Builtin(v) => v.set_armed(value),
            Backend::Programmed { runner, .. } => runner.set_armed(value),
        }
    }

    pub fn set_focused(&mut self, value: bool) {
        match &mut self.0 {
            Backend::Builtin(v) => v.set_focused(value),
            Backend::Programmed { runner, .. } => runner.set_focused(value),
        }
    }

    pub fn set_on_ground(&mut self, value: bool) {
        match &mut self.0 {
            Backend::Builtin(v) => v.set_on_ground(value),
            Backend::Programmed { runner, .. } => runner.set_on_ground(value),
        }
    }

    pub fn set_ultimate_ready(&mut self, value: bool) {
        match &mut self.0 {
            Backend::Builtin(v) => v.set_ultimate_ready(value),
            Backend::Programmed { runner, .. } => runner.set_ultimate_ready(value),
        }
    }

    pub fn apply_settings(&mut self, settings: VblSettings) {
        match &mut self.0 {
            Backend::Builtin(v) => v.apply_settings(settings),
            Backend::Programmed { runner, .. } => runner.apply_settings(settings),
        }
    }

    pub fn now(&self) -> Duration {
        match &self.0 {
            Backend::Builtin(v) => v.now(),
            Backend::Programmed { runner, .. } => runner.now(),
        }
    }

    pub fn next_wake(&self) -> Option<Duration> {
        match &self.0 {
            Backend::Builtin(v) => v.next_wake(),
            Backend::Programmed { runner, .. } => runner.next_wake(),
        }
    }

    pub fn advance(&mut self, to: Duration) {
        match &mut self.0 {
            Backend::Builtin(v) => v.advance(to),
            Backend::Programmed { runner, .. } => runner.advance(to),
        }
    }

    pub fn take_events(&mut self) -> Vec<EngineEvent> {
        match &mut self.0 {
            Backend::Builtin(v) => v.take_events(),
            Backend::Programmed { runner, .. } => runner.take_events(),
        }
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        match &mut self.0 {
            Backend::Programmed { runner, .. } => runner.take_logs(),
            Backend::Builtin(_) => Vec::new(),
        }
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        match &mut self.0 {
            Backend::Programmed { runner, .. } => runner.take_errors(),
            Backend::Builtin(_) => Vec::new(),
        }
    }

    pub fn state(&self) -> EngineState {
        match &self.0 {
            Backend::Builtin(v) => v.state(),
            Backend::Programmed { runner, .. } => runner.state(),
        }
    }
}

fn builtin(vbl: Vbl) -> Driver {
    Driver(Backend::Builtin(Box::new(vbl)))
}

fn programmed<S: MacroSource + 'static>(runner: ProfileRunner<S>, label: &'static str) -> Driver {
    Driver(Backend::Programmed {
        runner: Box::new(runner),
        label,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vbl_core::input::KeyAction;
    use vbl_core::testing::{MockInputSink, RecordedInput};
    use vbl_core::time::MockClock;

    fn build(settings: VblSettings) -> (Driver, Arc<MockInputSink>, Option<String>) {
        let clock = Arc::new(MockClock::new());
        let sink = Arc::new(MockInputSink::new(clock.clone()));
        let (driver, err) = Driver::new(clock, sink.clone(), settings);
        (driver, sink, err)
    }

    fn armed(settings: VblSettings) -> (Driver, Arc<MockInputSink>) {
        let (mut driver, sink, err) = build(settings);
        assert!(err.is_none(), "expected clean load: {err:?}");
        driver.set_armed(true);
        driver.set_focused(true);
        (driver, sink)
    }

    #[test]
    fn no_program_uses_builtin() {
        let (driver, _, err) = build(VblSettings::default());
        assert_eq!(driver.active_label(), None);
        assert!(err.is_none());
    }

    #[test]
    fn script_takes_precedence_over_dsl() {
        let settings = VblSettings {
            script: Some(r#"vbl.on("X1.down", function() vbl.tap("e") end)"#.to_string()),
            dsl: Some(r#"{"macros":[{"on":"X1.down","do":[{"tap":"r"}]}]}"#.to_string()),
            ..Default::default()
        };
        let (driver, _, _) = build(settings);
        assert_eq!(driver.active_label(), Some("Luau script"));
    }

    #[test]
    fn dsl_used_when_no_script() {
        let settings = VblSettings {
            dsl: Some(r#"{"macros":[{"on":"X1.down","do":[{"tap":"space"}]}]}"#.to_string()),
            ..Default::default()
        };
        let (mut driver, sink) = armed(settings);
        assert_eq!(driver.active_label(), Some("DSL program"));
        driver.press_x1();
        driver.advance(Duration::from_millis(50));
        assert_eq!(
            sink.actions(),
            vec![
                RecordedInput::Key {
                    key: "space".into(),
                    action: KeyAction::Press
                },
                RecordedInput::Key {
                    key: "space".into(),
                    action: KeyAction::Release
                },
            ]
        );
    }

    #[test]
    fn invalid_dsl_falls_back_to_builtin_with_error() {
        let settings = VblSettings {
            dsl: Some(r#"{"macros":[{"on":"x","do":[{"tap":"not_a_key"}]}]}"#.to_string()),
            ..Default::default()
        };
        let (driver, _, err) = build(settings);
        assert_eq!(driver.active_label(), None);
        assert!(err.is_some());
    }

    #[test]
    fn invalid_script_falls_back_to_builtin_with_error() {
        let settings = VblSettings {
            script: Some("this is not valid lua %%%".to_string()),
            ..Default::default()
        };
        let (driver, _, err) = build(settings);
        assert_eq!(driver.active_label(), None);
        assert!(err.is_some());
    }
}

use std::time::Duration;

use crate::input::{Key, KeyAction, MouseButton};
use crate::time::Clock;
use crate::traits::InputSink;

pub fn tap(sink: &dyn InputSink, clock: &dyn Clock, key: &Key, hold: Duration) {
    sink.key(key, KeyAction::Press);
    clock.sleep_until(clock.now() + hold);
    sink.key(key, KeyAction::Release);
}

pub fn click(sink: &dyn InputSink, clock: &dyn Clock, button: MouseButton, hold: Duration) {
    sink.mouse_button(button, KeyAction::Press);
    clock.sleep_until(clock.now() + hold);
    sink.mouse_button(button, KeyAction::Release);
}

pub fn wait(clock: &dyn Clock, dur: Duration) {
    clock.sleep_until(clock.now() + dur);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::testing::{MockInputSink, RecordedInput};
    use crate::time::MockClock;

    #[test]
    fn tap_presses_then_releases_after_hold() {
        let clock = Arc::new(MockClock::new());
        let sink = MockInputSink::new(clock.clone());
        let key = Key::parse("e").unwrap();

        tap(&sink, &*clock, &key, Duration::from_millis(35));

        let log = sink.log();
        assert_eq!(log.len(), 2);

        assert_eq!(log[0].0, Duration::ZERO);
        assert_eq!(
            log[0].1,
            RecordedInput::Key {
                key: "e".into(),
                action: KeyAction::Press
            }
        );
        assert_eq!(log[1].0, Duration::from_millis(35));
        assert_eq!(
            log[1].1,
            RecordedInput::Key {
                key: "e".into(),
                action: KeyAction::Release
            }
        );
    }

    #[test]
    fn click_timing() {
        let clock = Arc::new(MockClock::new());
        let sink = MockInputSink::new(clock.clone());

        click(&sink, &*clock, MouseButton::Left, Duration::from_millis(35));

        let log = sink.log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[1].0, Duration::from_millis(35));
        assert_eq!(
            log[1].1,
            RecordedInput::Mouse {
                button: MouseButton::Left,
                action: KeyAction::Release
            }
        );
    }
}

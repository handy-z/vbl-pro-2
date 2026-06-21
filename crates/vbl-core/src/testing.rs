use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::input::{Key, KeyAction, MouseButton};
use crate::time::Clock;
use crate::traits::{InputSink, TargetWindow, WindowTracker};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordedInput {
    Key {
        key: String,
        action: KeyAction,
    },
    Mouse {
        button: MouseButton,
        action: KeyAction,
    },
    ReleaseAll,
}

pub struct MockInputSink {
    clock: Arc<dyn Clock>,
    log: Mutex<Vec<(Duration, RecordedInput)>>,
}

impl MockInputSink {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn log(&self) -> Vec<(Duration, RecordedInput)> {
        self.log.lock().unwrap().clone()
    }

    pub fn actions(&self) -> Vec<RecordedInput> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .map(|(_, a)| a.clone())
            .collect()
    }

    fn push(&self, item: RecordedInput) {
        let t = self.clock.now();
        self.log.lock().unwrap().push((t, item));
    }
}

impl InputSink for MockInputSink {
    fn key(&self, key: &Key, action: KeyAction) {
        self.push(RecordedInput::Key {
            key: key.as_str().to_string(),
            action,
        });
    }

    fn mouse_button(&self, button: MouseButton, action: KeyAction) {
        self.push(RecordedInput::Mouse { button, action });
    }

    fn release_all(&self) {
        self.push(RecordedInput::ReleaseAll);
    }
}

pub struct MockWindowTracker {
    focused: AtomicBool,
}

impl MockWindowTracker {
    pub fn new(focused: bool) -> Self {
        Self {
            focused: AtomicBool::new(focused),
        }
    }

    pub fn set_focused(&self, value: bool) {
        self.focused.store(value, Ordering::SeqCst);
    }
}

impl WindowTracker for MockWindowTracker {
    fn target_window(&self) -> Option<TargetWindow> {
        None
    }

    fn is_target_focused(&self) -> bool {
        self.focused.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::MockClock;

    #[test]
    fn mock_sink_records_in_order_with_time() {
        let clock = Arc::new(MockClock::new());
        let sink = MockInputSink::new(clock.clone());

        let key = Key::parse("space").unwrap();
        sink.key(&key, KeyAction::Press);
        clock.advance(Duration::from_millis(10));
        sink.release_all();

        let log = sink.log();
        assert_eq!(log[0].0, Duration::ZERO);
        assert_eq!(log[1].0, Duration::from_millis(10));
        assert_eq!(log[1].1, RecordedInput::ReleaseAll);
    }

    #[test]
    fn mock_window_tracker_toggles() {
        let w = MockWindowTracker::new(false);
        assert!(!w.is_target_focused());
        w.set_focused(true);
        assert!(w.is_target_focused());
    }
}

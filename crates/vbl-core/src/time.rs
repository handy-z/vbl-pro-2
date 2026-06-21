use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;

    fn sleep_until(&self, deadline: Duration);
}

pub struct SystemClock {
    origin: std::time::Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: std::time::Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }

    fn sleep_until(&self, deadline: Duration) {
        let now = self.origin.elapsed();
        if deadline > now {
            std::thread::sleep(deadline - now);
        }
    }
}

#[derive(Default)]
pub struct MockClock {
    micros: AtomicU64,
}

impl MockClock {
    pub fn new() -> Self {
        Self {
            micros: AtomicU64::new(0),
        }
    }

    pub fn advance(&self, by: Duration) {
        self.micros
            .fetch_add(by.as_micros() as u64, Ordering::SeqCst);
    }

    pub fn set(&self, at: Duration) {
        self.micros.store(at.as_micros() as u64, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    fn now(&self) -> Duration {
        Duration::from_micros(self.micros.load(Ordering::SeqCst))
    }

    fn sleep_until(&self, deadline: Duration) {
        let target = deadline.as_micros() as u64;

        let mut cur = self.micros.load(Ordering::SeqCst);
        while target > cur {
            match self
                .micros
                .compare_exchange(cur, target, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => break,
                Err(actual) => cur = actual,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_clock_advances_on_sleep_until() {
        let c = MockClock::new();
        assert_eq!(c.now(), Duration::ZERO);
        c.sleep_until(Duration::from_millis(35));
        assert_eq!(c.now(), Duration::from_millis(35));

        c.sleep_until(Duration::from_millis(10));
        assert_eq!(c.now(), Duration::from_millis(35));
    }

    #[test]
    fn mock_clock_advance_accumulates() {
        let c = MockClock::new();
        c.advance(Duration::from_millis(25));
        c.advance(Duration::from_millis(100));
        assert_eq!(c.now(), Duration::from_millis(125));
    }
}

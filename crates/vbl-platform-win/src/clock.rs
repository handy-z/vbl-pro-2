use std::time::Duration;

use vbl_core::time::Clock;
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

const SPIN_THRESHOLD: Duration = Duration::from_millis(2);

pub struct WinClock {
    freq: i64,
    origin: i64,
}

impl WinClock {
    pub fn new() -> Self {
        unsafe {
            let _ = timeBeginPeriod(1);
        }
        let mut freq: i64 = 0;
        let mut origin: i64 = 0;
        unsafe {
            let _ = QueryPerformanceFrequency(&mut freq);
            let _ = QueryPerformanceCounter(&mut origin);
        }
        Self {
            freq: if freq == 0 { 1 } else { freq },
            origin,
        }
    }

    fn counter(&self) -> i64 {
        let mut c: i64 = 0;
        unsafe {
            let _ = QueryPerformanceCounter(&mut c);
        }
        c
    }
}

impl Default for WinClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WinClock {
    fn drop(&mut self) {
        unsafe {
            let _ = timeEndPeriod(1);
        }
    }
}

impl Clock for WinClock {
    fn now(&self) -> Duration {
        let ticks = (self.counter() - self.origin).max(0) as u128;
        let nanos = ticks * 1_000_000_000u128 / self.freq as u128;
        Duration::from_nanos(nanos as u64)
    }

    fn sleep_until(&self, deadline: Duration) {
        loop {
            let now = self.now();
            if now >= deadline {
                return;
            }
            let remaining = deadline - now;
            if remaining > SPIN_THRESHOLD {
                std::thread::sleep(remaining - Duration::from_millis(1));
            } else {
                std::hint::spin_loop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_monotonic() {
        let clock = WinClock::new();
        let a = clock.now();
        let b = clock.now();
        assert!(b >= a);
    }

    #[test]
    fn sleep_until_waits_at_least_the_duration() {
        let clock = WinClock::new();
        let start = clock.now();
        clock.sleep_until(start + Duration::from_millis(10));
        assert!(clock.now() - start >= Duration::from_millis(10));
    }
}

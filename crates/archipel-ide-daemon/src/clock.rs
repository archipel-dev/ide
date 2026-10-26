//! Injected time source: the scheduler never reads the system clock directly,
//! so debounce and ordering are testable with a [`ManualClock`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Monotonic time since an arbitrary origin.
pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}

/// The real clock (monotonic, origin = creation).
#[derive(Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
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
}

/// A clock advanced by hand (tests, benches).
#[derive(Debug, Default)]
pub struct ManualClock {
    micros: AtomicU64,
}

impl ManualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&self, by: Duration) {
        self.micros
            .fetch_add(by.as_micros() as u64, Ordering::SeqCst);
    }

    pub fn set(&self, to: Duration) {
        self.micros.store(to.as_micros() as u64, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_micros(self.micros.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_only_moves_when_told() {
        let clock = ManualClock::new();
        assert_eq!(clock.now(), Duration::ZERO);
        clock.advance(Duration::from_millis(300));
        assert_eq!(clock.now(), Duration::from_millis(300));
        clock.set(Duration::from_secs(1));
        assert_eq!(clock.now(), Duration::from_secs(1));
        assert!(SystemClock::new().now() < Duration::from_secs(5));
    }
}

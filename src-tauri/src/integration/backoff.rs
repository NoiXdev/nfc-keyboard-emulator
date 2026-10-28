use std::time::Duration;

const BASE: Duration = Duration::from_secs(1);
const CAP: Duration = Duration::from_secs(60);
/// 2^6 s = 64 s already exceeds the cap, so larger exponents are never needed.
const MAX_EXPONENT: u32 = 6;

/// Reconnect backoff: 1 s doubling up to 60 s, with +/-20 % jitter.
#[derive(Debug, Default)]
pub struct Backoff {
    attempt: u32,
}

impl Backoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Returns the delay before the next attempt. `jitter` must be in `[0, 1)`.
    pub fn next_delay(&mut self, jitter: f64) -> Duration {
        let base = (BASE * 2u32.pow(self.attempt.min(MAX_EXPONENT))).min(CAP);
        self.attempt = self.attempt.saturating_add(1);
        base.mul_f64(0.8 + 0.4 * jitter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_caps() {
        let mut b = Backoff::new();
        let secs: Vec<f64> = (0..10).map(|_| b.next_delay(0.5).as_secs_f64()).collect();
        assert_eq!(
            secs,
            vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 60.0, 60.0, 60.0, 60.0]
        );
    }

    #[test]
    fn jitter_stays_within_twenty_percent() {
        let mut b = Backoff::new();
        assert_eq!(b.next_delay(0.0), Duration::from_millis(800));
        let mut b = Backoff::new();
        assert!(b.next_delay(0.999) < Duration::from_millis(1200));
        let mut b = Backoff::new();
        for _ in 0..20 {
            assert!(b.next_delay(0.999) < Duration::from_secs(72));
        }
    }

    #[test]
    fn reset_returns_to_one_second() {
        let mut b = Backoff::new();
        for _ in 0..5 {
            b.next_delay(0.5);
        }
        b.reset();
        assert_eq!(b.next_delay(0.5), Duration::from_secs(1));
    }
}

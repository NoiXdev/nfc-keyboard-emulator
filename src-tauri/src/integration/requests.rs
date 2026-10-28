use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const MAX_TIMEOUT_MS: u64 = 300_000;

struct Pending {
    request_id: String,
    deadline: Instant,
}

/// Holds at most one armed scan request. The next scan answers it instead of
/// being typed. Shared via `Arc` between the worker thread and async tasks.
#[derive(Default)]
pub struct RequestSlot {
    inner: Mutex<Option<Pending>>,
}

impl RequestSlot {
    /// Arms `request_id`, replacing any armed request, and returns the id it
    /// superseded. The timeout is clamped to `1..=MAX_TIMEOUT_MS`; `None` means
    /// `DEFAULT_TIMEOUT_MS`.
    pub fn arm(&self, request_id: String, timeout_ms: Option<u64>, now: Instant) -> Option<String> {
        let timeout = timeout_ms
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(1, MAX_TIMEOUT_MS);
        let pending = Pending {
            request_id,
            deadline: now + Duration::from_millis(timeout),
        };
        self.inner
            .lock()
            .unwrap()
            .replace(pending)
            .map(|p| p.request_id)
    }

    /// Disarms the slot only if `request_id` is the armed one.
    pub fn cancel(&self, request_id: &str) -> bool {
        let mut slot = self.inner.lock().unwrap();
        if slot.as_ref().is_some_and(|p| p.request_id == request_id) {
            *slot = None;
            true
        } else {
            false
        }
    }

    /// Returns and clears the armed id for an incoming scan. An expired entry is
    /// left in place for `expire`, so the timeout can still be reported.
    pub fn take_for_scan(&self, now: Instant) -> Option<String> {
        let mut slot = self.inner.lock().unwrap();
        if slot.as_ref().is_some_and(|p| now < p.deadline) {
            slot.take().map(|p| p.request_id)
        } else {
            None
        }
    }

    /// Clears and returns the armed id once its deadline has passed.
    pub fn expire(&self, now: Instant) -> Option<String> {
        let mut slot = self.inner.lock().unwrap();
        if slot.as_ref().is_some_and(|p| now >= p.deadline) {
            slot.take().map(|p| p.request_id)
        } else {
            None
        }
    }

    pub fn clear(&self) -> Option<String> {
        self.inner.lock().unwrap().take().map(|p| p.request_id)
    }

    pub fn is_armed(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn arm_then_take_returns_id_once() {
        let slot = RequestSlot::default();
        let t0 = Instant::now();
        assert_eq!(slot.arm("a".into(), None, t0), None);
        assert!(slot.is_armed());
        assert_eq!(slot.take_for_scan(t0 + ms(10)), Some("a".to_string()));
        assert!(!slot.is_armed());
        assert_eq!(slot.take_for_scan(t0 + ms(20)), None);
    }

    #[test]
    fn arm_replaces_and_reports_superseded() {
        let slot = RequestSlot::default();
        let t0 = Instant::now();
        slot.arm("a".into(), None, t0);
        assert_eq!(slot.arm("b".into(), None, t0), Some("a".to_string()));
        assert_eq!(slot.take_for_scan(t0), Some("b".to_string()));
    }

    #[test]
    fn cancel_only_matching_id() {
        let slot = RequestSlot::default();
        let t0 = Instant::now();
        slot.arm("a".into(), None, t0);
        assert!(!slot.cancel("other"));
        assert!(slot.is_armed());
        assert!(slot.cancel("a"));
        assert!(!slot.is_armed());
        assert!(!slot.cancel("a"));
    }

    #[test]
    fn expired_request_is_not_taken_but_reported_by_expire() {
        let slot = RequestSlot::default();
        let t0 = Instant::now();
        slot.arm("a".into(), Some(1000), t0);
        assert_eq!(slot.take_for_scan(t0 + ms(1001)), None);
        assert_eq!(slot.expire(t0 + ms(1001)), Some("a".to_string()));
        assert_eq!(slot.expire(t0 + ms(1001)), None);
    }

    #[test]
    fn expire_leaves_a_live_request_alone() {
        let slot = RequestSlot::default();
        let t0 = Instant::now();
        slot.arm("a".into(), Some(1000), t0);
        assert_eq!(slot.expire(t0 + ms(999)), None);
        assert!(slot.is_armed());
    }

    #[test]
    fn clear_returns_the_armed_id() {
        let slot = RequestSlot::default();
        assert_eq!(slot.clear(), None);
        slot.arm("a".into(), None, Instant::now());
        assert_eq!(slot.clear(), Some("a".to_string()));
        assert!(!slot.is_armed());
    }

    /// Whether a request armed with `timeout` is expired `after` ms later.
    fn expired(timeout: Option<u64>, after: u64) -> bool {
        let t0 = Instant::now();
        let slot = RequestSlot::default();
        slot.arm("a".into(), timeout, t0);
        slot.expire(t0 + ms(after)).is_some()
    }

    #[test]
    fn timeout_is_clamped() {
        assert!(!expired(Some(0), 0));
        assert!(expired(Some(0), 1));
        assert!(!expired(Some(1_000_000_000), MAX_TIMEOUT_MS - 1));
        assert!(expired(Some(1_000_000_000), MAX_TIMEOUT_MS));
        assert!(!expired(None, DEFAULT_TIMEOUT_MS - 1));
        assert!(expired(None, DEFAULT_TIMEOUT_MS));
    }

    #[test]
    fn slot_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RequestSlot>();
    }
}

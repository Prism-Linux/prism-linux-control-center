use std::collections::HashMap;
use std::time::{Duration, Instant};

const FIRST_RETRY: Duration = Duration::from_secs(30);
const MAX_RETRY: Duration = Duration::from_secs(600);
pub(super) const MAX_ATTEMPTS: u32 = 8;

/// The bind thread reports whether it transmitted and why it failed.
pub(super) type RebindJob = ([u8; 6], std::thread::JoinHandle<(bool, Option<String>)>);

struct RebindBackoff {
    attempts: u32,
    next_at: Instant,
    deferred: Option<String>,
}

/// Paces automatic rebinds per device. A device that keeps corrupting its
/// address is retried with growing gaps and then left for the user.
#[derive(Default)]
pub(super) struct RebindSchedule {
    devices: HashMap<[u8; 6], RebindBackoff>,
}

impl RebindSchedule {
    pub(super) fn due(&self, mac: &[u8; 6], now: Instant) -> bool {
        self.devices
            .get(mac)
            .is_none_or(|entry| entry.attempts < MAX_ATTEMPTS && now >= entry.next_at)
    }

    /// Returns the attempt number just recorded.
    pub(super) fn record_attempt(&mut self, mac: [u8; 6], now: Instant) -> u32 {
        let entry = self.devices.entry(mac).or_insert(RebindBackoff {
            attempts: 0,
            next_at: now,
            deferred: None,
        });
        entry.deferred = None;
        let wait = FIRST_RETRY
            .saturating_mul(1 << entry.attempts.min(5))
            .min(MAX_RETRY);
        entry.attempts += 1;
        entry.next_at = now + wait;
        entry.attempts
    }

    pub(super) fn attempts(&self, mac: &[u8; 6]) -> u32 {
        self.devices.get(mac).map_or(0, |entry| entry.attempts)
    }

    /// A bind that never reached the radio waits without spending an attempt.
    /// Returns whether the reason differs from the previous deferral.
    pub(super) fn defer(&mut self, mac: [u8; 6], now: Instant, reason: &str) -> bool {
        let entry = self.devices.entry(mac).or_insert(RebindBackoff {
            attempts: 0,
            next_at: now,
            deferred: None,
        });
        entry.next_at = entry.next_at.max(now + FIRST_RETRY);
        if entry.deferred.as_deref() == Some(reason) {
            return false;
        }
        entry.deferred = Some(reason.to_owned());
        true
    }

    pub(super) fn deferred(&self, mac: &[u8; 6]) -> bool {
        self.devices
            .get(mac)
            .is_some_and(|entry| entry.deferred.is_some())
    }

    pub(super) fn retain(&mut self, mut pending: impl FnMut(&[u8; 6]) -> bool) {
        self.devices.retain(|mac, _| pending(mac));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebinds_back_off_exponentially_to_a_cap_then_stop() {
        let mac = [1; 6];
        let mut schedule = RebindSchedule::default();
        let mut now = Instant::now();
        assert!(schedule.due(&mac, now));
        for (attempt, wait) in [30, 60, 120, 240, 480, 600, 600].into_iter().enumerate() {
            assert_eq!(schedule.record_attempt(mac, now), attempt as u32 + 1);
            assert!(!schedule.due(&mac, now + Duration::from_secs(wait - 1)));
            now += Duration::from_secs(wait);
            assert!(schedule.due(&mac, now));
        }
        assert_eq!(schedule.record_attempt(mac, now), MAX_ATTEMPTS);
        assert!(!schedule.due(&mac, now + Duration::from_secs(3600)));
        assert!(schedule.due(&[2; 6], now));
    }

    #[test]
    fn untransmitted_binds_wait_without_spending_attempts() {
        let mac = [1; 6];
        let now = Instant::now();
        let mut schedule = RebindSchedule::default();
        assert!(schedule.defer(mac, now, "slot limit"));
        for _ in 0..20 {
            assert!(!schedule.defer(mac, now, "slot limit"));
        }
        assert!(schedule.defer(mac, now, "TX unavailable"));
        assert_eq!(schedule.attempts(&mac), 0);
        assert!(schedule.deferred(&mac));
        assert!(!schedule.due(&mac, now + FIRST_RETRY - Duration::from_secs(1)));
        assert!(schedule.due(&mac, now + FIRST_RETRY));
    }

    #[test]
    fn recovered_device_starts_a_fresh_schedule() {
        let mac = [1; 6];
        let now = Instant::now();
        let mut schedule = RebindSchedule::default();
        for _ in 0..MAX_ATTEMPTS {
            schedule.record_attempt(mac, now);
        }
        assert!(!schedule.due(&mac, now));
        schedule.retain(|_| false);
        assert!(schedule.due(&mac, now));
        assert_eq!(schedule.record_attempt(mac, now), 1);
    }
}

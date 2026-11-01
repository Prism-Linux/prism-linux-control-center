use super::ServiceManager;
use lianli_devices::detect::known_devices_missing_from_libusb;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// A device appears in sysfs shortly before libusb processes its udev event,
/// so only devices missing for longer than this are treated as stale.
const MISSING_GRACE: Duration = Duration::from_secs(2);
const MIN_RESCAN_INTERVAL: Duration = Duration::from_secs(10);
const MAX_RESCAN_INTERVAL: Duration = Duration::from_secs(300);

#[derive(Default)]
pub(super) struct UsbRescanPolicy {
    first_missing: HashMap<String, Instant>,
    last_rescan: Option<Instant>,
    unresolved: HashSet<String>,
    retry_interval: Duration,
}

impl UsbRescanPolicy {
    fn should_rescan(&mut self, missing: &[String], now: Instant) -> bool {
        self.first_missing
            .retain(|device, _| missing.contains(device));
        for device in missing {
            self.first_missing.entry(device.clone()).or_insert(now);
        }
        let mut stale = self
            .first_missing
            .iter()
            .filter(|(_, since)| now.duration_since(**since) >= MISSING_GRACE)
            .map(|(device, _)| device)
            .peekable();
        if stale.peek().is_none() {
            return false;
        }
        let interval = if stale.all(|device| self.unresolved.contains(device)) {
            self.retry_interval.max(MIN_RESCAN_INTERVAL)
        } else {
            MIN_RESCAN_INTERVAL
        };
        self.last_rescan
            .is_none_or(|at| now.duration_since(at) >= interval)
    }

    /// Returns true when the set of devices that a rescan could not recover
    /// changed, so callers log once per outcome rather than once per attempt.
    fn record_rescan(&mut self, still_missing: &[String], now: Instant) -> bool {
        self.last_rescan = Some(now);
        let still_missing: HashSet<String> = still_missing.iter().cloned().collect();
        let repeated = !still_missing.is_empty() && still_missing.is_subset(&self.unresolved);
        self.retry_interval = if repeated {
            (self.retry_interval.max(MIN_RESCAN_INTERVAL) * 2).min(MAX_RESCAN_INTERVAL)
        } else {
            MIN_RESCAN_INTERVAL
        };
        let changed = still_missing != self.unresolved;
        self.unresolved = still_missing;
        changed
    }
}

impl ServiceManager {
    /// Replaces the libusb context when the kernel lists supported devices
    /// that libusb never learned about, which happens when udev events do
    /// not reach the daemon inside some containers.
    pub(super) fn rescan_stale_usb_devices(&mut self) {
        let missing = known_devices_missing_from_libusb();
        let now = Instant::now();
        if !self.usb_rescan.should_rescan(&missing, now) {
            return;
        }
        if let Err(error) = lianli_transport::usb_context::rescan() {
            warn!("Could not rescan USB devices: {error}");
            self.usb_rescan.record_rescan(&missing, now);
            return;
        }
        let still_missing = known_devices_missing_from_libusb();
        let recovered = missing
            .iter()
            .filter(|device| !still_missing.contains(device))
            .count();
        if recovered > 0 {
            info!(
                "Rescanned USB devices because hotplug events did not reach the daemon; \
                 recovered {recovered}"
            );
        }
        let outcome_changed = self.usb_rescan.record_rescan(&still_missing, now);
        if still_missing.is_empty() {
            return;
        }
        if outcome_changed {
            warn!(
                "USB devices listed by the kernel are still missing from libusb after a rescan: {}",
                still_missing.join(", ")
            );
        } else {
            debug!(
                "USB devices still missing from libusb after a rescan: {}",
                still_missing.join(", ")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing(devices: &[&str]) -> Vec<String> {
        devices.iter().map(|device| device.to_string()).collect()
    }

    #[test]
    fn nothing_missing_never_rescans() {
        let mut policy = UsbRescanPolicy::default();
        let start = Instant::now();
        assert!(!policy.should_rescan(&[], start));
        assert!(!policy.should_rescan(&[], start + Duration::from_secs(60)));
    }

    #[test]
    fn device_must_stay_missing_past_the_grace_period() {
        let mut policy = UsbRescanPolicy::default();
        let start = Instant::now();
        let ad21 = missing(&["1a86:ad21 at 1-4"]);
        assert!(!policy.should_rescan(&ad21, start));
        assert!(!policy.should_rescan(&ad21, start + Duration::from_secs(1)));
        assert!(policy.should_rescan(&ad21, start + MISSING_GRACE));
    }

    #[test]
    fn device_that_reappears_resets_its_grace_period() {
        let mut policy = UsbRescanPolicy::default();
        let start = Instant::now();
        let ad21 = missing(&["1a86:ad21 at 1-4"]);
        assert!(!policy.should_rescan(&ad21, start));
        assert!(!policy.should_rescan(&[], start + Duration::from_secs(1)));
        assert!(!policy.should_rescan(&ad21, start + Duration::from_secs(2)));
        assert!(policy.should_rescan(&ad21, start + Duration::from_secs(4)));
    }

    #[test]
    fn rescans_are_rate_limited() {
        let mut policy = UsbRescanPolicy::default();
        let start = Instant::now();
        let a088 = missing(&["1cbe:a088 at 1-4"]);
        policy.should_rescan(&a088, start);
        let first = start + MISSING_GRACE;
        assert!(policy.should_rescan(&a088, first));
        policy.record_rescan(&[], first);

        let ad21 = missing(&["1a86:ad21 at 1-4"]);
        policy.should_rescan(&ad21, first + Duration::from_secs(1));
        assert!(!policy.should_rescan(&ad21, first + Duration::from_secs(5)));
        assert!(policy.should_rescan(&ad21, first + MIN_RESCAN_INTERVAL));
    }

    #[test]
    fn unrecovered_devices_back_off_until_the_cap() {
        let mut policy = UsbRescanPolicy::default();
        let ad21 = missing(&["1a86:ad21 at 1-4"]);
        let mut now = Instant::now();
        policy.should_rescan(&ad21, now);
        now += MISSING_GRACE;
        assert!(policy.should_rescan(&ad21, now));
        assert!(policy.record_rescan(&ad21, now));

        let mut expected = MIN_RESCAN_INTERVAL;
        for _ in 0..8 {
            assert!(!policy.should_rescan(&ad21, now + expected - Duration::from_secs(1)));
            now += expected;
            assert!(policy.should_rescan(&ad21, now));
            assert!(!policy.record_rescan(&ad21, now));
            expected = (expected * 2).min(MAX_RESCAN_INTERVAL);
        }
        assert_eq!(expected, MAX_RESCAN_INTERVAL);
    }

    #[test]
    fn newly_missing_device_skips_the_backoff() {
        let mut policy = UsbRescanPolicy::default();
        let ad21 = missing(&["1a86:ad21 at 1-4"]);
        let mut now = Instant::now();
        policy.should_rescan(&ad21, now);
        for _ in 0..4 {
            now += MAX_RESCAN_INTERVAL;
            assert!(policy.should_rescan(&ad21, now));
            policy.record_rescan(&ad21, now);
        }

        let both = missing(&["1a86:ad21 at 1-4", "1cbe:a088 at 1-5"]);
        policy.should_rescan(&both, now + Duration::from_secs(1));
        assert!(policy.should_rescan(&both, now + MIN_RESCAN_INTERVAL));
    }

    #[test]
    fn recovery_resets_the_backoff() {
        let mut policy = UsbRescanPolicy::default();
        let ad21 = missing(&["1a86:ad21 at 1-4"]);
        let mut now = Instant::now();
        policy.should_rescan(&ad21, now);
        for _ in 0..4 {
            now += MAX_RESCAN_INTERVAL;
            assert!(policy.should_rescan(&ad21, now));
            policy.record_rescan(&ad21, now);
        }
        assert!(policy.record_rescan(&[], now));

        now += Duration::from_secs(1);
        policy.should_rescan(&ad21, now);
        assert!(policy.should_rescan(&ad21, now + MIN_RESCAN_INTERVAL));
    }
}

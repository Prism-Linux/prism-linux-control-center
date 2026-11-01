use super::controller::WirelessController;
use super::transport::open_any;
use super::{RX_IDS, TX_IDS, USB_CMD_RESET_COMPANION};
use anyhow::{Context, Result};
use lianli_transport::usb::{RusbBulk, USB_TIMEOUT};
use lianli_transport::TransportError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

const RESET_BACKOFF: [Duration; 3] = [
    Duration::from_secs(10),
    Duration::from_secs(30),
    Duration::from_secs(120),
];
const SLOW_RETRY: Duration = Duration::from_secs(300);
const HEALTHY_LINK: Duration = Duration::from_secs(300);
const REENUMERATION_WAIT: Duration = Duration::from_secs(5);
const REENUMERATION_POLL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dongle {
    Tx,
    Rx,
}

impl Dongle {
    fn ids(self) -> &'static [(u16, u16)] {
        match self {
            Self::Tx => &TX_IDS,
            Self::Rx => &RX_IDS,
        }
    }

    fn companion(self) -> Self {
        match self {
            Self::Tx => Self::Rx,
            Self::Rx => Self::Tx,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Tx => "TX",
            Self::Rx => "RX",
        }
    }
}

impl std::fmt::Display for Dongle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug)]
pub(super) struct DongleFailure {
    dongle: Dongle,
    stage: &'static str,
}

impl DongleFailure {
    pub(super) fn new(dongle: Dongle, stage: &'static str) -> Self {
        Self { dongle, stage }
    }
}

impl std::fmt::Display for DongleFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.stage)
    }
}

/// Bus and address of each dongle; a changed address means it re-enumerated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DonglePresence {
    pub tx: Option<(u8, u8)>,
    pub rx: Option<(u8, u8)>,
}

impl DonglePresence {
    pub fn scan() -> Self {
        let mut presence = Self::default();
        let Ok(devices) = lianli_transport::usb_context::devices() else {
            return presence;
        };
        for device in devices.iter() {
            let Ok(desc) = device.device_descriptor() else {
                continue;
            };
            let id = (desc.vendor_id(), desc.product_id());
            let location = Some((device.bus_number(), device.address()));
            if TX_IDS.contains(&id) {
                presence.tx = presence.tx.or(location);
            } else if RX_IDS.contains(&id) {
                presence.rx = presence.rx.or(location);
            }
        }
        presence
    }

    pub fn any(&self) -> bool {
        self.tx.is_some() || self.rx.is_some()
    }

    fn location(&self, dongle: Dongle) -> Option<(u8, u8)> {
        match dongle {
            Dongle::Tx => self.tx,
            Dongle::Rx => self.rx,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryPlan {
    pub allow_reset: bool,
}

pub struct RecoveryAttempt {
    pub reset: Option<Dongle>,
    pub result: Result<()>,
}

/// Paces reconnects: three fast companion resets, then one every five minutes.
/// Hotplug and short-lived links only advance the next attempt; the reset
/// budget refills only after the link has stayed up, since a reset
/// re-enumerates the dongle and a flapping link must not refill its own budget.
#[derive(Debug, Default)]
pub struct RecoveryLadder {
    resets: usize,
    last_reset: Option<Instant>,
    due: Option<Instant>,
    connected_at: Option<Instant>,
}

impl RecoveryLadder {
    pub fn next_attempt(&self, now: Instant) -> Option<RecoveryPlan> {
        if self.settled(now) {
            return Some(RecoveryPlan { allow_reset: true });
        }
        if self.due.is_some_and(|due| now < due) {
            return None;
        }
        let spacing = match self.resets {
            0 => Duration::ZERO,
            1 => RESET_BACKOFF[0],
            2 => RESET_BACKOFF[1],
            _ => SLOW_RETRY,
        };
        let allow_reset = self
            .last_reset
            .is_none_or(|last| now.duration_since(last) >= spacing);
        Some(RecoveryPlan { allow_reset })
    }

    pub fn record(&mut self, attempt: &RecoveryAttempt, now: Instant) {
        if self.settled(now) {
            *self = Self::default();
        }
        if attempt.reset.is_some() {
            self.resets += 1;
            self.last_reset = Some(now);
        }
        if attempt.result.is_ok() {
            self.connected_at = Some(now);
            self.due = None;
            return;
        }
        self.connected_at = None;
        let wait = match self.resets.checked_sub(1) {
            None => RESET_BACKOFF[0],
            Some(index) => RESET_BACKOFF.get(index).copied().unwrap_or(SLOW_RETRY),
        };
        let mut due = now + wait;
        if attempt.reset.is_none() && self.resets >= RESET_BACKOFF.len() {
            if let Some(last) = self.last_reset {
                due = due.max(last + SLOW_RETRY);
            }
        }
        self.due = Some(due);
    }

    pub fn hotplug(&mut self, now: Instant) {
        self.due = Some(now);
    }

    fn settled(&self, now: Instant) -> bool {
        self.connected_at
            .is_some_and(|since| now.duration_since(since) >= HEALTHY_LINK)
    }

    pub fn resets(&self) -> usize {
        self.resets
    }
}

fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Acquire) || lianli_transport::usb::shutting_down()
}

/// A busy or inaccessible interface belongs to another owner, so resetting
/// its dongle would disrupt that owner instead of recovering ours.
fn reset_forbidden(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<TransportError>(),
            Some(TransportError::Usb(rusb::Error::Busy | rusb::Error::Access))
        )
    })
}

fn reset_target(presence: DonglePresence, error: &anyhow::Error) -> Option<Dongle> {
    if reset_forbidden(error) {
        return None;
    }
    let target = match (presence.tx, presence.rx) {
        (None, Some(_)) => Dongle::Tx,
        (Some(_), None) => Dongle::Rx,
        (Some(_), Some(_)) => error.downcast_ref::<DongleFailure>()?.dongle,
        (None, None) => return None,
    };
    presence.location(target.companion())?;
    Some(target)
}

/// The vendor reset opcode restarts the opposite dongle: sent to the RX it
/// resets the TX, sent to the TX it resets the RX.
pub(super) fn write_companion_reset(via: &RusbBulk) -> Result<()> {
    let mut frame = [0u8; 64];
    frame[0] = USB_CMD_RESET_COMPANION;
    let written = via
        .write(&frame, USB_TIMEOUT)
        .context("writing companion reset")?;
    anyhow::ensure!(written == frame.len(), "short companion reset write");
    Ok(())
}

fn reset_via_companion(target: Dongle, cancel: &AtomicBool) -> Result<()> {
    let via = target.companion();
    let mut handle = open_any(via.ids())
        .with_context(|| format!("opening {via} dongle to reset the {target}"))?;
    handle
        .detach_and_configure_with_cancel(via.name(), || cancelled(cancel))
        .with_context(|| format!("claiming {via} dongle to reset the {target}"))?;
    write_companion_reset(&handle)
}

fn wait_for_reenumeration(target: Dongle, before: Option<(u8, u8)>, cancel: &AtomicBool) -> bool {
    let deadline = Instant::now() + REENUMERATION_WAIT;
    loop {
        let location = DonglePresence::scan().location(target);
        if location.is_some() && location != before {
            return true;
        }
        if cancelled(cancel) || Instant::now() >= deadline {
            return false;
        }
        thread::sleep(REENUMERATION_POLL);
    }
}

impl WirelessController {
    /// Runs one recovery step off the service thread. A failed connect may
    /// reset the failing dongle through its companion and connect once more.
    pub fn recover(&mut self, plan: RecoveryPlan, cancel: &AtomicBool) -> RecoveryAttempt {
        let first = self.connect_and_poll(cancel);
        let Err(error) = first else {
            return RecoveryAttempt {
                reset: None,
                result: Ok(()),
            };
        };
        if !plan.allow_reset || cancelled(cancel) {
            return RecoveryAttempt {
                reset: None,
                result: Err(error),
            };
        }
        let presence = DonglePresence::scan();
        let Some(target) = reset_target(presence, &error) else {
            return RecoveryAttempt {
                reset: None,
                result: Err(error),
            };
        };
        let via = target.companion();
        warn!("Wireless {target} unresponsive ({error:#}); resetting it through the {via}");
        if let Err(reset_error) = reset_via_companion(target, cancel) {
            return RecoveryAttempt {
                reset: Some(target),
                result: Err(error.context(format!("{target} reset failed: {reset_error:#}"))),
            };
        }
        if !wait_for_reenumeration(target, presence.location(target), cancel) {
            debug!("Wireless {target} did not re-enumerate after reset");
        }
        RecoveryAttempt {
            reset: Some(target),
            result: self.connect_and_poll(cancel),
        }
    }

    fn connect_and_poll(&mut self, cancel: &AtomicBool) -> Result<()> {
        self.connect_with_cancel(cancel)?;
        anyhow::ensure!(!cancelled(cancel), "wireless recovery cancelled");
        self.start_polling_with_cancel(cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(reset: Option<Dongle>) -> RecoveryAttempt {
        RecoveryAttempt {
            reset,
            result: Err(anyhow::anyhow!("connect failed")),
        }
    }

    fn both() -> DonglePresence {
        DonglePresence {
            tx: Some((1, 2)),
            rx: Some((1, 3)),
        }
    }

    #[test]
    fn three_fast_resets_back_off_then_reset_every_five_minutes() {
        let start = Instant::now();
        let mut ladder = RecoveryLadder::default();
        let mut now = start;
        for wait in [10, 30, 120] {
            assert_eq!(
                ladder.next_attempt(now),
                Some(RecoveryPlan { allow_reset: true })
            );
            ladder.record(&failed(Some(Dongle::Tx)), now);
            assert_eq!(
                ladder.next_attempt(now + Duration::from_secs(wait - 1)),
                None
            );
            now += Duration::from_secs(wait);
        }
        let last_reset = now - Duration::from_secs(120);
        assert_eq!(
            ladder.next_attempt(now),
            Some(RecoveryPlan { allow_reset: false })
        );
        ladder.record(&failed(None), now);
        assert_eq!(
            ladder.next_attempt(last_reset + SLOW_RETRY - Duration::from_secs(1)),
            None
        );
        now = last_reset + SLOW_RETRY;
        assert_eq!(
            ladder.next_attempt(now),
            Some(RecoveryPlan { allow_reset: true })
        );
        ladder.record(&failed(Some(Dongle::Tx)), now);
        assert_eq!(
            ladder.next_attempt(now + SLOW_RETRY - Duration::from_secs(1)),
            None
        );
        assert_eq!(
            ladder.next_attempt(now + SLOW_RETRY),
            Some(RecoveryPlan { allow_reset: true })
        );
    }

    #[test]
    fn hotplug_retries_now_without_refilling_the_reset_budget() {
        let mut now = Instant::now();
        let mut ladder = RecoveryLadder::default();
        for _ in 0..3 {
            ladder.record(&failed(Some(Dongle::Tx)), now);
            now += Duration::from_secs(1);
        }
        ladder.hotplug(now);
        assert_eq!(
            ladder.next_attempt(now),
            Some(RecoveryPlan { allow_reset: false })
        );
        assert_eq!(ladder.resets(), 3);
    }

    fn connected() -> RecoveryAttempt {
        RecoveryAttempt {
            reset: None,
            result: Ok(()),
        }
    }

    #[test]
    fn link_that_stays_up_restores_the_full_ladder() {
        let now = Instant::now();
        let mut ladder = RecoveryLadder::default();
        for _ in 0..4 {
            ladder.record(&failed(Some(Dongle::Rx)), now);
        }
        ladder.record(&connected(), now);
        let lost = now + HEALTHY_LINK;
        assert_eq!(
            ladder.next_attempt(lost),
            Some(RecoveryPlan { allow_reset: true })
        );
        ladder.record(&failed(Some(Dongle::Rx)), lost);
        assert_eq!(ladder.resets(), 1);
    }

    #[test]
    fn flapping_link_keeps_the_reset_budget_and_spacing() {
        let mut now = Instant::now();
        let mut ladder = RecoveryLadder::default();
        let mut resets = Vec::new();
        for _ in 0..80 {
            let plan = ladder
                .next_attempt(now)
                .expect("a lost link retries at once");
            if plan.allow_reset {
                resets.push(now);
            }
            ladder.record(
                &RecoveryAttempt {
                    reset: plan.allow_reset.then_some(Dongle::Tx),
                    result: Ok(()),
                },
                now,
            );
            now += Duration::from_secs(5);
        }
        let gaps: Vec<u64> = resets
            .windows(2)
            .map(|pair| pair[1].duration_since(pair[0]).as_secs())
            .collect();
        assert_eq!(gaps, [10, 30, 300]);
    }

    #[test]
    fn failures_without_a_reset_retry_at_the_first_backoff() {
        let now = Instant::now();
        let mut ladder = RecoveryLadder::default();
        ladder.record(&failed(None), now);
        assert_eq!(ladder.next_attempt(now + Duration::from_secs(9)), None);
        assert!(ladder.next_attempt(now + Duration::from_secs(10)).is_some());
        assert_eq!(ladder.resets(), 0);
    }

    #[test]
    fn missing_dongle_is_reset_through_the_present_companion() {
        let error = anyhow::anyhow!("not found");
        let tx_missing = DonglePresence {
            tx: None,
            rx: Some((1, 3)),
        };
        let rx_missing = DonglePresence {
            tx: Some((1, 2)),
            rx: None,
        };
        assert_eq!(reset_target(tx_missing, &error), Some(Dongle::Tx));
        assert_eq!(reset_target(rx_missing, &error), Some(Dongle::Rx));
        assert_eq!(reset_target(DonglePresence::default(), &error), None);
    }

    #[test]
    fn present_dongles_reset_the_stage_that_failed() {
        let tx: anyhow::Error = Err::<(), _>(anyhow::anyhow!("no reply"))
            .context(DongleFailure::new(Dongle::Tx, "discovering master"))
            .unwrap_err();
        let rx: anyhow::Error = Err::<(), _>(anyhow::anyhow!("pipe"))
            .context(DongleFailure::new(Dongle::Rx, "claiming RX"))
            .unwrap_err()
            .context("outer");
        assert_eq!(reset_target(both(), &tx), Some(Dongle::Tx));
        assert_eq!(reset_target(both(), &rx), Some(Dongle::Rx));
        assert_eq!(reset_target(both(), &anyhow::anyhow!("unclassified")), None);
    }

    #[test]
    fn interface_owned_elsewhere_is_never_reset() {
        for usb in [rusb::Error::Busy, rusb::Error::Access] {
            let error = Err::<(), _>(TransportError::Usb(usb))
                .context(DongleFailure::new(Dongle::Tx, "claiming TX"))
                .unwrap_err();
            assert_eq!(reset_target(both(), &error), None);
        }
    }
}

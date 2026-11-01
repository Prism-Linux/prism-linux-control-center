use super::RF_SET_RGB;
use anyhow::{Context, Result};
use lianli_transport::usb::RusbBulk;
use parking_lot::Mutex;
use std::cell::Cell;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

pub(super) type SharedTransport = Arc<Mutex<TransportState<RusbBulk>>>;

const GAP_AFTER_RGB: Duration = Duration::from_millis(10);
const GAP_BETWEEN_FRAMES: Duration = Duration::from_millis(5);

#[derive(Clone, Copy)]
struct SentFrame {
    target: [u8; 6],
    opcode: u8,
    sent_at: Instant,
}

/// A device that hears a frame too soon after another device's frame can
/// latch the earlier frame's trailing bytes as its address and PWM, so frames
/// to a new target or of a new kind wait out the vendor's air gap.
#[derive(Default)]
pub(super) struct RfPacer(Cell<Option<SentFrame>>);

impl RfPacer {
    fn send<R>(&self, rf_data: &[u8], send: impl FnOnce() -> Result<R>) -> Result<R> {
        let (target, opcode) = frame_identity(rf_data);
        let gap = gap_before(self.0.get(), target, opcode, Instant::now());
        if !gap.is_zero() {
            std::thread::sleep(gap);
        }
        let result = send();
        self.0.set(Some(SentFrame {
            target,
            opcode,
            sent_at: Instant::now(),
        }));
        result
    }
}

fn frame_identity(rf_data: &[u8]) -> ([u8; 6], u8) {
    let target = rf_data
        .get(2..8)
        .and_then(|target| target.try_into().ok())
        .unwrap_or_default();
    (target, rf_data.get(1).copied().unwrap_or_default())
}

fn gap_before(last: Option<SentFrame>, target: [u8; 6], opcode: u8, now: Instant) -> Duration {
    let Some(last) = last else {
        return Duration::ZERO;
    };
    if last.target == target && last.opcode == opcode {
        return Duration::ZERO;
    }
    let gap = if last.opcode == RF_SET_RGB {
        GAP_AFTER_RGB
    } else {
        GAP_BETWEEN_FRAMES
    };
    gap.saturating_sub(now.saturating_duration_since(last.sent_at))
}

pub(super) struct DongleLink<'a, T = RusbBulk> {
    handle: &'a T,
    pacer: &'a RfPacer,
}

impl<T> Clone for DongleLink<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for DongleLink<'_, T> {}

impl<T> Deref for DongleLink<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.handle
    }
}

impl<T> DongleLink<'_, T> {
    /// `rf_data` is the frame being sent, starting with its select byte. Call
    /// once per frame, around all of its USB chunks.
    pub(super) fn paced_frame<R>(
        &self,
        rf_data: &[u8],
        send: impl FnOnce() -> Result<R>,
    ) -> Result<R> {
        self.pacer.send(rf_data, send)
    }
}

#[derive(Debug)]
pub(super) struct RecoveryBackoff;

impl std::fmt::Display for RecoveryBackoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("wireless transport recovery is backing off")
    }
}
impl std::error::Error for RecoveryBackoff {}

pub(super) struct TransportState<T> {
    handle: Option<T>,
    retry_after: Option<Instant>,
    pacer: RfPacer,
}

impl<T> TransportState<T> {
    pub(super) fn new(handle: T) -> Self {
        Self {
            handle: Some(handle),
            retry_after: None,
            pacer: RfPacer::default(),
        }
    }

    pub(super) fn get(&self) -> Result<&T> {
        self.handle
            .as_ref()
            .ok_or_else(|| lianli_transport::TransportError::Usb(rusb::Error::NoDevice).into())
    }

    fn link(&self) -> Result<DongleLink<'_, T>> {
        Ok(DongleLink {
            handle: self.get()?,
            pacer: &self.pacer,
        })
    }

    fn replace(
        &mut self,
        open: impl FnOnce() -> Result<T>,
        claim: impl FnOnce(&mut T) -> Result<()>,
    ) -> Result<()> {
        if self.retry_after.is_some_and(|until| Instant::now() < until) {
            return Err(RecoveryBackoff.into());
        }
        drop(self.handle.take());
        let result = (|| {
            let mut replacement = open()?;
            claim(&mut replacement)?;
            self.handle = Some(replacement);
            Ok(())
        })();
        self.retry_after = Some(Instant::now() + Duration::from_secs(1));
        result
    }
}

/// Controller clones share this state, so releasing here frees the claimed
/// interface even while other clones still hold the `Arc`.
pub(super) fn release_transport<T>(transport: &Mutex<TransportState<T>>) {
    let handle = transport.lock().handle.take();
    drop(handle);
}

/// Try to open a USB device matching any of the given VID:PID pairs.
pub(super) fn open_any(ids: &[(u16, u16)]) -> Result<RusbBulk> {
    let mut failure = None;
    for &(vid, pid) in ids {
        match RusbBulk::open(vid, pid) {
            Ok(transport) => return Ok(transport),
            Err(lianli_transport::TransportError::DeviceNotFound { .. }) => {}
            Err(e) => {
                failure.get_or_insert(e);
            }
        }
    }
    if let Some(e) = failure {
        return Err(e.into());
    }
    let labels: Vec<String> = ids
        .iter()
        .map(|(vid, pid)| format!("{vid:04x}:{pid:04x}"))
        .collect();
    anyhow::bail!("no device matching {}", labels.join(" or "))
}

/// Run a USB op on a dongle transport with one-shot reopen + retry on failure.
/// `op` must be safe to call twice (idempotent at the protocol level).
pub(super) fn with_transport_recovery<F, R>(
    arc: &SharedTransport,
    ids: &[(u16, u16)],
    name: &str,
    stop: &AtomicBool,
    mut op: F,
) -> Result<R>
where
    F: FnMut(DongleLink<'_>) -> Result<R>,
{
    let mut handle = arc.lock();
    retry_transport_operation(
        &mut *handle,
        stop,
        |handle| {
            anyhow::ensure!(
                !stop.load(Ordering::Acquire),
                "wireless controller is stopping"
            );
            op(handle.link()?)
        },
        |handle, error| reopen_transport(handle, ids, name, stop, error),
    )
}

fn reopen_transport(
    handle: &mut TransportState<RusbBulk>,
    ids: &[(u16, u16)],
    name: &str,
    stop: &AtomicBool,
    error: &anyhow::Error,
) -> Result<()> {
    handle.replace(
        || {
            warn!("{name} transport op failed ({error}); attempting reopen");
            open_any(ids).context(format!("reopening {name} dongle"))
        },
        |replacement| {
            replacement
                .detach_and_configure_with_cancel(name, || stop.load(Ordering::Acquire))
                .context("claiming replacement wireless transport")?;
            if let Err(error) = replacement.clear_halts() {
                debug!("{name}: clearing endpoint halts after reopen failed: {error}");
            }
            anyhow::ensure!(
                !stop.load(Ordering::Acquire)
                    && !lianli_transport::usb::SHUTTING_DOWN.load(Ordering::Relaxed),
                "wireless controller is stopping"
            );
            Ok(())
        },
    )?;
    info!("{name} transport reopened");
    Ok(())
}

/// Recover an absent handle before the operation; never replay a partially sent transaction.
pub(super) fn with_ready_transport<R>(
    arc: &SharedTransport,
    ids: &[(u16, u16)],
    name: &str,
    stop: &AtomicBool,
    op: impl FnOnce(DongleLink<'_>) -> Result<R>,
) -> Result<R> {
    let mut handle = arc.lock();
    transport_operation_once(&mut handle, stop, op, |handle, error| {
        reopen_transport(handle, ids, name, stop, error)
    })
}

fn transport_operation_once<T, R>(
    state: &mut TransportState<T>,
    stop: &AtomicBool,
    op: impl FnOnce(DongleLink<'_, T>) -> Result<R>,
    reopen: impl FnOnce(&mut TransportState<T>, &anyhow::Error) -> Result<()>,
) -> Result<R> {
    retry_transport_operation(state, stop, |state| state.get().map(|_| ()), reopen)?;
    anyhow::ensure!(
        !stop.load(Ordering::Acquire),
        "wireless controller is stopping"
    );
    let result = op(state.link()?);
    if result.as_ref().is_err_and(needs_reopen) {
        drop(state.handle.take());
    }
    result
}

fn retry_transport_operation<T, R>(
    transport: &mut T,
    stop: &AtomicBool,
    mut operation: impl FnMut(&mut T) -> Result<R>,
    reopen: impl FnOnce(&mut T, &anyhow::Error) -> Result<()>,
) -> Result<R> {
    anyhow::ensure!(
        !stop.load(Ordering::Acquire),
        "wireless controller is stopping"
    );
    match operation(transport) {
        Ok(result) => Ok(result),
        Err(error) => {
            if lianli_transport::usb::SHUTTING_DOWN.load(Ordering::Relaxed)
                || stop.load(Ordering::Acquire)
            {
                return Err(error).context("shutting down");
            }
            if !needs_reopen(&error) {
                return Err(error);
            }
            reopen(transport, &error)?;
            anyhow::ensure!(
                !stop.load(Ordering::Acquire),
                "wireless controller is stopping"
            );
            operation(transport)
        }
    }
}

fn needs_reopen(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<lianli_transport::TransportError>(),
        Some(lianli_transport::TransportError::Usb(
            rusb::Error::NoDevice | rusb::Error::Io | rusb::Error::Pipe
        ))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wireless::RF_PWM_CMD;
    use std::cell::Cell;

    #[test]
    fn frames_to_another_target_or_opcode_wait_out_the_remaining_air_gap() {
        let sent_at = Instant::now();
        let last = |opcode| {
            Some(SentFrame {
                target: [1; 6],
                opcode,
                sent_at,
            })
        };
        let after = |ms| sent_at + Duration::from_millis(ms);
        let ms = Duration::from_millis;
        for (previous, target, opcode, now, expected) in [
            (None, [1; 6], RF_SET_RGB, after(0), ms(0)),
            (last(RF_SET_RGB), [1; 6], RF_SET_RGB, after(0), ms(0)),
            (last(RF_PWM_CMD), [1; 6], RF_PWM_CMD, after(0), ms(0)),
            (last(RF_SET_RGB), [2; 6], RF_SET_RGB, after(0), ms(10)),
            (last(RF_SET_RGB), [1; 6], RF_PWM_CMD, after(0), ms(10)),
            (last(RF_PWM_CMD), [2; 6], RF_PWM_CMD, after(0), ms(5)),
            (last(RF_PWM_CMD), [1; 6], RF_SET_RGB, after(0), ms(5)),
            (last(RF_SET_RGB), [2; 6], RF_PWM_CMD, after(4), ms(6)),
            (last(RF_PWM_CMD), [2; 6], RF_PWM_CMD, after(3), ms(2)),
            (last(RF_SET_RGB), [2; 6], RF_PWM_CMD, after(25), ms(0)),
        ] {
            assert_eq!(gap_before(previous, target, opcode, now), expected);
        }
    }

    #[test]
    fn pacing_survives_transport_release_and_reopen() {
        let transport = Mutex::new(TransportState::new(1u8));
        let mut rgb_frame = [0u8; 240];
        rgb_frame[..8].copy_from_slice(&[0x12, RF_SET_RGB, 1, 1, 1, 1, 1, 1]);
        transport
            .lock()
            .link()
            .unwrap()
            .paced_frame(&rgb_frame, || Ok(()))
            .unwrap();

        release_transport(&transport);
        transport.lock().replace(|| Ok(2), |_| Ok(())).unwrap();

        let last = transport.lock().pacer.0.get();
        let sent_at = last.expect("sent frame must be remembered").sent_at;
        assert_eq!(gap_before(last, [2; 6], RF_PWM_CMD, sent_at), GAP_AFTER_RGB);
        assert_eq!(
            gap_before(last, [1; 6], RF_SET_RGB, sent_at),
            Duration::ZERO
        );
    }

    #[test]
    fn non_replayable_operation_recovers_missing_handle_before_sending() {
        let mut state = TransportState::<u8> {
            handle: None,
            retry_after: None,
            pacer: RfPacer::default(),
        };
        let sends = Cell::new(0);
        let result = transport_operation_once(
            &mut state,
            &AtomicBool::new(false),
            |handle| {
                sends.set(sends.get() + 1);
                Ok(*handle)
            },
            |state, _| state.replace(|| Ok(7), |_| Ok(())),
        )
        .unwrap();
        assert_eq!(result, 7);
        assert_eq!(sends.get(), 1);
    }

    #[test]
    fn partial_transaction_is_not_replayed_and_disconnect_invalidates_handle() {
        let mut state = TransportState::new(1u8);
        let packets = Cell::new(0);
        let result: Result<()> = transport_operation_once(
            &mut state,
            &AtomicBool::new(false),
            |_| {
                packets.set(packets.get() + 1);
                Err(lianli_transport::TransportError::Usb(rusb::Error::NoDevice).into())
            },
            |_, _| panic!("must not reopen during a transaction"),
        );
        assert!(result.is_err());
        assert_eq!(packets.get(), 1);
        assert!(state.get().is_err());
    }

    #[test]
    fn transaction_never_starts_when_recovery_fails_or_is_cancelled() {
        for cancel in [false, true] {
            let mut state = TransportState::<u8> {
                handle: None,
                retry_after: None,
                pacer: RfPacer::default(),
            };
            let stop = AtomicBool::new(false);
            let result: Result<()> = transport_operation_once(
                &mut state,
                &stop,
                |_| panic!("transaction must not start"),
                |state, _| {
                    if cancel {
                        stop.store(true, Ordering::Release);
                        state.replace(|| Ok(1), |_| Ok(()))
                    } else {
                        state.replace(|| anyhow::bail!("open failed"), |_| Ok(()))
                    }
                },
            );
            assert!(result.is_err());
        }
    }

    #[test]
    fn replacement_releases_old_owner_before_claim_even_when_claim_fails() {
        struct Handle<'a>(&'a Cell<bool>);
        impl Drop for Handle<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let old_dropped = Cell::new(false);
        let new_dropped = Cell::new(false);
        let mut handle = TransportState::new(Handle(&old_dropped));
        let result = handle.replace(
            || Ok(Handle(&new_dropped)),
            |_| {
                assert!(old_dropped.get());
                assert!(!new_dropped.get());
                anyhow::bail!("interface unavailable")
            },
        );
        assert!(result.is_err());
        assert!(new_dropped.get());
        assert!(handle.get().is_err());
        assert!(handle.handle.is_none());
    }

    #[test]
    fn release_frees_the_interface_for_every_clone_of_the_transport() {
        struct Handle<'a>(&'a Cell<bool>);
        impl Drop for Handle<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Cell::new(false);
        let owner = std::rc::Rc::new(Mutex::new(TransportState::new(Handle(&dropped))));
        let clone = std::rc::Rc::clone(&owner);
        release_transport(&owner);
        assert!(dropped.get());
        drop(owner);
        let result: Result<()> = transport_operation_once(
            &mut clone.lock(),
            &AtomicBool::new(true),
            |_| panic!("released transport must not send"),
            |_, _| panic!("stopped clone must not reclaim the interface"),
        );
        assert!(result.is_err());
        assert!(clone.lock().get().is_err());
    }

    #[test]
    fn failed_claim_recovers_on_a_later_request_after_backoff() {
        let mut state = TransportState::new(1u8);
        assert!(state
            .replace(|| Ok(2), |_| anyhow::bail!("claim failed"))
            .is_err());
        assert!(state.get().is_err());
        assert!(state
            .replace(|| panic!("must back off"), |_| Ok(()))
            .unwrap_err()
            .is::<RecoveryBackoff>());
        state.retry_after = Some(Instant::now());
        let sends = Cell::new(0);
        let result = retry_transport_operation(
            &mut state,
            &AtomicBool::new(false),
            |state| {
                let handle = state.get()?;
                sends.set(sends.get() + 1);
                Ok(*handle)
            },
            |state, _| {
                state.replace(
                    || Ok(3),
                    |handle| {
                        *handle = 4;
                        Ok(())
                    },
                )
            },
        )
        .unwrap();
        assert_eq!(result, 4);
        assert_eq!(sends.get(), 1);
        assert_eq!(*state.get().unwrap(), 4);
    }

    #[test]
    fn failed_open_leaves_no_transport_and_backs_off() {
        let mut state = TransportState::new(1u8);
        assert!(state
            .replace(
                || anyhow::bail!("open failed"),
                |_| panic!("must not claim")
            )
            .is_err());
        assert!(state.handle.is_none());
        assert!(state
            .replace(|| panic!("must back off"), |_| Ok(()))
            .unwrap_err()
            .is::<RecoveryBackoff>());
    }

    #[test]
    fn stopped_controller_does_not_recover_missing_transport() {
        let mut state = TransportState::<u8> {
            handle: None,
            retry_after: None,
            pacer: RfPacer::default(),
        };
        let result: Result<()> = retry_transport_operation(
            &mut state,
            &AtomicBool::new(true),
            |_| panic!("must not send"),
            |_, _| panic!("must not reopen"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn protocol_errors_and_timeouts_do_not_reopen_transport() {
        for error in [
            anyhow::anyhow!("missing or invalid GetDev response"),
            lianli_transport::TransportError::Usb(rusb::Error::Timeout).into(),
        ] {
            let mut error = Some(error);
            let result: Result<()> = retry_transport_operation(
                &mut (),
                &AtomicBool::new(false),
                |_| Err(error.take().unwrap()),
                |_, _| panic!("response failure must not reopen a claimed interface"),
            );
            assert!(result.is_err());
        }
    }

    #[test]
    fn failed_reopen_never_retries_operation() {
        let attempts = Cell::new(0);
        let result: Result<()> = retry_transport_operation(
            &mut (),
            &AtomicBool::new(false),
            |_| {
                attempts.set(attempts.get() + 1);
                Err(lianli_transport::TransportError::Usb(rusb::Error::NoDevice).into())
            },
            |_, _| anyhow::bail!("replacement unavailable"),
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("replacement unavailable"));
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn stopped_controller_never_starts_an_operation() {
        let stop = AtomicBool::new(true);
        let result: Result<()> = retry_transport_operation(
            &mut (),
            &stop,
            |_| panic!("operation must not start"),
            |_, _| panic!("transport must not reopen"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn stop_during_failed_operation_prevents_reopen() {
        let stop = AtomicBool::new(false);
        let result: Result<()> = retry_transport_operation(
            &mut (),
            &stop,
            |_| {
                stop.store(true, Ordering::Release);
                anyhow::bail!("receiver read failed")
            },
            |_, _| panic!("transport must not reopen after stop"),
        );
        assert!(result.unwrap_err().to_string().contains("shutting down"));
    }

    #[test]
    fn stop_during_reopen_prevents_retry() {
        let stop = AtomicBool::new(false);
        let attempts = Cell::new(0);
        let result: Result<()> = retry_transport_operation(
            &mut (),
            &stop,
            |_| {
                attempts.set(attempts.get() + 1);
                Err(lianli_transport::TransportError::Usb(rusb::Error::NoDevice).into())
            },
            |_, _| {
                stop.store(true, Ordering::Release);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(stop.load(Ordering::Acquire));
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn running_controller_reopens_once_and_returns_retry_result() {
        let stop = AtomicBool::new(false);
        let attempts = Cell::new(0);
        let reopens = Cell::new(0);
        let result = retry_transport_operation(
            &mut (),
            &stop,
            |_| {
                attempts.set(attempts.get() + 1);
                if attempts.get() == 1 {
                    return Err(lianli_transport::TransportError::Usb(rusb::Error::NoDevice).into());
                }
                Ok(42)
            },
            |_, _| {
                reopens.set(reopens.get() + 1);
                Ok(())
            },
        );
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.get(), 2);
        assert_eq!(reopens.get(), 1);
    }
}

use super::ServiceManager;
use lianli_devices::wireless::{
    Dongle, DonglePresence, RecoveryAttempt, RecoveryLadder, RecoveryPlan, WirelessController,
};
use lianli_shared::ipc::WirelessDongle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;
use tracing::{debug, info, warn};

const UNRESPONSIVE_AFTER_RESETS: usize = 2;

struct RecoveryWorker {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<(WirelessController, RecoveryAttempt)>,
}

#[derive(Default)]
pub(super) struct WirelessRecovery {
    ladder: RecoveryLadder,
    worker: Option<RecoveryWorker>,
    presence: DonglePresence,
    link_up: bool,
    ever_connected: bool,
    last_error: Option<String>,
    unresponsive: Option<Dongle>,
}

impl WirelessRecovery {
    fn take_finished(&mut self) -> Option<thread::Result<(WirelessController, RecoveryAttempt)>> {
        if !self.worker.as_ref()?.handle.is_finished() {
            return None;
        }
        self.worker.take().map(|worker| worker.handle.join())
    }

    pub(super) fn unresponsive_dongle(&self) -> Option<WirelessDongle> {
        self.unresponsive.map(|dongle| match dongle {
            Dongle::Tx => WirelessDongle::Transmitter,
            Dongle::Rx => WirelessDongle::Receiver,
        })
    }

    /// A dongle that stays plugged in through repeated failed companion
    /// resets needs a physical replug.
    fn record(&mut self, attempt: &RecoveryAttempt, now: Instant) {
        self.ladder.record(attempt, now);
        if attempt.result.is_ok() {
            self.unresponsive = None;
            return;
        }
        if let Some(target) = attempt.reset {
            if self.ladder.resets() >= UNRESPONSIVE_AFTER_RESETS && self.present(target) {
                self.unresponsive = Some(target);
            }
        }
    }

    fn present(&self, dongle: Dongle) -> bool {
        location(self.presence, dongle).is_some()
    }

    /// Stores a fresh scan and reports whether it changed. A flagged dongle
    /// that moved or vanished was replugged, so the flag no longer applies.
    fn observe(&mut self, presence: DonglePresence) -> bool {
        if let Some(dongle) = self.unresponsive {
            if location(presence, dongle) != location(self.presence, dongle) {
                self.unresponsive = None;
            }
        }
        std::mem::replace(&mut self.presence, presence) != presence
    }

    pub(super) fn cancel(&self) {
        if let Some(worker) = &self.worker {
            worker.stop.store(true, Ordering::Release);
        }
    }

    pub(super) fn shutdown(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };
        worker.stop.store(true, Ordering::Release);
        if worker.handle.join().is_err() {
            warn!("Wireless recovery worker panicked during shutdown");
        }
    }
}

fn location(presence: DonglePresence, dongle: Dongle) -> Option<(u8, u8)> {
    match dongle {
        Dongle::Tx => presence.tx,
        Dongle::Rx => presence.rx,
    }
}

impl ServiceManager {
    fn observe_wireless_presence(&mut self, presence: DonglePresence) -> bool {
        let flagged = self.wireless_recovery.unresponsive.is_some();
        let changed = self.wireless_recovery.observe(presence);
        if flagged && self.wireless_recovery.unresponsive.is_none() {
            self.ipc.state.lock().state_health.wireless_unavailable(
                "A wireless dongle was reconnected. The daemon is reconnecting the dongles.",
            );
        }
        changed
    }

    pub(super) fn connect_wireless_at_startup(&mut self) {
        let presence = DonglePresence::scan();
        self.wireless_recovery.presence = presence;
        if presence.tx.is_none() {
            self.report_missing_transmitter();
            return;
        }
        let result = self
            .wireless
            .connect()
            .and_then(|()| self.wireless.start_polling());
        self.finish_wireless_attempt(RecoveryAttempt {
            reset: None,
            result,
        });
    }

    pub(super) fn poll_wireless_recovery(&mut self) {
        let Some(finished) = self.wireless_recovery.take_finished() else {
            return;
        };
        // A companion reset re-enumerates the dongle; that must not count as hotplug.
        self.observe_wireless_presence(DonglePresence::scan());
        let attempt = match finished {
            Ok((controller, attempt)) => {
                if attempt.result.is_ok() {
                    self.wireless = controller;
                }
                attempt
            }
            Err(_) => RecoveryAttempt {
                reset: None,
                result: Err(anyhow::anyhow!("wireless recovery worker panicked")),
            },
        };
        self.finish_wireless_attempt(attempt);
    }

    pub(super) fn supervise_wireless(&mut self) {
        self.poll_wireless_recovery();
        if self.wireless_recovery.worker.is_some() {
            return;
        }
        if self.wireless.is_connected() {
            if self.wireless_rx_init_pending {
                self.init_wireless_receiver();
            }
            return;
        }
        if std::mem::take(&mut self.wireless_recovery.link_up) {
            warn!("Wireless link lost; releasing the dongles and starting recovery");
            self.release_wireless();
            self.ipc.state.lock().state_health.wireless_unavailable(
                "The wireless link was lost. The daemon is reconnecting the dongles.",
            );
        }

        let now = Instant::now();
        let presence = DonglePresence::scan();
        if self.observe_wireless_presence(presence) {
            debug!(?presence, "Wireless dongle presence changed");
            self.wireless_recovery.ladder.hotplug(now);
        }
        let recoverable = presence.tx.is_some()
            || (presence.rx.is_some()
                && (self.wireless_recovery.ever_connected || self.wireless_configured()));
        if !recoverable {
            self.report_missing_transmitter();
            return;
        }
        if let Some(plan) = self.wireless_recovery.ladder.next_attempt(now) {
            self.start_wireless_recovery(plan);
        }
    }

    fn release_wireless(&mut self) {
        if let Some(rgb) = &self.controllers.rgb {
            rgb.lock().set_wireless(None);
        }
        self.wireless.stop();
    }

    fn start_wireless_recovery(&mut self, plan: RecoveryPlan) {
        self.release_wireless();
        let mut controller = self.wireless.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        debug!(?plan, "Starting wireless recovery attempt");
        let spawned = thread::Builder::new()
            .name("wireless-recovery".into())
            .spawn(move || {
                let attempt = controller.recover(plan, &worker_stop);
                (controller, attempt)
            });
        match spawned {
            Ok(handle) => self.wireless_recovery.worker = Some(RecoveryWorker { stop, handle }),
            Err(error) => self.finish_wireless_attempt(RecoveryAttempt {
                reset: None,
                result: Err(anyhow::Error::new(error).context("spawning wireless recovery")),
            }),
        }
    }

    fn finish_wireless_attempt(&mut self, attempt: RecoveryAttempt) {
        self.wireless_recovery.record(&attempt, Instant::now());
        if let Err(error) = attempt.result {
            let error = format!("{error:#}");
            let unresponsive = self.wireless_recovery.unresponsive_dongle();
            {
                let mut state = self.ipc.state.lock();
                match unresponsive {
                    Some(dongle) => state
                        .state_health
                        .wireless_dongle_unresponsive(dongle, &error),
                    None => state.state_health.wireless_unavailable(&error),
                }
            }
            if self.wireless_recovery.last_error.as_ref() != Some(&error) {
                warn!(%error, "Wireless reconnect failed; wireless fans and pumps keep their last speed until it recovers");
                self.wireless_recovery.last_error = Some(error);
            } else {
                debug!(%error, "Wireless reconnect failed again");
            }
            return;
        }
        if self.wireless_recovery.last_error.take().is_some() {
            info!(reset = ?attempt.reset, "Wireless link recovered");
        }
        self.wireless_recovery.link_up = true;
        self.wireless_recovery.ever_connected = true;
        self.init_wireless_receiver();
        info!("Wireless links active");
        if self.controllers.fan.is_some() || self.controllers.aio.is_some() {
            self.start_fan_control();
            self.start_aio_control();
        }
        self.rebuild_rgb_controller();
    }

    fn wireless_configured(&self) -> bool {
        self.configured_wireless_device_ids()
            .iter()
            .any(|id| id.starts_with("wireless:"))
    }

    fn report_missing_transmitter(&mut self) {
        let configured = self.wireless_configured();
        let mut state = self.ipc.state.lock();
        if configured {
            state.state_health.wireless_unavailable(
                "Saved settings reference wireless devices, but no transmitter was detected.",
            );
        } else if self.wireless_recovery.ever_connected {
            state.state_health.wireless_unavailable(
                "The wireless transmitter disconnected and has not returned.",
            );
        } else {
            state.state_health.wireless_ready();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(reset: Option<Dongle>, ok: bool) -> RecoveryAttempt {
        RecoveryAttempt {
            reset,
            result: if ok {
                Ok(())
            } else {
                Err(anyhow::anyhow!("TX is not accepting USB writes"))
            },
        }
    }

    fn with_both_dongles() -> WirelessRecovery {
        WirelessRecovery {
            presence: DonglePresence {
                tx: Some((8, 22)),
                rx: Some((8, 21)),
            },
            ..Default::default()
        }
    }

    #[test]
    fn present_dongle_is_unresponsive_only_after_repeated_failed_resets() {
        let now = Instant::now();
        let mut recovery = with_both_dongles();
        recovery.record(&attempt(None, false), now);
        recovery.record(&attempt(Some(Dongle::Tx), false), now);
        assert_eq!(recovery.unresponsive_dongle(), None);
        recovery.record(&attempt(Some(Dongle::Tx), false), now);
        assert_eq!(
            recovery.unresponsive_dongle(),
            Some(WirelessDongle::Transmitter)
        );
        recovery.record(&attempt(None, false), now);
        assert_eq!(
            recovery.unresponsive_dongle(),
            Some(WirelessDongle::Transmitter)
        );
        recovery.record(&attempt(None, true), now);
        assert_eq!(recovery.unresponsive_dongle(), None);
    }

    #[test]
    fn replugging_the_flagged_dongle_clears_it_even_mid_attempt() {
        let now = Instant::now();
        let mut recovery = with_both_dongles();
        for _ in 0..2 {
            recovery.record(&attempt(Some(Dongle::Tx), false), now);
        }
        let stuck = recovery.presence;
        assert!(!recovery.observe(stuck));
        assert!(recovery.unresponsive_dongle().is_some());
        let unplugged = DonglePresence { tx: None, ..stuck };
        assert!(recovery.observe(unplugged));
        assert_eq!(recovery.unresponsive_dongle(), None);

        for _ in 0..2 {
            recovery.record(&attempt(Some(Dongle::Tx), false), now);
        }
        assert_eq!(recovery.unresponsive_dongle(), None);
        let replugged = DonglePresence {
            tx: Some((8, 30)),
            ..stuck
        };
        recovery.observe(replugged);
        for _ in 0..2 {
            recovery.record(&attempt(Some(Dongle::Tx), false), now);
        }
        assert!(recovery.unresponsive_dongle().is_some());
        recovery.observe(DonglePresence {
            tx: Some((8, 31)),
            ..replugged
        });
        assert_eq!(recovery.unresponsive_dongle(), None);
    }

    #[test]
    fn missing_dongle_is_never_reported_as_unresponsive() {
        let now = Instant::now();
        let mut recovery = WirelessRecovery {
            presence: DonglePresence {
                tx: None,
                rx: Some((8, 21)),
            },
            ..Default::default()
        };
        for _ in 0..3 {
            recovery.record(&attempt(Some(Dongle::Tx), false), now);
        }
        assert_eq!(recovery.unresponsive_dongle(), None);
    }
}

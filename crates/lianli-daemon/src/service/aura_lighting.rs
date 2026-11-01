//! Opt-in owner Rainbow stream lifecycle for the ASUS Aura controller.

use hidapi::{DeviceInfo, HidApi};
use lianli_devices::asus_aura::{
    query_configuration, run_rainbow_stream_observed, AuraSyncRainbow, AuraWriterLock, PRODUCT_ID,
    VENDOR_ID,
};
use lianli_shared::config::AsusAuraConfig;
use lianli_shared::ipc::{AuraLightingStatus, AuraWorkerState};
use lianli_transport::HidrawTransport;
use parking_lot::Mutex;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const AURA_USAGE_PAGE: u16 = 0xFF72;
const RETRY_DELAY: Duration = Duration::from_secs(2);

struct Worker {
    stop: Sender<()>,
    join: JoinHandle<()>,
}

/// Owns at most one direct ASUS writer. Disabling/reloading the feature stops and joins it.
pub(super) struct AuraLightingService {
    status: Arc<Mutex<AuraLightingStatus>>,
    worker: Option<Worker>,
    applied_config: Option<AsusAuraConfig>,
}

impl AuraLightingService {
    pub(super) fn new(status: Arc<Mutex<AuraLightingStatus>>) -> Self {
        Self {
            status,
            worker: None,
            applied_config: None,
        }
    }

    pub(super) fn apply_config(&mut self, config: &AsusAuraConfig) {
        if self.applied_config.as_ref() == Some(config)
            && (!config.rainbow_enabled || self.worker.is_some())
        {
            return;
        }
        self.applied_config = Some(config.clone());
        if self.worker.is_some() {
            self.stop();
        }
        if config.rainbow_enabled {
            self.start(config.clone());
        } else {
            self.status.lock().worker_state = AuraWorkerState::Disabled;
        }
        self.status.lock().configured_enabled = config.rainbow_enabled;
    }

    pub(super) fn reconnect(&mut self) {
        let Some(config) = self.applied_config.clone() else {
            return;
        };
        self.applied_config = None;
        self.apply_config(&config);
    }

    fn start(&mut self, config: AsusAuraConfig) {
        let (stop_tx, stop_rx) = mpsc::channel();
        let status = Arc::clone(&self.status);
        self.status.lock().worker_state = AuraWorkerState::Connecting;
        self.status.lock().last_error = None;
        let join = thread::Builder::new()
            .name("asus-aura-rainbow".into())
            .spawn(move || {
                let mut complete_frames = 0u64;
                loop {
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                    *status.lock() = AuraLightingStatus {
                        configured_enabled: true,
                        worker_state: if complete_frames == 0 {
                            AuraWorkerState::Connecting
                        } else {
                            AuraWorkerState::Reconnecting
                        },
                        complete_frames_accepted: complete_frames,
                        reported_argb_channels: None,
                        last_error: None,
                        device_state_readback_available: false,
                    };
                    let result = stream_once(&stop_rx, &status, &mut complete_frames, &config);
                    match result {
                        Ok(()) => break,
                        Err(error) => {
                            let mut current = status.lock();
                            current.worker_state = AuraWorkerState::Reconnecting;
                            current.complete_frames_accepted = complete_frames;
                            current.last_error = Some(format!("{error:#}"));
                            drop(current);
                            if stop_rx.recv_timeout(RETRY_DELAY).is_ok() {
                                break;
                            }
                        }
                    }
                }
                let mut current = status.lock();
                current.worker_state = AuraWorkerState::Stopped;
                current.complete_frames_accepted = complete_frames;
                current.device_state_readback_available = false;
            });
        match join {
            Ok(join) => {
                self.worker = Some(Worker {
                    stop: stop_tx,
                    join,
                });
            }
            Err(error) => {
                let mut status = self.status.lock();
                status.worker_state = AuraWorkerState::Reconnecting;
                status.last_error = Some(format!("could not start ASUS Aura worker: {error}"));
            }
        }
    }

    pub(super) fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.stop.send(());
            if worker.join.join().is_err() {
                self.status.lock().last_error = Some("ASUS Aura worker panicked".into());
            }
        }
        self.status.lock().worker_state = AuraWorkerState::Stopped;
    }
}

impl Drop for AuraLightingService {
    fn drop(&mut self) {
        self.stop();
    }
}

fn stream_once(
    stop: &mpsc::Receiver<()>,
    status: &Arc<Mutex<AuraLightingStatus>>,
    complete_frames: &mut u64,
    config: &AsusAuraConfig,
) -> anyhow::Result<()> {
    let api = HidApi::new()?;
    let candidates: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|info| {
            info.vendor_id() == VENDOR_ID
                && info.product_id() == PRODUCT_ID
                && info.usage_page() == AURA_USAGE_PAGE
        })
        .collect();
    anyhow::ensure!(
        candidates.len() == 1,
        "expected one ASUS Aura interface {:04x}:{:04x} usage page 0x{AURA_USAGE_PAGE:04x}, found {}",
        VENDOR_ID,
        PRODUCT_ID,
        candidates.len()
    );
    let info = candidates[0];
    let path = info.path();
    let path_os = OsStr::from_bytes(path.to_bytes()).to_os_string();
    let _writer_lock = AuraWriterLock::acquire(Path::new(&path_os))?;
    let device = api.open_path(path)?;
    let mut transport = HidrawTransport::new(device, Some(path_os));
    let configuration = query_configuration(&mut transport)?;
    status.lock().reported_argb_channels = Some(configuration.reported_channel_count);
    let targets =
        configuration.rainbow_targets_with_reversed_channels(&config.reverse_argb_channels)?;
    let animation = AuraSyncRainbow {
        wavelength_leds: config.rainbow_wavelength_leds,
        frequency_hz: config.rainbow_frequency_hz,
        phase_degrees: config.rainbow_phase_degrees,
        frame_interval: Duration::from_millis(u64::from(config.rainbow_frame_interval_ms)),
    };
    status.lock().worker_state = AuraWorkerState::Streaming;
    run_rainbow_stream_observed(
        &mut transport,
        &configuration,
        &targets,
        animation,
        stop,
        || {
            *complete_frames = complete_frames.saturating_add(1);
            status.lock().complete_frames_accepted = *complete_frames;
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_configuration_keeps_asus_worker_stopped_without_opening_hardware() {
        let status = Arc::new(Mutex::new(AuraLightingStatus::default()));
        let mut service = AuraLightingService::new(Arc::clone(&status));
        let config = AsusAuraConfig {
            rainbow_enabled: false,
            ..AsusAuraConfig::default()
        };

        service.apply_config(&config);

        assert!(!status.lock().configured_enabled);
        assert_eq!(status.lock().worker_state, AuraWorkerState::Disabled);
        assert_eq!(status.lock().complete_frames_accepted, 0);
        service.stop();
    }
}

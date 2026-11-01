//! OpenRGB Direct frames for RF devices. Every frame costs a full RF upload,
//! so live frames are rate limited and repeating streams are uploaded once as
//! a native loop that the devices play on the RF master clock.
use super::direct_color::DirectColorBuffer;
use super::live_loop::{phase_aligned, Cycle, Frame, Recorder};
use super::RgbController;
use anyhow::{Context, Result};
use lianli_devices::wireless::{WirelessController, WirelessRgbUpload};
use lianli_shared::rgb::RgbPlaybackTiming;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

const LIVE_FRAME_INTERVAL: Duration = Duration::from_millis(200);
const LIVE_UPLOAD_SPACING: Duration = Duration::from_millis(150);
const LOOP_UPLOAD_PAUSE: Duration = Duration::from_millis(500);
const DETECT_INTERVAL: Duration = Duration::from_secs(1);
const LOCK_RETRY: Duration = Duration::from_secs(10);
const RELOCK_INTERVAL: Duration = Duration::from_secs(5);
const MAX_LOOP_INTERVAL: Duration = Duration::from_millis(200);
const MAX_LOOP_DIVISOR: usize = 4;
const MIN_BREAK_DISTANCE: f32 = 10.0;
const BREAK_STRIKES: u8 = 3;
const DRIFT_STRIKES: u8 = 10;
const MAX_RELOCK_FAILURES: u8 = 3;

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Holding,
    Drifting,
    Broken,
}

struct Locked {
    cycle: Cycle,
    upload: Arc<WirelessRgbUpload>,
    since: Instant,
    strikes: u8,
    drift: u8,
    relock_failures: u8,
}

impl Locked {
    fn new(cycle: Cycle, upload: Arc<WirelessRgbUpload>, since: Instant) -> Self {
        Self {
            cycle,
            upload,
            since,
            strikes: 0,
            drift: 0,
            relock_failures: 0,
        }
    }

    fn observe(&mut self, at: Instant, frame: &[[u8; 3]]) -> Verdict {
        let distance = self.cycle.distance(at, frame);
        let tolerance = self.cycle.tolerance;
        if distance > MIN_BREAK_DISTANCE.max(3.0 * tolerance) {
            self.strikes += 1;
            return if self.strikes >= BREAK_STRIKES {
                Verdict::Broken
            } else {
                Verdict::Holding
            };
        }
        self.strikes = 0;
        self.drift = if distance > 1.5 * tolerance {
            self.drift.saturating_add(1)
        } else {
            0
        };
        if self.drift >= DRIFT_STRIKES {
            Verdict::Drifting
        } else {
            Verdict::Holding
        }
    }

    fn wants_relock(&self, now: Instant) -> bool {
        self.drift >= DRIFT_STRIKES && now >= self.since + RELOCK_INTERVAL
    }
}

struct LiveDevice {
    counts: Vec<usize>,
    colors: Vec<[u8; 3]>,
    dirty: bool,
    last_upload: Option<Instant>,
    recorder: Recorder,
    locked: Option<Locked>,
    next_detect: Instant,
    failing: bool,
}

impl LiveDevice {
    fn new(counts: Vec<usize>, colors: Vec<[u8; 3]>) -> Self {
        Self {
            counts,
            colors,
            dirty: false,
            last_upload: None,
            recorder: Recorder::new(),
            locked: None,
            next_detect: Instant::now() + DETECT_INTERVAL,
            failing: false,
        }
    }

    fn merge(&mut self, zones: &HashMap<u8, Vec<[u8; 3]>>) {
        for (&zone, colors) in zones {
            let Some(&count) = self.counts.get(usize::from(zone)) else {
                continue;
            };
            let start: usize = self.counts[..usize::from(zone)].iter().sum();
            let len = colors.len().min(count);
            self.colors[start..start + len].copy_from_slice(&colors[..len]);
        }
    }

    fn zones(&self) -> HashMap<u8, Vec<[u8; 3]>> {
        let mut start = 0;
        self.counts
            .iter()
            .enumerate()
            .map(|(zone, &count)| {
                let colors = self.colors[start..start + count].to_vec();
                start += count;
                (zone as u8, colors)
            })
            .collect()
    }

    fn wants_live_frame(&self) -> bool {
        self.locked.is_none() && self.dirty
    }

    fn wants_detection(&self, now: Instant) -> bool {
        now >= self.next_detect
            && self.recorder.is_live(now)
            && self
                .locked
                .as_ref()
                .is_none_or(|locked| locked.wants_relock(now))
    }

    fn unlock(&mut self, id: &str, reason: &str) {
        if self.locked.take().is_some() {
            debug!(device = %id, reason, "Wireless RGB loop released; streaming live frames");
            self.dirty = true;
            self.next_detect = Instant::now() + DETECT_INTERVAL;
        }
    }
}

#[derive(Default)]
pub(super) struct LiveWireless {
    devices: HashMap<String, LiveDevice>,
    next_upload: Option<Instant>,
}

impl LiveWireless {
    pub fn forget(&mut self, id: &str) {
        self.devices.remove(id);
    }

    pub fn receive(
        &mut self,
        rgb: &RgbController,
        id: &str,
        zones: &HashMap<u8, Vec<[u8; 3]>>,
        at: Instant,
    ) {
        let Some((counts, colors)) = rgb.wireless_live_layout(id) else {
            self.devices.remove(id);
            return;
        };
        if self
            .devices
            .get(id)
            .is_none_or(|device| device.counts != counts)
        {
            self.devices
                .insert(id.to_owned(), LiveDevice::new(counts, colors));
        }
        let device = self.devices.get_mut(id).unwrap();
        device.merge(zones);
        let frame: Frame = device.colors.as_slice().into();
        device.recorder.push(at, frame.clone());
        let Some(locked) = device.locked.as_mut() else {
            device.dirty = true;
            return;
        };
        if !rgb.wireless_upload_current(id, &locked.upload) {
            device.unlock(id, "the loop was replaced");
        } else if locked.observe(at, &frame) == Verdict::Broken {
            device.unlock(id, "the stream no longer matches");
        }
    }

    pub fn has_work(&self, now: Instant) -> bool {
        self.devices
            .values()
            .any(|device| device.wants_live_frame() || device.wants_detection(now))
    }

    pub fn service(
        &mut self,
        rgb: &Mutex<RgbController>,
        buffer: &Mutex<DirectColorBuffer>,
        now: Instant,
    ) {
        if self.next_upload.is_some_and(|next| now < next) {
            return;
        }
        if !self.prune(&rgb.lock(), buffer) {
            return;
        }
        if let Some(id) = self.lock_candidate(now) {
            if self.try_lock(rgb, buffer, &id, now) {
                return;
            }
        }
        let due = pick_due(
            self.devices
                .iter()
                .filter(|(_, device)| device.wants_live_frame())
                .map(|(id, device)| (id.as_str(), device.last_upload)),
            now,
        )
        .map(str::to_owned);
        let Some(id) = due else {
            return;
        };
        let result = {
            let mut rgb = rgb.lock();
            if !self.prune(&rgb, buffer) {
                return;
            }
            let Some(device) = self.devices.get(&id) else {
                return;
            };
            rgb.apply_wireless_batch(&id, &device.zones())
        };
        let device = self.devices.get_mut(&id).unwrap();
        device.last_upload = Some(now);
        self.next_upload = Some(now + LIVE_UPLOAD_SPACING);
        match result {
            Ok(()) => {
                device.dirty = false;
                device.failing = false;
            }
            Err(error) => {
                if !std::mem::replace(&mut device.failing, true) {
                    debug!(device = %id, %error, "Wireless RGB live frame failed");
                }
            }
        }
    }

    /// Drops state that no longer belongs to the OpenRGB stream. A mode change
    /// is applied under the RGB lock, so checking discards under the same lock
    /// keeps a held frame from overwriting it.
    fn prune(&mut self, rgb: &RgbController, buffer: &Mutex<DirectColorBuffer>) -> bool {
        for id in buffer.lock().take_discarded() {
            self.devices.remove(&id);
        }
        if !rgb.is_openrgb_controlled() {
            self.devices.clear();
            return false;
        }
        self.devices.retain(|id, _| rgb.is_wireless_rgb(id));
        true
    }

    fn lock_candidate(&mut self, now: Instant) -> Option<String> {
        let (id, device) = self
            .devices
            .iter_mut()
            .find(|(_, device)| device.wants_detection(now))?;
        device.next_detect = now + DETECT_INTERVAL;
        Some(id.clone())
    }

    fn try_lock(
        &mut self,
        rgb: &Mutex<RgbController>,
        buffer: &Mutex<DirectColorBuffer>,
        id: &str,
        now: Instant,
    ) -> bool {
        let device = self.devices.get_mut(id).unwrap();
        let Some(cycle) = device.recorder.detect() else {
            if let Some(locked) = device.locked.as_mut() {
                locked.relock_failures += 1;
                if locked.relock_failures >= MAX_RELOCK_FAILURES {
                    device.unlock(id, "the stream stopped repeating");
                }
            }
            return false;
        };
        let colors = device.colors.clone();
        let result = (|| {
            let (wireless, mac) = rgb.lock().wireless_loop_target(id)?;
            let clock = wireless.read_rgb_clock()?;
            let upload = Arc::new(fit_cycle(&wireless, &mac, &cycle, clock)?);
            let mut rgb = rgb.lock();
            anyhow::ensure!(
                self.prune(&rgb, buffer) && self.devices.contains_key(id),
                "OpenRGB no longer streams to this device"
            );
            rgb.submit_wireless_loop(id, &wireless, upload.clone(), &colors)?;
            Ok(upload)
        })();
        let Some(device) = self.devices.get_mut(id) else {
            return false;
        };
        match result {
            Ok(upload) => {
                debug!(
                    device = %id,
                    period_ms = cycle.period.as_millis() as u64,
                    frames = upload.frame_count(),
                    "OpenRGB stream repeats; playing it as a wireless RGB loop"
                );
                device.locked = Some(Locked::new(cycle, upload, now));
                device.dirty = false;
                device.last_upload = Some(now);
                self.next_upload = Some(now + LOOP_UPLOAD_PAUSE);
                true
            }
            Err(error) => {
                debug!(device = %id, %error, "Could not upload the repeating OpenRGB stream as a loop");
                device.next_detect = now + LOCK_RETRY;
                false
            }
        }
    }
}

/// Lowers the frame rate until the loop fits the device's compressed RGB memory.
fn fit_cycle(
    wireless: &WirelessController,
    mac: &[u8; 6],
    cycle: &Cycle,
    clock: (u32, Instant),
) -> Result<WirelessRgbUpload> {
    let mut last_error = None;
    for divisor in 1..=MAX_LOOP_DIVISOR {
        let count = cycle.frames.len() / divisor;
        if count < 2 || cycle.period / count as u32 > MAX_LOOP_INTERVAL {
            break;
        }
        let interval_hundredths =
            (cycle.period.as_secs_f64() / count as f64 * 160_000.0).round() as u32;
        let frames: Vec<Vec<[u8; 3]>> = phase_aligned(
            &cycle.resampled(count),
            cycle.start,
            interval_hundredths,
            clock,
        )
        .iter()
        .map(|frame| frame.to_vec())
        .collect();
        let timing = RgbPlaybackTiming {
            interval_hundredths,
            ..Default::default()
        };
        match wireless.prepare_rgb_animation(mac, &frames, timing) {
            Ok(upload) => return Ok(upload),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("loop is too slow to upload")))
        .context("preparing wireless RGB loop")
}

/// Picks the device whose live frame has waited longest, honouring the
/// per-device frame interval.
fn pick_due<'a>(
    candidates: impl Iterator<Item = (&'a str, Option<Instant>)>,
    now: Instant,
) -> Option<&'a str> {
    candidates
        .filter(|(_, last)| last.is_none_or(|last| now >= last + LIVE_FRAME_INTERVAL))
        .min_by_key(|(_, last)| *last)
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_frames_respect_device_interval_and_serve_the_longest_waiting_device() {
        let now = Instant::now() + Duration::from_secs(1);
        let recent = Some(now - Duration::from_millis(100));
        let older = Some(now - Duration::from_millis(400));
        let oldest = Some(now - Duration::from_millis(900));
        assert_eq!(pick_due([("a", recent)].into_iter(), now), None);
        assert_eq!(
            pick_due(
                [("a", older), ("b", oldest), ("c", recent)].into_iter(),
                now
            ),
            Some("b")
        );
        assert_eq!(
            pick_due([("a", older), ("new", None)].into_iter(), now),
            Some("new")
        );
    }

    fn ramp_lock(start: Instant) -> Locked {
        let frames = (0..20u8)
            .map(|step| vec![[step * 10; 3]; 4].into())
            .collect();
        let upload = WirelessRgbUpload::new(&[vec![[0; 3]; 4]], 50, None).unwrap();
        Locked::new(
            Cycle {
                frames,
                start,
                period: Duration::from_secs(1),
                tolerance: 4.0,
            },
            Arc::new(upload),
            start,
        )
    }

    #[test]
    fn locked_loop_holds_for_matching_frames_and_breaks_on_a_new_effect() {
        let start = Instant::now();
        let mut locked = ramp_lock(start);
        for step in 0..60u32 {
            let at = start + Duration::from_millis(u64::from(step) * 50);
            let value = (step % 20) as u8 * 10;
            assert_eq!(locked.observe(at, &[[value; 3]; 4]), Verdict::Holding);
        }
        let at = start + Duration::from_secs(3);
        assert_eq!(locked.observe(at, &[[255, 0, 0]; 4]), Verdict::Holding);
        assert_eq!(locked.observe(at, &[[255, 0, 0]; 4]), Verdict::Holding);
        assert_eq!(locked.observe(at, &[[255, 0, 0]; 4]), Verdict::Broken);
    }

    #[test]
    fn sustained_small_mismatch_requests_a_relock_after_the_minimum_interval() {
        let start = Instant::now();
        let mut locked = ramp_lock(start);
        let mut verdict = Verdict::Holding;
        for step in 0..DRIFT_STRIKES {
            let at = start + Duration::from_millis(u64::from(step) * 50);
            let value = (step % 20) * 10;
            verdict = locked.observe(at, &[[value + 21, value, value]; 4]);
        }
        assert_eq!(verdict, Verdict::Drifting);
        assert!(!locked.wants_relock(start + Duration::from_secs(1)));
        assert!(locked.wants_relock(start + RELOCK_INTERVAL));
        let value = 10 * 10;
        let at = start + Duration::from_millis(500);
        assert_eq!(locked.observe(at, &[[value; 3]; 4]), Verdict::Holding);
        assert!(!locked.wants_relock(start + RELOCK_INTERVAL));
    }
}

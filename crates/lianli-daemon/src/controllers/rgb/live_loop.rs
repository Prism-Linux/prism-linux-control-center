use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) type Frame = Arc<[[u8; 3]]>;

pub(super) const SLOT: Duration = Duration::from_millis(50);
const MIN_PERIOD: usize = 10;
const MAX_PERIOD: usize = 240;
const HISTORY: usize = 2 * MAX_PERIOD;
const STALL: Duration = Duration::from_millis(500);
const SAMPLES: usize = 24;
const MATCH_TOLERANCE: f32 = 3.0;
const MIN_CONTRAST: f32 = 12.0;
const CONTRAST_RATIO: f32 = 4.0;
const RF_TICK_NANOS: i128 = 625_000;

/// Live frames resampled onto a fixed grid with sample-and-hold.
pub(super) struct Recorder {
    origin: Instant,
    slots: VecDeque<Frame>,
    emitted: u64,
    latest: Option<Frame>,
    last_input: Option<Instant>,
}

/// One period of a repeating stream, played back from `start` over `period`.
pub(super) struct Cycle {
    pub frames: Vec<Frame>,
    pub start: Instant,
    pub period: Duration,
    pub tolerance: f32,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            slots: VecDeque::new(),
            emitted: 0,
            latest: None,
            last_input: None,
        }
    }

    pub fn push(&mut self, at: Instant, frame: Frame) {
        let stalled = self
            .last_input
            .is_some_and(|last| at.saturating_duration_since(last) > STALL);
        let resized = self
            .latest
            .as_ref()
            .is_some_and(|latest| latest.len() != frame.len());
        if stalled || resized {
            *self = Self::new();
        }
        if let Some(latest) = &self.latest {
            while self.slot_time(self.emitted) < at {
                self.slots.push_back(latest.clone());
                self.emitted += 1;
                if self.slots.len() > HISTORY {
                    self.slots.pop_front();
                }
            }
        } else {
            self.origin = at;
        }
        self.latest = Some(frame);
        self.last_input = Some(at);
    }

    pub fn is_live(&self, now: Instant) -> bool {
        self.last_input
            .is_some_and(|last| now.saturating_duration_since(last) <= STALL)
    }

    fn slot_time(&self, index: u64) -> Instant {
        self.origin + Duration::from_millis(SLOT.as_millis() as u64 * index)
    }

    pub fn detect(&self) -> Option<Cycle> {
        let count = self.slots.len();
        let longest = (count / 2).min(MAX_PERIOD);
        if longest < MIN_PERIOD {
            return None;
        }
        let motion = self.lag_distance(1, longest, SAMPLES);
        let tolerance = MATCH_TOLERANCE + motion / 2.0;
        let error = |period| self.lag_distance(period, period, SAMPLES);
        let first = (MIN_PERIOD..=longest).find(|&period| {
            let error = error(period);
            error <= tolerance && {
                let contrast = self.lag_distance(period / 2, period, SAMPLES);
                contrast >= MIN_CONTRAST && contrast >= CONTRAST_RATIO * error
            }
        })?;
        let period =
            (first..=(first + 2).min(longest)).min_by(|a, b| error(*a).total_cmp(&error(*b)))?;
        if self.lag_distance(period, period, 1) > tolerance {
            return None;
        }
        // Mean absolute error grows linearly with phase error, so a V fit
        // recovers the fractional period that integer slots cannot express.
        let refined = if period > MIN_PERIOD && period < longest {
            let (before, at, after) = (error(period - 1), error(period), error(period + 1));
            let rise = before.max(after) - at;
            if rise > f32::EPSILON {
                f64::from(((before - after) / (2.0 * rise)).clamp(-0.5, 0.5))
            } else {
                0.0
            }
        } else {
            0.0
        };
        let first_slot = count - period;
        Some(Cycle {
            frames: self.slots.range(first_slot..).cloned().collect(),
            start: self.slot_time(self.emitted - period as u64),
            period: SLOT.mul_f64(period as f64 + refined),
            tolerance,
        })
    }

    fn lag_distance(&self, lag: usize, span: usize, samples: usize) -> f32 {
        let count = self.slots.len();
        let step = (span / samples).max(1);
        let (sum, measured) =
            (count - span..count)
                .step_by(step)
                .fold((0.0, 0u32), |(sum, measured), index| {
                    (
                        sum + distance(&self.slots[index], &self.slots[index - lag]),
                        measured + 1,
                    )
                });
        sum / measured.max(1) as f32
    }
}

impl Cycle {
    /// Distance from a live frame to the nearest loop frame around its phase,
    /// which absorbs arrival jitter of about one frame.
    pub fn distance(&self, at: Instant, frame: &[[u8; 3]]) -> f32 {
        let count = self.frames.len() as u64;
        let position = at.saturating_duration_since(self.start).as_secs_f64()
            / self.period.as_secs_f64()
            * count as f64;
        let index = position as u64 % count;
        [count - 1, 0, 1]
            .into_iter()
            .map(|offset| distance(&self.frames[((index + offset) % count) as usize], frame))
            .fold(f32::MAX, f32::min)
    }

    pub fn resampled(&self, count: usize) -> Vec<Frame> {
        let available = self.frames.len();
        (0..count)
            .map(|index| self.frames[index * available / count].clone())
            .collect()
    }
}

/// Rotates a loop so the device, which shows frame
/// `ticks * 100 / interval_hundredths % len` of the RF master clock,
/// shows `frames[0]` at `start`.
pub(super) fn phase_aligned(
    frames: &[Frame],
    start: Instant,
    interval_hundredths: u32,
    clock: (u32, Instant),
) -> Vec<Frame> {
    let (ticks, sampled) = clock;
    let offset = if start >= sampled {
        (start - sampled).as_nanos() as i128
    } else {
        -((sampled - start).as_nanos() as i128)
    };
    let start_ticks = (i128::from(ticks) + offset / RF_TICK_NANOS).rem_euclid(1 << 32) as u64;
    let shift = start_ticks * 100 / u64::from(interval_hundredths) % frames.len() as u64;
    let mut aligned = frames.to_vec();
    aligned.rotate_right(shift as usize);
    aligned
}

fn distance(a: &[[u8; 3]], b: &[[u8; 3]]) -> f32 {
    let sum: u32 = a
        .iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| u32::from(x.abs_diff(*y)))
        .sum();
    sum as f32 / (a.len() * 3).max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    const FPS: f64 = 60.0;

    fn hue(position: f64) -> [u8; 3] {
        let h = position.rem_euclid(1.0) * 6.0;
        let x = 1.0 - ((h % 2.0) - 1.0).abs();
        let (r, g, b) = match h as u32 {
            0 => (1.0, x, 0.0),
            1 => (x, 1.0, 0.0),
            2 => (0.0, 1.0, x),
            3 => (0.0, x, 1.0),
            4 => (x, 0.0, 1.0),
            _ => (1.0, 0.0, x),
        };
        [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
    }

    fn wave(seconds: f64, period: f64) -> Frame {
        (0..48)
            .map(|led| hue(led as f64 / 48.0 + seconds / period))
            .collect()
    }

    fn feed(
        recorder: &mut Recorder,
        origin: Instant,
        seconds: f64,
        mut frame: impl FnMut(f64) -> Frame,
    ) {
        for step in 0..(seconds * FPS) as u32 {
            let elapsed = f64::from(step) / FPS;
            let delivery = Duration::from_millis(u64::from(step * 7 % 9));
            recorder.push(
                origin + Duration::from_secs_f64(elapsed) + delivery,
                frame(elapsed),
            );
        }
    }

    #[test]
    fn locks_onto_a_fractional_period_and_predicts_the_stream() {
        let origin = Instant::now();
        let mut recorder = Recorder::new();
        feed(&mut recorder, origin, 9.0, |t| wave(t, 3.27));
        let cycle = recorder.detect().expect("periodic stream");
        assert!(
            (cycle.period.as_secs_f64() - 3.27).abs() < 0.01,
            "{:?}",
            cycle.period
        );
        assert_eq!(cycle.frames.len(), 65);
        for step in 0..120 {
            let elapsed = 9.0 + f64::from(step) / 7.0;
            let at = origin + Duration::from_secs_f64(elapsed);
            assert!(cycle.distance(at, &wave(elapsed, 3.27)) <= cycle.tolerance);
        }
        let changed = wave(9.0, 3.27)
            .iter()
            .map(|[r, g, b]| [*b, *r, *g])
            .collect::<Vec<_>>();
        assert!(cycle.distance(origin + Duration::from_secs(9), &changed) > 4.0 * cycle.tolerance);
    }

    #[test]
    fn ignores_streams_that_do_not_repeat() {
        let origin = Instant::now();
        let mut state = 0x2545_f491u32;
        let mut random = Recorder::new();
        feed(&mut random, origin, 25.0, |_| {
            (0..48)
                .map(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    state.to_be_bytes()[..3].try_into().unwrap()
                })
                .collect()
        });
        assert!(random.detect().is_none());

        let mut fixed = Recorder::new();
        feed(&mut fixed, origin, 25.0, |_| vec![[10, 20, 30]; 48].into());
        assert!(fixed.detect().is_none());

        let mut drifting = Recorder::new();
        feed(&mut drifting, origin, 25.0, |t| wave(t * t / 4.0, 30.0));
        assert!(drifting.detect().is_none());
    }

    #[test]
    fn a_paused_stream_starts_a_new_recording() {
        let origin = Instant::now();
        let mut recorder = Recorder::new();
        feed(&mut recorder, origin, 9.0, |t| wave(t, 2.0));
        assert!(recorder.detect().is_some());
        let resumed = origin + Duration::from_secs(10);
        feed(&mut recorder, resumed, 1.0, |t| wave(t, 2.0));
        assert!(recorder.detect().is_none());
    }

    #[test]
    fn aligned_loop_shows_its_first_frame_at_the_recorded_start() {
        let frames: Vec<Frame> = (0..40u8).map(|value| vec![[value; 3]].into()).collect();
        let sampled = Instant::now() + Duration::from_secs(5);
        let start = sampled - Duration::from_millis(1_234);
        let interval = 8_000;
        let ticks = 0x1234_5678u32;
        let aligned = phase_aligned(&frames, start, interval, (ticks, sampled));
        let start_ticks = u64::from(ticks) - 1_974;
        for frame in [0u64, 1, 17, 39, 40, 95] {
            let ticks_then = start_ticks + frame * u64::from(interval) / 100;
            let shown = ticks_then * 100 / u64::from(interval) % 40;
            assert_eq!(aligned[shown as usize][0][0], (frame % 40) as u8);
        }
    }
}

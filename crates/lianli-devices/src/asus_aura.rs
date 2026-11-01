//! ASUS Aura USB protocol support with explicit channel targeting.
//!
//! Lighting operations are opt-in commands. This module deliberately does
//! not implement a device-open initialization routine or claim all channels.

use anyhow::{bail, ensure, Context, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

pub const VENDOR_ID: u16 = 0x0B05;
pub const PRODUCT_ID: u16 = 0x19AF;
pub const REPORT_ID: u8 = 0xEC;
pub const FRAME_LENGTH: usize = 65;
pub const REPORT_TIMEOUT_MS: i32 = 1_000;
const CONFIG_TABLE_LENGTH: usize = 60;
const MAX_REPORTED_CHANNELS: usize = 9;
const MAX_CHANNEL_LEDS: usize = 120;
const LEDS_PER_DIRECT_FRAME: usize = 20;
const WINDOWS_RAINBOW_WAVELENGTH_LEDS: f64 = 22.0;
const WINDOWS_RAINBOW_FREQUENCY_HZ: f64 = 0.4;
const WINDOWS_RAINBOW_PHASE_DEGREES: f64 = 16.0;
const WINDOWS_RAINBOW_FRAME_INTERVAL: Duration = Duration::from_millis(80);
const MIN_RAINBOW_FRAME_INTERVAL: Duration = Duration::from_millis(40);
const MAX_RAINBOW_FRAME_INTERVAL: Duration = Duration::from_secs(1);
const CPU_WATERBLOCK_DIRECT_CHANNEL: u8 = 2;

pub struct AuraWriterLock {
    _device: File,
}

impl AuraWriterLock {
    pub fn acquire(hidraw_path: &Path) -> Result<Self> {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(hidraw_path)
            .with_context(|| format!("opening ASUS Aura writer lock {}", hidraw_path.display()))?;
        let result = unsafe { libc::flock(device.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        ensure!(
            result == 0,
            "ASUS Aura HID interface {} already has an application writer",
            hidraw_path.display()
        );
        Ok(Self { _device: device })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Query {
    Firmware,
    Configuration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectChannel {
    Mainboard,
    /// Zero-based ARGB header index, mapped to the protocol's 1-based effect channel.
    Argb(u8),
}

impl EffectChannel {
    pub fn protocol_number(self, reported_argb_channels: u8) -> Result<u8> {
        match self {
            Self::Mainboard => Ok(0),
            Self::Argb(index) => {
                ensure!(
                    index < reported_argb_channels,
                    "ARGB effect channel {index} is out of range (device reports {reported_argb_channels})"
                );
                index
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("effect channel overflow"))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectChannel {
    /// Zero-based ARGB direct-stream index.
    Argb(u8),
    Mainboard,
}

impl DirectChannel {
    pub fn protocol_number(self, reported_argb_channels: u8) -> Result<u8> {
        match self {
            Self::Argb(index) => {
                ensure!(
                    index < reported_argb_channels,
                    "ARGB direct channel {index} is out of range (device reports {reported_argb_channels})"
                );
                Ok(index)
            }
            Self::Mainboard => Ok(0x04),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuraEffect {
    Off,
    Static,
    Breathing,
    SpectrumCycle,
    RainbowWave,
}

impl AuraEffect {
    pub const fn mode_byte(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::Static => 0x01,
            Self::Breathing => 0x02,
            Self::SpectrumCycle => 0x04,
            Self::RainbowWave => 0x05,
        }
    }
}

/// Channel identifiers used by the controller's EC 35/EC 36 effect sequence.
/// These are distinct from the sequential channels used by the EC 3B protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceChannel {
    MainboardRgb,
    /// Zero-based ARGB header index.
    Argb(u8),
}

impl SequenceChannel {
    fn mapping(self, reported_argb_channels: u8) -> Result<(u8, u8, usize)> {
        match self {
            Self::MainboardRgb => Ok((0, 0x01, 0)),
            Self::Argb(index) => {
                ensure!(
                    index < reported_argb_channels && index < 3,
                    "ARGB sequence channel {index} is out of range (device reports {reported_argb_channels}; max supported index is 2)"
                );
                Ok((1, 1 << (index + 1), usize::from(index) + 1))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RainbowTarget {
    pub channel: DirectChannel,
    pub led_count: usize,
    /// Optional half-open LED range whose physical order is reversed in emitted frames.
    pub reversed_segment: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuraSyncRainbow {
    /// Starting values from the saved Rainbow script and observed USB update cadence. The HSV
    /// waveform below is an approximation because the captured stream does not expose the effect evaluator.
    pub wavelength_leds: f64,
    pub frequency_hz: f64,
    pub phase_degrees: f64,
    pub frame_interval: Duration,
}

impl Default for AuraSyncRainbow {
    fn default() -> Self {
        Self {
            wavelength_leds: WINDOWS_RAINBOW_WAVELENGTH_LEDS,
            frequency_hz: WINDOWS_RAINBOW_FREQUENCY_HZ,
            phase_degrees: WINDOWS_RAINBOW_PHASE_DEGREES,
            frame_interval: WINDOWS_RAINBOW_FRAME_INTERVAL,
        }
    }
}

impl AuraSyncRainbow {
    fn validate(self) -> Result<()> {
        ensure!(
            self.wavelength_leds.is_finite() && (1.0..=120.0).contains(&self.wavelength_leds),
            "Rainbow wavelength must be finite and within 1..=120 LEDs"
        );
        ensure!(
            self.frequency_hz.is_finite() && (0.01..=2.0).contains(&self.frequency_hz),
            "Rainbow frequency must be finite and within 0.01..=2.0 Hz"
        );
        ensure!(
            self.phase_degrees.is_finite(),
            "Rainbow phase must be finite"
        );
        ensure!(
            (MIN_RAINBOW_FRAME_INTERVAL..=MAX_RAINBOW_FRAME_INTERVAL)
                .contains(&self.frame_interval),
            "Rainbow frame interval must be between 40 ms and 1 second"
        );
        Ok(())
    }

    pub fn colors_at(self, elapsed: Duration, led_count: usize) -> Result<Vec<Rgb>> {
        self.validate()?;
        ensure!(
            (1..=MAX_CHANNEL_LEDS).contains(&led_count),
            "Rainbow LED count {led_count} is outside the supported range 1..={MAX_CHANNEL_LEDS}"
        );

        let phase = self.phase_degrees / 360.0 - elapsed.as_secs_f64() * self.frequency_hz;
        Ok((0..led_count)
            .map(|index| {
                let hue = (phase + index as f64 / self.wavelength_leds).rem_euclid(1.0) * 360.0;
                hsv_rainbow_color(hue)
            })
            .collect())
    }

    pub fn colors_for_target(self, elapsed: Duration, target: RainbowTarget) -> Result<Vec<Rgb>> {
        let mut colors = self.colors_at(elapsed, target.led_count)?;
        if let Some((start, end)) = target.reversed_segment {
            ensure!(
                start < end && end <= target.led_count,
                "reversed Rainbow segment {start}..{end} is outside target capacity {}",
                target.led_count
            );
            colors[start..end].reverse();
        }
        Ok(colors)
    }
}

fn hsv_rainbow_color(hue_degrees: f64) -> Rgb {
    let sector = hue_degrees.rem_euclid(360.0) / 60.0;
    let secondary = 255.0 * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector as u8 {
        0 => (255.0, secondary, 0.0),
        1 => (secondary, 255.0, 0.0),
        2 => (0.0, 255.0, secondary),
        3 => (0.0, secondary, 255.0),
        4 => (secondary, 0.0, 255.0),
        _ => (255.0, 0.0, secondary),
    };
    Rgb::new(red.round() as u8, green.round() as u8, blue.round() as u8)
}

impl Rgb {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

impl Query {
    pub const fn response_opcode(self) -> u8 {
        match self {
            Self::Firmware => 0x02,
            Self::Configuration => 0x30,
        }
    }

    pub const fn frame(self) -> [u8; FRAME_LENGTH] {
        let mut frame = [0; FRAME_LENGTH];
        frame[0] = REPORT_ID;
        frame[1] = match self {
            Self::Firmware => 0x82,
            Self::Configuration => 0xB0,
        };
        frame
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuraConfiguration {
    /// Raw channel count returned by the device.
    pub reported_channel_count: u8,
    /// Number of channel records that fit in the documented table.
    pub parsed_channel_count: usize,
    /// Raw values reported for each advertised ARGB channel. Zero is retained
    /// because reference software applies a fallback value of 30 separately.
    pub reported_led_counts: Vec<u8>,
    pub mainboard_led_count: u8,
    pub raw_table: [u8; CONFIG_TABLE_LENGTH],
}

impl AuraConfiguration {
    /// Return the protocol's usable capacity for a reported ARGB channel.
    /// Zero is the protocol's "unknown" value and uses its documented fallback.
    pub fn effective_led_capacity(&self, channel: usize) -> Result<usize> {
        ensure!(
            channel < self.reported_led_counts.len(),
            "ARGB channel {channel} is not present in the configuration"
        );
        let reported = usize::from(self.reported_led_counts[channel]);
        Ok(if reported == 0 {
            30
        } else {
            reported.min(MAX_CHANNEL_LEDS)
        })
    }

    pub fn rainbow_targets(&self) -> Result<Vec<RainbowTarget>> {
        self.rainbow_targets_with_reversed_channels(&[CPU_WATERBLOCK_DIRECT_CHANNEL])
    }

    pub fn rainbow_targets_with_reversed_channels(
        &self,
        reversed_channels: &[u8],
    ) -> Result<Vec<RainbowTarget>> {
        ensure!(
            reversed_channels
                .iter()
                .all(|channel| { usize::from(*channel) < self.reported_led_counts.len() }),
            "a reversed ARGB channel is not present in the Aura configuration"
        );
        let mut targets = Vec::with_capacity(self.reported_led_counts.len() + 1);
        for (index, _) in self.reported_led_counts.iter().enumerate() {
            let channel = index as u8;
            let led_count = self.effective_led_capacity(index)?;
            targets.push(RainbowTarget {
                channel: DirectChannel::Argb(channel),
                led_count,
                reversed_segment: reversed_channels
                    .contains(&channel)
                    .then_some((0, led_count)),
            });
        }
        let mainboard_led_count = usize::from(self.mainboard_led_count).min(MAX_CHANNEL_LEDS);
        if mainboard_led_count > 0 {
            targets.push(RainbowTarget {
                channel: DirectChannel::Mainboard,
                led_count: mainboard_led_count,
                reversed_segment: None,
            });
        }
        ensure!(!targets.is_empty(), "Aura configuration has no RGB targets");
        Ok(targets)
    }
}

pub type AuraFrame = [u8; FRAME_LENGTH];

pub fn native_effect_frame(
    channel: EffectChannel,
    reported_argb_channels: u8,
    effect: AuraEffect,
    color: Rgb,
) -> Result<AuraFrame> {
    let effect_channel = channel.protocol_number(reported_argb_channels)?;
    let mut frame = [0; FRAME_LENGTH];
    frame[..8].copy_from_slice(&[
        REPORT_ID,
        0x3B,
        effect_channel,
        0,
        effect.mode_byte(),
        color.red,
        color.green,
        color.blue,
    ]);
    Ok(frame)
}

/// Build the five-frame EC 35/36/3F effect sequence used by the 0B05:19AF
/// controller implementation documented by liquidctl. The EC 36 mask and RGB
/// slot offset are intentionally kept separate from the EC 3B channel number.
pub fn effect_sequence_frames(
    channel: SequenceChannel,
    reported_argb_channels: u8,
    effect: AuraEffect,
    color: Rgb,
) -> Result<[AuraFrame; 5]> {
    let (channel_type, channel_mask, rgb_offset) = channel.mapping(reported_argb_channels)?;
    let mut start = [0; FRAME_LENGTH];
    start[..6].copy_from_slice(&[REPORT_ID, 0x35, channel_type, 0, 0, effect.mode_byte()]);

    let mut color_data = [0; FRAME_LENGTH];
    color_data[..5].copy_from_slice(&[REPORT_ID, 0x36, 0, channel_mask, 0]);
    let rgb_start = 5 + rgb_offset * 3;
    if matches!(effect, AuraEffect::Static | AuraEffect::Breathing) {
        color_data[rgb_start..rgb_start + 3].copy_from_slice(&[color.red, color.green, color.blue]);
    }

    let mut commit_data = [0; FRAME_LENGTH];
    commit_data[..5].copy_from_slice(&[REPORT_ID, 0x3F, 0x55, 0, 0]);
    let mut finish_mode = [0; FRAME_LENGTH];
    finish_mode[..6].copy_from_slice(&[REPORT_ID, 0x35, 0, 0, 1, 0x05]);
    Ok([start, color_data, commit_data, finish_mode, commit_data])
}

/// Send one targeted EC 35/36/3F effect sequence. The controller does not
/// return an ACK; success means only that all five reports were written.
pub fn send_effect_sequence<T: lianli_transport::HidTransport>(
    transport: &mut T,
    channel: SequenceChannel,
    reported_argb_channels: u8,
    effect: AuraEffect,
    color: Rgb,
) -> Result<()> {
    let frames = effect_sequence_frames(channel, reported_argb_channels, effect, color)?;
    for (index, frame) in frames.iter().enumerate() {
        send_lighting_frame(transport, frame)
            .with_context(|| format!("sending Aura effect-sequence report {}/5", index + 1))?;
    }
    Ok(())
}

pub fn direct_mode_frame(channel: EffectChannel, reported_argb_channels: u8) -> Result<AuraFrame> {
    let effect_channel = channel.protocol_number(reported_argb_channels)?;
    let mut frame = [0; FRAME_LENGTH];
    frame[..6].copy_from_slice(&[REPORT_ID, 0x35, effect_channel, 0, 0, 0xFF]);
    Ok(frame)
}

/// Build direct RGB packets for one ARGB channel. The maximum comes from the
/// controller's configuration response, not a presumed physical LED count.
pub fn direct_stream_frames(
    channel: DirectChannel,
    reported_argb_channels: u8,
    colors: &[Rgb],
    channel_capacity: usize,
) -> Result<Vec<AuraFrame>> {
    let direct_channel = channel.protocol_number(reported_argb_channels)?;
    ensure!(
        !colors.is_empty(),
        "direct RGB stream must contain at least one LED"
    );
    ensure!(
        channel_capacity > 0 && channel_capacity <= MAX_CHANNEL_LEDS,
        "channel capacity {channel_capacity} is invalid; expected 1..={MAX_CHANNEL_LEDS}"
    );
    ensure!(
        colors.len() <= channel_capacity,
        "RGB stream has {} LEDs, exceeding channel capacity {channel_capacity}",
        colors.len()
    );
    ensure!(
        colors.len() <= u8::MAX as usize,
        "RGB stream exceeds the protocol's 255 LED offset limit"
    );

    let mut frames = Vec::with_capacity(colors.len().div_ceil(LEDS_PER_DIRECT_FRAME));
    for (chunk_index, chunk) in colors.chunks(LEDS_PER_DIRECT_FRAME).enumerate() {
        let offset = chunk_index * LEDS_PER_DIRECT_FRAME;
        let is_last = offset + chunk.len() == colors.len();
        let mut frame = [0; FRAME_LENGTH];
        frame[0] = REPORT_ID;
        frame[1] = 0x40;
        frame[2] = direct_channel | if is_last { 0x80 } else { 0 };
        frame[3] = offset as u8;
        frame[4] = chunk.len() as u8;
        for (index, rgb) in chunk.iter().enumerate() {
            let base = 5 + index * 3;
            frame[base..base + 3].copy_from_slice(&[rgb.red, rgb.green, rgb.blue]);
        }
        frames.push(frame);
    }
    Ok(frames)
}

/// Send exactly one already-validated 65-byte output report.
/// A successful USB write is not a hardware ACK or a readback of visible state.
fn send_lighting_frame<T: lianli_transport::HidTransport>(
    transport: &mut T,
    frame: &AuraFrame,
) -> Result<()> {
    let written = transport.write_timeout(frame, std::time::Duration::from_secs(1))?;
    ensure!(
        written == FRAME_LENGTH,
        "short Aura frame write: {written}/{FRAME_LENGTH}"
    );
    Ok(())
}

pub fn send_native_effect<T: lianli_transport::HidTransport>(
    transport: &mut T,
    channel: EffectChannel,
    reported_argb_channels: u8,
    effect: AuraEffect,
    color: Rgb,
) -> Result<()> {
    let frame = native_effect_frame(channel, reported_argb_channels, effect, color)?;
    send_lighting_frame(transport, &frame)
}

pub fn set_direct_control<T: lianli_transport::HidTransport>(
    transport: &mut T,
    channel: EffectChannel,
    reported_argb_channels: u8,
) -> Result<()> {
    let frame = direct_mode_frame(channel, reported_argb_channels)?;
    send_lighting_frame(transport, &frame)
}

pub fn stream_direct_channel<T: lianli_transport::HidTransport>(
    transport: &mut T,
    channel: DirectChannel,
    reported_argb_channels: u8,
    colors: &[Rgb],
    channel_capacity: usize,
) -> Result<usize> {
    let frames = direct_stream_frames(channel, reported_argb_channels, colors, channel_capacity)?;
    for (index, frame) in frames.iter().enumerate() {
        send_lighting_frame(transport, frame)
            .with_context(|| format!("sending direct RGB packet {}/{}", index + 1, frames.len()))?;
    }
    Ok(frames.len())
}

fn validate_rainbow_targets(
    configuration: &AuraConfiguration,
    targets: &[RainbowTarget],
) -> Result<()> {
    ensure!(
        !targets.is_empty(),
        "Rainbow stream requires at least one target"
    );
    for (index, target) in targets.iter().enumerate() {
        ensure!(
            !targets[..index]
                .iter()
                .any(|previous| previous.channel == target.channel),
            "Rainbow target {:?} is duplicated",
            target.channel
        );
        ensure!(
            (1..=MAX_CHANNEL_LEDS).contains(&target.led_count),
            "Rainbow target {:?} has invalid LED count {}",
            target.channel,
            target.led_count
        );
        if let Some((start, end)) = target.reversed_segment {
            ensure!(
                start < end && end <= target.led_count,
                "Rainbow target {:?} has invalid reversed segment {start}..{end}",
                target.channel
            );
        }
        match target.channel {
            DirectChannel::Argb(channel) => ensure!(
                target.led_count <= configuration.effective_led_capacity(channel as usize)?,
                "Rainbow target ARGB {channel} exceeds its reported LED capacity"
            ),
            DirectChannel::Mainboard => ensure!(
                target.led_count <= usize::from(configuration.mainboard_led_count),
                "Rainbow target exceeds the reported motherboard LED count"
            ),
        }
    }
    Ok(())
}

fn activate_direct_channels<T: lianli_transport::HidTransport>(
    transport: &mut T,
    reported_argb_channels: u8,
    targets: &[RainbowTarget],
) -> Result<()> {
    for target in targets {
        let channel = match target.channel {
            DirectChannel::Argb(index) => EffectChannel::Argb(index),
            DirectChannel::Mainboard => EffectChannel::Mainboard,
        };
        set_direct_control(transport, channel, reported_argb_channels)
            .with_context(|| format!("activating direct mode for {:?}", target.channel))?;
    }
    Ok(())
}

fn stream_rainbow_frame<T: lianli_transport::HidTransport>(
    transport: &mut T,
    configuration: &AuraConfiguration,
    targets: &[RainbowTarget],
    animation: AuraSyncRainbow,
    elapsed: Duration,
) -> Result<()> {
    for target in targets {
        let colors = animation.colors_for_target(elapsed, *target)?;
        stream_direct_channel(
            transport,
            target.channel,
            configuration.reported_channel_count,
            &colors,
            target.led_count,
        )
        .with_context(|| format!("streaming Rainbow to {:?}", target.channel))?;
    }
    Ok(())
}

/// Activate each selected channel once, then stream synchronized frames until the stop receiver
/// receives a message or disconnects. Stopping leaves the controller on its last accepted frame.
pub fn run_rainbow_stream<T: lianli_transport::HidTransport>(
    transport: &mut T,
    configuration: &AuraConfiguration,
    targets: &[RainbowTarget],
    animation: AuraSyncRainbow,
    stop: &Receiver<()>,
) -> Result<()> {
    run_rainbow_stream_observed(transport, configuration, targets, animation, stop, || {})
}

/// Run the synchronized direct stream and call `frame_accepted` after every complete target
/// group has been accepted by HID writes. This is write acceptance, not controller readback.
pub fn run_rainbow_stream_observed<T, F>(
    transport: &mut T,
    configuration: &AuraConfiguration,
    targets: &[RainbowTarget],
    animation: AuraSyncRainbow,
    stop: &Receiver<()>,
    mut frame_accepted: F,
) -> Result<()>
where
    T: lianli_transport::HidTransport,
    F: FnMut(),
{
    animation.validate()?;
    validate_rainbow_targets(configuration, targets)?;
    match stop.try_recv() {
        Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(()),
        Err(std::sync::mpsc::TryRecvError::Empty) => {}
    }

    activate_direct_channels(transport, configuration.reported_channel_count, targets)?;

    let started = Instant::now();
    let mut next_frame = started;
    loop {
        match stop.try_recv() {
            Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(()),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }

        stream_rainbow_frame(
            transport,
            configuration,
            targets,
            animation,
            started.elapsed(),
        )?;
        frame_accepted();

        next_frame += animation.frame_interval;
        let now = Instant::now();
        if next_frame <= now {
            next_frame = now + animation.frame_interval;
        }
        match stop.recv_timeout(next_frame.saturating_duration_since(Instant::now())) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

/// Resume the capture-derived Rainbow animation and keep it streaming until stopped.
pub fn restore_rainbow<T: lianli_transport::HidTransport>(
    transport: &mut T,
    configuration: &AuraConfiguration,
    targets: &[RainbowTarget],
    stop: Receiver<()>,
) -> Result<()> {
    run_rainbow_stream(
        transport,
        configuration,
        targets,
        AuraSyncRainbow::default(),
        &stop,
    )
}

/// Run a single-channel native-effect test and attempt the explicitly supplied
/// restore effect even if the observation callback returns an error.
pub fn test_effect_with_restore<T, F>(
    transport: &mut T,
    channel: EffectChannel,
    reported_argb_channels: u8,
    test_effect: AuraEffect,
    restore_effect: AuraEffect,
    color: Rgb,
    observe: F,
) -> Result<()>
where
    T: lianli_transport::HidTransport,
    F: FnOnce() -> Result<()>,
{
    send_lighting_frame(
        transport,
        &native_effect_frame(channel, reported_argb_channels, test_effect, color)?,
    )?;
    let observation = observe();
    let restoration = send_lighting_frame(
        transport,
        &native_effect_frame(channel, reported_argb_channels, restore_effect, color)?,
    );
    match (observation, restoration) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(observation), Ok(())) => Err(observation.context("lighting observation failed; restore command was sent")),
        (Ok(()), Err(restore)) => Err(restore.context("failed to send the requested Aura restore effect")),
        (Err(observation), Err(restore)) => Err(anyhow::anyhow!(
            "lighting observation failed ({observation:#}); restore command also failed ({restore:#})"
        )),
    }
}

pub fn validate_device_identity(vendor_id: u16, product_id: u16) -> Result<()> {
    ensure!(
        vendor_id == VENDOR_ID && product_id == PRODUCT_ID,
        "unexpected ASUS Aura device {:04x}:{:04x}; expected {:04x}:{:04x}",
        vendor_id,
        product_id,
        VENDOR_ID,
        PRODUCT_ID
    );
    Ok(())
}

pub fn validate_response(query: Query, raw: &[u8]) -> Result<()> {
    ensure!(
        raw.len() == FRAME_LENGTH,
        "response length is {}, expected {FRAME_LENGTH}",
        raw.len()
    );
    ensure!(
        raw[0] == REPORT_ID,
        "response header is 0x{:02x}, expected 0xec",
        raw[0]
    );
    ensure!(
        raw[1] == query.response_opcode(),
        "response opcode is 0x{:02x}, expected 0x{:02x}",
        raw[1],
        query.response_opcode()
    );
    Ok(())
}

pub fn parse_firmware(raw: &[u8]) -> Result<String> {
    validate_response(Query::Firmware, raw)?;
    let bytes = &raw[2..18];
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    ensure!(
        end > 0,
        "firmware response contains an empty version string"
    );
    ensure!(
        bytes[..end].iter().all(u8::is_ascii_graphic),
        "firmware version contains non-printable bytes"
    );
    Ok(std::str::from_utf8(&bytes[..end])?.to_owned())
}

pub fn parse_configuration(raw: &[u8]) -> Result<AuraConfiguration> {
    validate_response(Query::Configuration, raw)?;
    let table: [u8; CONFIG_TABLE_LENGTH] = raw[4..64]
        .try_into()
        .expect("validated response contains the complete config table");
    let reported_channel_count = table[0x02];
    let parsed_channel_count = usize::from(reported_channel_count).min(MAX_REPORTED_CHANNELS);
    let mut reported_led_counts = Vec::with_capacity(parsed_channel_count);
    for channel in 0..parsed_channel_count {
        let offset = 4 + channel * 6 + 2;
        reported_led_counts.push(table[offset]);
    }
    Ok(AuraConfiguration {
        reported_channel_count,
        parsed_channel_count,
        reported_led_counts,
        mainboard_led_count: table[0x1B],
        raw_table: table,
    })
}

/// Read one complete expected response, discarding at most seven unrelated
/// frames. Every individual read has the protocol's one-second timeout.
pub fn read_response<T: lianli_transport::HidTransport>(
    transport: &mut T,
    query: Query,
) -> Result<[u8; FRAME_LENGTH]> {
    for attempt in 1..=8 {
        let mut response = [0u8; FRAME_LENGTH];
        let length = transport.read_timeout(&mut response, REPORT_TIMEOUT_MS)?;
        if length == 0 {
            continue;
        }
        ensure!(
            length == FRAME_LENGTH,
            "short HID response on attempt {attempt}: {length}/{FRAME_LENGTH} bytes"
        );
        if response[0] == REPORT_ID && response[1] == query.response_opcode() {
            return Ok(response);
        }
    }
    bail!(
        "timed out waiting for ASUS Aura {:?} response after 8 reads",
        query
    );
}

/// Query the controller's reported target configuration without writing lighting settings.
pub fn query_configuration<T: lianli_transport::HidTransport>(
    transport: &mut T,
) -> Result<AuraConfiguration> {
    send_lighting_frame(transport, &Query::Configuration.frame())
        .context("sending read-only Aura configuration query")?;
    let response = read_response(transport, Query::Configuration)?;
    parse_configuration(&response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lianli_transport::{HidTransport, TransportError};

    fn response(query: Query) -> [u8; FRAME_LENGTH] {
        let mut frame = [0u8; FRAME_LENGTH];
        frame[0] = REPORT_ID;
        frame[1] = query.response_opcode();
        frame
    }

    #[test]
    fn aura_writer_lock_excludes_another_cooperating_process() {
        let path = std::env::temp_dir().join(format!(
            "lianli-aura-writer-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        File::create(&path).unwrap();
        let first = AuraWriterLock::acquire(&path).unwrap();
        assert!(AuraWriterLock::acquire(&path).is_err());
        drop(first);
        assert!(AuraWriterLock::acquire(&path).is_ok());
        let _ = std::fs::remove_file(path);
    }

    struct NoResponseTransport {
        reads: usize,
        writes: usize,
    }

    #[derive(Default)]
    struct RecordingTransport {
        writes: Vec<AuraFrame>,
        fail_write_number: Option<usize>,
    }

    impl HidTransport for RecordingTransport {
        fn write(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            if data.len() == FRAME_LENGTH {
                self.writes.push(data.try_into().unwrap());
            }
            Ok(data.len())
        }
        fn write_timeout(
            &mut self,
            data: &[u8],
            _timeout: std::time::Duration,
        ) -> Result<usize, TransportError> {
            if self.fail_write_number == Some(self.writes.len() + 1) {
                return Err(TransportError::Write("synthetic USB write failure".into()));
            }
            self.writes.push(data.try_into().unwrap());
            Ok(data.len())
        }
        fn read_timeout(
            &mut self,
            _buf: &mut [u8],
            _timeout_ms: i32,
        ) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn send_feature_report(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            self.write(data)
        }
        fn get_feature_report(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn get_input_report(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn read_flush(&mut self) {}
    }

    struct StopAfterWrites {
        writes: Vec<AuraFrame>,
        stop_after: usize,
        stop: std::sync::mpsc::Sender<()>,
    }

    impl HidTransport for StopAfterWrites {
        fn write(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            self.write_timeout(data, Duration::from_secs(1))
        }
        fn write_timeout(
            &mut self,
            data: &[u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let frame: AuraFrame = data.try_into().unwrap();
            self.writes.push(frame);
            if self.writes.len() == self.stop_after {
                let _ = self.stop.send(());
            }
            Ok(data.len())
        }
        fn read_timeout(
            &mut self,
            _buf: &mut [u8],
            _timeout_ms: i32,
        ) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn send_feature_report(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            self.write(data)
        }
        fn get_feature_report(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn get_input_report(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn read_flush(&mut self) {}
    }

    fn rainbow_test_configuration() -> AuraConfiguration {
        AuraConfiguration {
            reported_channel_count: 3,
            parsed_channel_count: 3,
            reported_led_counts: vec![120; 3],
            mainboard_led_count: 4,
            raw_table: [0; CONFIG_TABLE_LENGTH],
        }
    }

    impl HidTransport for NoResponseTransport {
        fn write(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            self.writes += 1;
            Ok(data.len())
        }
        fn read_timeout(
            &mut self,
            _buf: &mut [u8],
            timeout_ms: i32,
        ) -> Result<usize, TransportError> {
            assert_eq!(timeout_ms, REPORT_TIMEOUT_MS);
            self.reads += 1;
            Ok(0)
        }
        fn send_feature_report(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            self.writes += 1;
            Ok(data.len())
        }
        fn get_feature_report(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn get_input_report(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
        fn read_flush(&mut self) {}
    }

    #[test]
    fn firmware_response_is_decoded() {
        let mut raw = response(Query::Firmware);
        raw[2..7].copy_from_slice(b"1.2.3");
        assert_eq!(parse_firmware(&raw).unwrap(), "1.2.3");
    }

    #[test]
    fn invalid_firmware_response_is_rejected() {
        let mut raw = response(Query::Firmware);
        raw[1] = 0x30;
        assert!(parse_firmware(&raw).is_err());
        assert!(parse_firmware(&raw[..64]).is_err());
    }

    #[test]
    fn configuration_parses_channel_and_led_counts_without_defaults() {
        let mut raw = response(Query::Configuration);
        raw[4 + 2] = 2;
        raw[4 + 4 + 2] = 24;
        raw[4 + 10 + 2] = 0;
        raw[4 + 0x1B] = 12;
        let parsed = parse_configuration(&raw).unwrap();
        assert_eq!(parsed.reported_channel_count, 2);
        assert_eq!(parsed.parsed_channel_count, 2);
        assert_eq!(parsed.reported_led_counts, [24, 0]);
        assert_eq!(parsed.mainboard_led_count, 12);
    }

    #[test]
    fn channel_count_is_limited_to_records_that_fit_in_the_table() {
        let mut raw = response(Query::Configuration);
        raw[6] = 255;
        let parsed = parse_configuration(&raw).unwrap();
        assert_eq!(parsed.reported_channel_count, 255);
        assert_eq!(parsed.parsed_channel_count, 9);
        assert_eq!(parsed.reported_led_counts.len(), 9);
    }

    #[test]
    fn invalid_or_truncated_configuration_is_rejected() {
        let raw = response(Query::Configuration);
        assert!(parse_configuration(&raw[..64]).is_err());
        let mut wrong = raw;
        wrong[1] = 0x02;
        assert!(parse_configuration(&wrong).is_err());
    }

    #[test]
    fn identity_check_rejects_other_devices() {
        assert!(validate_device_identity(VENDOR_ID, PRODUCT_ID).is_ok());
        assert!(validate_device_identity(0x0B05, 0x19AE).is_err());
        assert!(validate_device_identity(0x1CBE, 0x0006).is_err());
    }

    #[test]
    fn query_builder_only_emits_read_only_opcodes() {
        assert_eq!(Query::Firmware.frame()[..2], [0xEC, 0x82]);
        assert_eq!(Query::Configuration.frame()[..2], [0xEC, 0xB0]);
        assert!(Query::Firmware.frame()[2..].iter().all(|byte| *byte == 0));
        assert!(Query::Configuration.frame()[2..]
            .iter()
            .all(|byte| *byte == 0));
        assert_eq!(Query::Firmware.frame().len(), FRAME_LENGTH);
    }

    #[test]
    fn response_wait_is_bounded_after_eight_timeouts() {
        let mut transport = NoResponseTransport {
            reads: 0,
            writes: 0,
        };
        assert!(read_response(&mut transport, Query::Firmware).is_err());
        assert_eq!(transport.reads, 8);
        assert_eq!(transport.writes, 0);
    }

    #[test]
    fn native_effect_uses_effect_channel_mapping_and_rgb_order() {
        let frame = native_effect_frame(
            EffectChannel::Argb(2),
            3,
            AuraEffect::Breathing,
            Rgb::new(0x11, 0x22, 0x33),
        )
        .unwrap();
        assert_eq!(&frame[..8], &[0xEC, 0x3B, 3, 0, 0x02, 0x11, 0x22, 0x33]);
        assert!(frame[8..].iter().all(|byte| *byte == 0));
        assert_eq!(
            native_effect_frame(
                EffectChannel::Mainboard,
                3,
                AuraEffect::Off,
                Rgb::new(0, 0, 0)
            )
            .unwrap()[2],
            0
        );
    }

    #[test]
    fn effect_sequence_encodes_header_zero_mask_rgb_offset_and_finalization() {
        let frames = effect_sequence_frames(
            SequenceChannel::Argb(0),
            3,
            AuraEffect::Static,
            Rgb::new(0xF1, 0x02, 0xA3),
        )
        .unwrap();
        assert_eq!(&frames[0][..6], &[0xEC, 0x35, 1, 0, 0, 1]);
        assert_eq!(&frames[1][..5], &[0xEC, 0x36, 0, 0x02, 0]);
        assert_eq!(&frames[1][5..8], &[0, 0, 0]);
        assert_eq!(&frames[1][8..11], &[0xF1, 0x02, 0xA3]);
        assert_eq!(&frames[2][..5], &[0xEC, 0x3F, 0x55, 0, 0]);
        assert_eq!(&frames[3][..6], &[0xEC, 0x35, 0, 0, 1, 5]);
        assert_eq!(&frames[4][..5], &[0xEC, 0x3F, 0x55, 0, 0]);
        assert!(frames.iter().all(|frame| frame.len() == FRAME_LENGTH));
    }

    #[test]
    fn effect_sequence_uses_distinct_masks_and_rgb_offsets_for_each_argb_header() {
        let expected = [(1, 0x02, 1), (2, 0x04, 2), (3, 0x08, 3)];
        for (index, mask, offset) in expected {
            let frames = effect_sequence_frames(
                SequenceChannel::Argb(index - 1),
                3,
                AuraEffect::Static,
                Rgb::new(1, 2, 3),
            )
            .unwrap();
            assert_eq!(frames[0][2], 1); // ARGB bus type, not effect channel number
            assert_eq!(frames[1][3], mask);
            assert_eq!(&frames[1][5..5 + offset * 3], vec![0; offset * 3]);
            assert_eq!(&frames[1][5 + offset * 3..8 + offset * 3], &[1, 2, 3]);
        }
        assert_eq!(
            effect_sequence_frames(
                SequenceChannel::MainboardRgb,
                3,
                AuraEffect::Static,
                Rgb::new(1, 2, 3)
            )
            .unwrap()[0][2],
            0
        );
    }

    #[test]
    fn effect_sequence_rejects_unreported_argb_channels() {
        assert!(effect_sequence_frames(
            SequenceChannel::Argb(0),
            0,
            AuraEffect::Static,
            Rgb::new(255, 0, 255)
        )
        .is_err());
        assert!(effect_sequence_frames(
            SequenceChannel::Argb(3),
            4,
            AuraEffect::Static,
            Rgb::new(255, 0, 255)
        )
        .is_err());
    }

    #[test]
    fn effect_sequence_write_failure_stops_before_later_reports() {
        let mut transport = RecordingTransport {
            writes: Vec::new(),
            fail_write_number: Some(3),
        };
        assert!(send_effect_sequence(
            &mut transport,
            SequenceChannel::Argb(0),
            3,
            AuraEffect::Static,
            Rgb::new(255, 0, 255)
        )
        .is_err());
        assert_eq!(transport.writes.len(), 2);
        assert_eq!(transport.writes[0][1], 0x35);
        assert_eq!(transport.writes[1][1], 0x36);
    }

    #[test]
    fn direct_mode_uses_effect_channel_numbers_not_direct_channel_numbers() {
        let argb_zero = direct_mode_frame(EffectChannel::Argb(0), 3).unwrap();
        let argb = direct_mode_frame(EffectChannel::Argb(2), 3).unwrap();
        let mainboard = direct_mode_frame(EffectChannel::Mainboard, 3).unwrap();
        assert_eq!(&argb_zero[..6], &[0xEC, 0x35, 1, 0, 0, 0xFF]);
        assert_eq!(&argb[..6], &[0xEC, 0x35, 3, 0, 0, 0xFF]);
        assert_eq!(&mainboard[..6], &[0xEC, 0x35, 0, 0, 0, 0xFF]);
        assert_eq!(DirectChannel::Argb(2).protocol_number(3).unwrap(), 2);
        assert_eq!(DirectChannel::Mainboard.protocol_number(3).unwrap(), 4);
        assert!(direct_mode_frame(EffectChannel::Argb(3), 3).is_err());
    }

    #[test]
    fn direct_rgb_stream_chunks_and_marks_only_final_packet_for_apply() {
        let colors: Vec<_> = (0..21).map(|n| Rgb::new(n, n + 1, n + 2)).collect();
        let frames = direct_stream_frames(DirectChannel::Argb(1), 3, &colors, 120).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(&frames[0][..5], &[0xEC, 0x40, 1, 0, 20]);
        assert_eq!(&frames[0][5..8], &[0, 1, 2]);
        assert_eq!(frames[0][2] & 0x80, 0);
        assert_eq!(&frames[1][..5], &[0xEC, 0x40, 0x81, 20, 1]);
        assert_eq!(&frames[1][5..8], &[20, 21, 22]);
    }

    #[test]
    fn argb_zero_direct_magenta_uses_direct_channel_zero_and_final_apply_bit() {
        let magenta = Rgb::new(255, 0, 255);
        let frames = direct_stream_frames(DirectChannel::Argb(0), 3, &[magenta; 20], 120).unwrap();

        assert_eq!(frames.len(), 1);
        assert_eq!(&frames[0][..5], &[0xEC, 0x40, 0x80, 0, 20]);
        assert_eq!(&frames[0][5..8], &[255, 0, 255]);
        assert_eq!(&frames[0][62..65], &[255, 0, 255]);
    }

    #[test]
    fn direct_stream_rejects_invalid_channels_and_excessive_led_counts() {
        assert!(
            direct_stream_frames(DirectChannel::Argb(3), 3, &[Rgb::new(1, 2, 3)], 120).is_err()
        );
        assert!(
            direct_stream_frames(DirectChannel::Argb(0), 3, &[Rgb::new(1, 2, 3); 31], 30).is_err()
        );
        assert!(
            direct_stream_frames(DirectChannel::Argb(0), 3, &[Rgb::new(1, 2, 3); 121], 120)
                .is_err()
        );
        assert!(direct_stream_frames(DirectChannel::Argb(0), 3, &[], 120).is_err());
        assert!(
            direct_stream_frames(DirectChannel::Argb(0), 3, &[Rgb::new(1, 2, 3)], 121).is_err()
        );
    }

    #[test]
    fn capture_based_rainbow_profile_generates_rgb_wave_for_variable_led_counts() {
        let animation = AuraSyncRainbow::default();
        let one_wave = animation.colors_at(Duration::ZERO, 44).unwrap();
        let moved_wave = animation
            .colors_at(Duration::from_millis(250), 120)
            .unwrap();

        assert_eq!(animation.wavelength_leds, 22.0);
        assert_eq!(animation.frequency_hz, 0.4);
        assert_eq!(animation.frame_interval, Duration::from_millis(80));
        assert_eq!(one_wave.len(), 44);
        assert_eq!(one_wave[0], one_wave[22]);
        assert_eq!(one_wave[0], Rgb::new(255, 68, 0));
        assert_eq!(moved_wave.len(), 120);
        assert_eq!(moved_wave[0], Rgb::new(255, 0, 85));
    }

    #[test]
    fn rainbow_mapping_reverses_only_the_selected_led_segment() {
        let animation = AuraSyncRainbow::default();
        let target = RainbowTarget {
            channel: DirectChannel::Argb(1),
            led_count: 12,
            reversed_segment: Some((3, 8)),
        };
        let original = animation
            .colors_at(Duration::ZERO, target.led_count)
            .unwrap();
        let mapped = animation.colors_for_target(Duration::ZERO, target).unwrap();

        assert_eq!(&mapped[..3], &original[..3]);
        assert_eq!(
            &mapped[3..8],
            &original[3..8].iter().rev().copied().collect::<Vec<_>>()
        );
        assert_eq!(&mapped[8..], &original[8..]);
        assert!(animation
            .colors_for_target(
                Duration::ZERO,
                RainbowTarget {
                    reversed_segment: Some((8, 13)),
                    ..target
                }
            )
            .is_err());
    }

    #[test]
    fn rainbow_targets_follow_reported_argb_and_mainboard_led_counts() {
        let mut configuration = rainbow_test_configuration();
        configuration.reported_led_counts = vec![120, 30, 0];
        configuration.mainboard_led_count = 4;
        let targets = configuration.rainbow_targets().unwrap();
        assert_eq!(
            targets,
            [
                RainbowTarget {
                    channel: DirectChannel::Argb(0),
                    led_count: 120,
                    reversed_segment: None,
                },
                RainbowTarget {
                    channel: DirectChannel::Argb(1),
                    led_count: 30,
                    reversed_segment: None,
                },
                RainbowTarget {
                    channel: DirectChannel::Argb(2),
                    led_count: 30,
                    reversed_segment: Some((0, 30)),
                },
                RainbowTarget {
                    channel: DirectChannel::Mainboard,
                    led_count: 4,
                    reversed_segment: None,
                },
            ]
        );
    }

    #[test]
    fn rainbow_targets_follow_saved_channel_direction_selection() {
        let mut configuration = rainbow_test_configuration();
        configuration.reported_led_counts = vec![12, 24, 30];
        let targets = configuration
            .rainbow_targets_with_reversed_channels(&[1])
            .unwrap();
        assert_eq!(targets[0].reversed_segment, None);
        assert_eq!(targets[1].reversed_segment, Some((0, 24)));
        assert_eq!(targets[2].reversed_segment, None);
        assert!(configuration
            .rainbow_targets_with_reversed_channels(&[3])
            .is_err());
    }

    #[test]
    fn rainbow_stream_initializes_targets_once_and_stops_after_complete_frame() {
        let configuration = rainbow_test_configuration();
        let targets = [
            RainbowTarget {
                channel: DirectChannel::Argb(0),
                led_count: 21,
                reversed_segment: None,
            },
            RainbowTarget {
                channel: DirectChannel::Argb(1),
                led_count: 1,
                reversed_segment: None,
            },
            RainbowTarget {
                channel: DirectChannel::Mainboard,
                led_count: 4,
                reversed_segment: None,
            },
        ];
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();
        let mut transport = StopAfterWrites {
            writes: Vec::new(),
            stop_after: 7,
            stop: stop_tx,
        };

        run_rainbow_stream(
            &mut transport,
            &configuration,
            &targets,
            AuraSyncRainbow::default(),
            &stop_rx,
        )
        .unwrap();

        assert_eq!(transport.writes.len(), 7);
        assert_eq!(
            transport.writes[..3]
                .iter()
                .map(|frame| frame[..2].to_vec())
                .collect::<Vec<_>>(),
            [
                vec![REPORT_ID, 0x35],
                vec![REPORT_ID, 0x35],
                vec![REPORT_ID, 0x35]
            ]
        );
        assert_eq!(
            transport.writes[..3]
                .iter()
                .map(|frame| frame[2])
                .collect::<Vec<_>>(),
            [1, 2, 0]
        );
        assert_eq!(
            transport.writes[3..]
                .iter()
                .map(|frame| (frame[2], frame[3], frame[4]))
                .collect::<Vec<_>>(),
            [(0, 0, 20), (0x80, 20, 1), (0x81, 0, 1), (0x84, 0, 4)]
        );
        assert!(transport.writes[3..].iter().all(|frame| frame[1] == 0x40));
    }

    #[test]
    fn rainbow_stream_rejects_led_counts_above_reported_channel_capacity() {
        let configuration = rainbow_test_configuration();
        let targets = [RainbowTarget {
            channel: DirectChannel::Argb(1),
            led_count: 121,
            reversed_segment: None,
        }];
        assert!(validate_rainbow_targets(&configuration, &targets).is_err());
    }

    #[test]
    fn direct_stream_write_failure_stops_without_sending_later_channels() {
        let mut transport = RecordingTransport {
            writes: Vec::new(),
            fail_write_number: Some(2),
        };
        let colors = vec![Rgb::new(1, 2, 3); 21];
        assert!(
            stream_direct_channel(&mut transport, DirectChannel::Argb(1), 3, &colors, 120).is_err()
        );
        assert_eq!(transport.writes.len(), 1);
        assert_eq!(transport.writes[0][2], 1);
    }

    #[test]
    fn effect_test_always_attempts_explicit_restore_after_observation_error() {
        let mut transport = RecordingTransport::default();
        let result = test_effect_with_restore(
            &mut transport,
            EffectChannel::Argb(0),
            3,
            AuraEffect::SpectrumCycle,
            AuraEffect::Breathing,
            Rgb::new(0x20, 0x30, 0x40),
            || bail!("visual observation failed"),
        );
        assert!(result.is_err());
        assert_eq!(transport.writes.len(), 2);
        assert_eq!(&transport.writes[0][..5], &[0xEC, 0x3B, 1, 0, 0x04]);
        assert_eq!(&transport.writes[1][..5], &[0xEC, 0x3B, 1, 0, 0x02]);
    }

    #[test]
    fn failed_restore_command_is_reported() {
        let mut transport = RecordingTransport {
            writes: Vec::new(),
            fail_write_number: Some(2),
        };
        let result = test_effect_with_restore(
            &mut transport,
            EffectChannel::Argb(1),
            3,
            AuraEffect::SpectrumCycle,
            AuraEffect::Off,
            Rgb::new(1, 2, 3),
            || Ok(()),
        );
        assert!(result.is_err());
        assert_eq!(transport.writes.len(), 1);
    }

    #[test]
    fn write_failure_is_reported_without_false_success() {
        let mut transport = RecordingTransport {
            writes: Vec::new(),
            fail_write_number: Some(1),
        };
        assert!(send_native_effect(
            &mut transport,
            EffectChannel::Argb(1),
            3,
            AuraEffect::RainbowWave,
            Rgb::new(255, 0, 0)
        )
        .is_err());
        assert!(transport.writes.is_empty());
    }
}

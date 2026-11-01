use anyhow::{ensure, Context, Result};
use clap::{Parser, ValueEnum};
use hidapi::{DeviceInfo, HidApi};
use lianli_devices::asus_aura::{
    parse_configuration, read_response, restore_rainbow, send_effect_sequence, set_direct_control,
    stream_direct_channel, AuraEffect, AuraWriterLock, DirectChannel, EffectChannel, Query, Rgb,
    SequenceChannel, FRAME_LENGTH, PRODUCT_ID, VENDOR_ID,
};
use lianli_transport::{HidTransport, HidrawTransport};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const AURA_USAGE_PAGE: u16 = 0xFF72;
const DEFAULT_TEST_EFFECT_SECONDS: u64 = 10;
const MAX_TEST_EFFECT_SECONDS: u64 = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EffectArg {
    Off,
    Static,
    Breathing,
    SpectrumCycle,
    RainbowWave,
    Rainbow,
    DirectMagenta,
}

impl EffectArg {
    fn native_effect(self) -> Option<AuraEffect> {
        match self {
            Self::Off => Some(AuraEffect::Off),
            Self::Static | Self::DirectMagenta => Some(AuraEffect::Static),
            Self::Breathing => Some(AuraEffect::Breathing),
            Self::SpectrumCycle => Some(AuraEffect::SpectrumCycle),
            Self::RainbowWave => Some(AuraEffect::RainbowWave),
            Self::Rainbow => None,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    about = "Run one bounded ASUS Aura ARGB identification test and restore Windows AuraSync Rainbow"
)]
struct Args {
    /// Start the continuous Windows AuraSync Rainbow stream on all configured Aura targets.
    #[arg(long, conflicts_with_all = ["identify_channel", "test_effect"])]
    rainbow_stream: bool,
    /// Reverse the entire selected ARGB channel while streaming (diagnostic/override).
    #[arg(long, requires = "rainbow_stream")]
    reverse_argb_channel: Option<u8>,
    /// Reverse only this half-open LED range on --reverse-argb-channel, e.g. 24:48.
    #[arg(long, value_parser = parse_led_segment, requires = "reverse_argb_channel")]
    reverse_led_segment: Option<(usize, usize)>,
    /// Zero-based ARGB header index (0..2). Never an effect-channel number.
    #[arg(long)]
    identify_channel: Option<u8>,
    /// Effect to apply for the bounded visible test.
    #[arg(long, value_enum)]
    test_effect: Option<EffectArg>,
    /// Seconds to hold the test effect before restoring; bounded to keep the test brief.
    #[arg(long, default_value_t = DEFAULT_TEST_EFFECT_SECONDS)]
    duration_seconds: u64,
    /// Effect to restore; defaults to the captured Windows AuraSync Rainbow stream.
    #[arg(long, value_enum, default_value = "rainbow")]
    restore_effect: Option<EffectArg>,
    #[arg(long, hide = true)]
    recovery_guard: bool,
    #[arg(long, hide = true)]
    watchdog_channel: Option<u8>,
    #[arg(long, value_enum, hide = true)]
    watchdog_effect: Option<EffectArg>,
}

fn parse_led_segment(value: &str) -> std::result::Result<(usize, usize), String> {
    let (start, end) = value
        .split_once(':')
        .ok_or_else(|| "expected a half-open LED range START:END".to_owned())?;
    let start = start
        .parse::<usize>()
        .map_err(|_| "LED range start must be an integer".to_owned())?;
    let end = end
        .parse::<usize>()
        .map_err(|_| "LED range end must be an integer".to_owned())?;
    if start >= end {
        return Err("LED range must have START < END".to_owned());
    }
    Ok((start, end))
}

fn select_interface<'a>(devices: &[&'a DeviceInfo]) -> Result<&'a DeviceInfo> {
    let matches: Vec<_> = devices
        .iter()
        .copied()
        .filter(|info| info.usage_page() == AURA_USAGE_PAGE)
        .collect();
    ensure!(
        matches.len() == 1,
        "expected one ASUS Aura HID interface with usage page 0x{AURA_USAGE_PAGE:04x}, found {}",
        matches.len()
    );
    Ok(matches[0])
}

fn open_controller(
    api: &HidApi,
    acquire_writer_lock: bool,
) -> Result<(HidrawTransport, Option<AuraWriterLock>)> {
    let candidates: Vec<_> = api
        .device_list()
        .filter(|info| info.vendor_id() == VENDOR_ID && info.product_id() == PRODUCT_ID)
        .collect();
    ensure!(
        !candidates.is_empty(),
        "ASUS Aura device {VENDOR_ID:04x}:{PRODUCT_ID:04x} not found"
    );
    let info = select_interface(&candidates)?;
    let path = info.path();
    let writer_lock = acquire_writer_lock
        .then(|| AuraWriterLock::acquire(std::path::Path::new(path.to_str()?)))
        .transpose()?;
    let path_os = OsStr::from_bytes(path.to_bytes()).to_os_string();
    eprintln!(
        "ASUS Aura {:04x}:{:04x}, interface {}, usage page 0x{:04x}, path {}",
        info.vendor_id(),
        info.product_id(),
        info.interface_number(),
        info.usage_page(),
        path.to_string_lossy()
    );
    let device = api
        .open_path(path)
        .context("opening ASUS Aura interface as normal user")?;
    Ok((HidrawTransport::new(device, Some(path_os)), writer_lock))
}

fn query_config<T: HidTransport>(
    transport: &mut T,
) -> Result<lianli_devices::asus_aura::AuraConfiguration> {
    let frame = Query::Configuration.frame();
    let written = transport.write_timeout(&frame, Duration::from_secs(1))?;
    ensure!(
        written == FRAME_LENGTH,
        "short configuration inquiry write {written}/{FRAME_LENGTH}"
    );
    let response = read_response(transport, Query::Configuration)?;
    parse_configuration(&response)
}

struct RecoveryGuard {
    child: Child,
    stdin: ChildStdin,
}

impl RecoveryGuard {
    fn arm(channel: u8, effect: EffectArg) -> Result<Self> {
        let executable = std::env::current_exe().context("resolving probe executable path")?;
        let mut child = Command::new(executable)
            .arg("--recovery-guard")
            .arg("--watchdog-channel")
            .arg(channel.to_string())
            .arg("--watchdog-effect")
            .arg(effect.to_possible_value().unwrap().get_name())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("starting bounded Aura lighting recovery guard")?;
        let stdin = child
            .stdin
            .take()
            .context("recovery guard stdin unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("recovery guard stdout unavailable")?;
        let mut reader = BufReader::new(stdout);
        let mut ready = String::new();
        reader
            .read_line(&mut ready)
            .context("waiting for recovery guard readiness")?;
        ensure!(
            ready.trim() == "READY",
            "recovery guard did not become ready: {}",
            ready.trim()
        );
        println!("Independent lighting recovery guard armed until this test exits");
        Ok(Self { child, stdin })
    }

    fn cancel(mut self) -> Result<()> {
        self.stdin.write_all(b"CANCEL\n")?;
        self.stdin.flush()?;
        let status = self
            .child
            .wait()
            .context("waiting for canceled recovery guard")?;
        ensure!(
            status.success(),
            "recovery guard exited with status {status}"
        );
        Ok(())
    }
}

fn run_recovery_guard(channel: u8, effect: EffectArg) -> Result<()> {
    let api = HidApi::new().context("initializing HIDAPI in recovery guard")?;
    let (mut transport, _writer_lock) = open_controller(&api, false)?;
    let config = query_config(&mut transport)?;
    ensure!(
        channel < config.reported_channel_count,
        "watchdog ARGB channel {channel} is not reported by the controller"
    );
    println!("READY");
    std::io::stdout().flush()?;
    let (cancel_tx, cancel_rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_ok() && line.trim() == "CANCEL" {
            let _ = cancel_tx.send(());
        }
        // EOF means the parent disappeared; leave the watchdog armed.
    });
    if cancel_rx.recv().is_ok() {
        return Ok(());
    }
    if effect == EffectArg::Rainbow {
        eprintln!("Recovery guard starting the Windows AuraSync Rainbow stream");
        let targets = config.rainbow_targets()?;
        let (_keep_alive, stop) = mpsc::channel();
        return restore_rainbow(&mut transport, &config, &targets, stop)
            .context("watchdog Rainbow stream failed");
    }
    eprintln!("Recovery guard restoring the selected effect on ARGB header {channel}");
    send_effect_sequence(
        &mut transport,
        SequenceChannel::Argb(channel),
        config.reported_channel_count,
        effect
            .native_effect()
            .context("Rainbow restoration must use the continuous stream")?,
        Rgb::new(255, 0, 255),
    )
    .context("watchdog restore sequence failed")
}

fn run_identification_test(
    channel: u8,
    test_effect: EffectArg,
    restore_effect: EffectArg,
    duration_seconds: u64,
) -> Result<()> {
    ensure!(channel < 3, "ARGB channel index must be 0, 1, or 2");
    ensure!(
        test_effect != EffectArg::Rainbow,
        "Windows AuraSync Rainbow is a continuous restore mode, not a bounded identification effect"
    );
    ensure!(
        (1..=MAX_TEST_EFFECT_SECONDS).contains(&duration_seconds),
        "test duration must be between 1 and {MAX_TEST_EFFECT_SECONDS} seconds"
    );
    let api = HidApi::new().context("initializing HIDAPI")?;
    let (mut transport, _writer_lock) = open_controller(&api, true)?;
    let config = query_config(&mut transport)?;
    ensure!(
        channel < config.reported_channel_count,
        "ARGB channel {channel} is not reported by controller"
    );
    let capacity = config.effective_led_capacity(channel as usize)?;
    println!(
        "Configuration: {} ARGB channels; channel {channel} capacity {capacity} LEDs",
        config.reported_channel_count
    );
    println!(
        "Aura sequence target: ARGB header index {channel}; channel mask 0x{:02x}; RGB slot offset {}",
        1u8 << (channel + 1),
        channel + 1
    );

    // The guard opens the same VID/PID+usage-page interface independently and
    // writes the agreed restore effect if this process is killed mid-test.
    let guard = RecoveryGuard::arm(channel, restore_effect)?;
    let test_color = if matches!(test_effect, EffectArg::Static | EffectArg::DirectMagenta) {
        Rgb::new(255, 0, 255)
    } else {
        Rgb::new(255, 255, 255)
    };
    println!(
        "TEST START: ARGB header {channel}, {test_effect:?}, RGB {:02X}-{:02X}-{:02X}; next observe the hardware",
        test_color.red, test_color.green, test_color.blue
    );
    let direct_led_count = capacity.min(20);
    if test_effect == EffectArg::DirectMagenta {
        println!("Direct stream target: ARGB header {channel}; effect channel {}; direct channel {channel}; first {direct_led_count} LEDs", channel + 1);
        set_direct_control(
            &mut transport,
            EffectChannel::Argb(channel),
            config.reported_channel_count,
        )
        .context("selecting targeted direct-control mode")?;
        let colors = vec![test_color; direct_led_count];
        let packet_count = stream_direct_channel(
            &mut transport,
            DirectChannel::Argb(channel),
            config.reported_channel_count,
            &colors,
            capacity,
        )
        .context("streaming targeted direct-magenta RGB data")?;
        println!("DIRECT RGB STREAM ACCEPTED: {packet_count} packet(s), {direct_led_count} LEDs; USB write acceptance is not a lighting acknowledgement");
    } else {
        send_effect_sequence(
            &mut transport,
            SequenceChannel::Argb(channel),
            config.reported_channel_count,
            test_effect
                .native_effect()
                .context("selected test effect has no native Aura command")?,
            test_color,
        )
        .with_context(|| format!("sending targeted {test_effect:?} effect sequence"))?;
        println!("EFFECT SEQUENCE ACCEPTED: five reports; USB write acceptance is not a lighting acknowledgement");
    }
    println!("EFFECT HOLD START: observe whether ARGB header {channel} turns magenta for {duration_seconds}s");
    thread::sleep(Duration::from_secs(duration_seconds));
    if restore_effect == EffectArg::Rainbow {
        println!("EFFECT HOLD END: restoring the captured Windows AuraSync Rainbow stream across configured Aura channels");
        println!("Rainbow will stream continuously at the capture-derived cadence; press Enter to stop this test utility. Stopping leaves the last frame on the controller.");
        let targets = config.rainbow_targets()?;
        let (stop_tx, stop_rx) = mpsc::channel();
        let input_stop = stop_tx.clone();
        let input = thread::spawn(move || {
            let mut line = String::new();
            let _ = std::io::stdin().lock().read_line(&mut line);
            if !line.is_empty() {
                let _ = input_stop.send(());
            }
        });
        restore_rainbow(&mut transport, &config, &targets, stop_rx)
            .context("restoring continuous Windows AuraSync Rainbow")?;
        drop(stop_tx);
        input
            .join()
            .map_err(|_| anyhow::anyhow!("Rainbow stop input thread failed"))?;
        guard.cancel()?;
    } else {
        let native_restore = restore_effect
            .native_effect()
            .context("Rainbow restoration must use the continuous stream")?;
        println!(
            "EFFECT HOLD END: sending the selected {restore_effect:?} restoration sequence now"
        );
        send_effect_sequence(
            &mut transport,
            SequenceChannel::Argb(channel),
            config.reported_channel_count,
            native_restore,
            Rgb::new(0, 0, 0),
        )
        .context("sending explicitly selected restore sequence")?;
        println!("RESTORE SENT: five {restore_effect:?} reports accepted for ARGB header {channel}; observe whether lighting resumes (the controller provides no ACK/readback)");
        guard.cancel()?;
    }
    Ok(())
}

fn run_rainbow_stream(
    reverse_argb_channel: Option<u8>,
    reverse_led_segment: Option<(usize, usize)>,
) -> Result<()> {
    let api = HidApi::new().context("initializing HIDAPI")?;
    let (mut transport, _writer_lock) = open_controller(&api, true)?;
    let config = query_config(&mut transport)?;
    let mut targets = config.rainbow_targets()?;
    if let Some(channel) = reverse_argb_channel {
        ensure!(
            channel < config.reported_channel_count,
            "ARGB channel {channel} is not reported by the controller"
        );
        let target = targets
            .iter_mut()
            .find(|target| target.channel == DirectChannel::Argb(channel))
            .context("selected ARGB channel has no Rainbow target")?;
        target.reversed_segment = Some(reverse_led_segment.unwrap_or((0, target.led_count)));
    }
    println!(
        "Configuration: {} ARGB channels; Rainbow targets: {:?}",
        config.reported_channel_count, targets
    );
    let guard = RecoveryGuard::arm(0, EffectArg::Rainbow)?;
    if let Some(channel) = reverse_argb_channel {
        let target = targets
            .iter()
            .find(|target| target.channel == DirectChannel::Argb(channel))
            .expect("validated Rainbow target");
        println!(
            "Starting direct AuraSync Rainbow; ARGB {channel} LED range {:?} is reversed.",
            target.reversed_segment
        );
    } else {
        println!("Starting direct AuraSync Rainbow on every configured Aura target.");
    }
    println!("Watch the lighting now; press Enter when the visual comparison is complete. The last frame remains displayed after stopping.");

    let (stop_tx, stop_rx) = mpsc::channel();
    let input_stop = stop_tx.clone();
    let input = thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        if !line.is_empty() {
            let _ = input_stop.send(());
        }
    });
    restore_rainbow(&mut transport, &config, &targets, stop_rx)
        .context("running continuous Windows AuraSync Rainbow")?;
    drop(stop_tx);
    input
        .join()
        .map_err(|_| anyhow::anyhow!("Rainbow stop input thread failed"))?;
    guard.cancel()?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.recovery_guard {
        return run_recovery_guard(
            args.watchdog_channel
                .context("watchdog channel is required")?,
            args.watchdog_effect
                .context("watchdog restore effect is required")?,
        );
    }
    if args.rainbow_stream {
        return run_rainbow_stream(args.reverse_argb_channel, args.reverse_led_segment);
    }
    let channel = args
        .identify_channel
        .context("--identify-channel is required")?;
    let test_effect = args
        .test_effect
        .context("--test-effect is required (for example --test-effect breathing)")?;
    let restore = args.restore_effect.unwrap_or(EffectArg::Rainbow);
    run_identification_test(channel, test_effect, restore, args.duration_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults_to_windows_rainbow_restoration_for_argb_zero() {
        let args = Args::try_parse_from([
            "asus-aura-lighting-test",
            "--identify-channel",
            "0",
            "--test-effect",
            "direct-magenta",
            "--duration-seconds",
            "15",
        ])
        .unwrap();
        assert_eq!(args.identify_channel, Some(0));
        assert_eq!(args.test_effect, Some(EffectArg::DirectMagenta));
        assert_eq!(args.duration_seconds, 15);
        assert_eq!(args.restore_effect, Some(EffectArg::Rainbow));
    }

    #[test]
    fn cli_supports_continuous_rainbow_without_a_preceding_test_effect() {
        let args = Args::try_parse_from(["asus-aura-lighting-test", "--rainbow-stream"]).unwrap();
        assert!(args.rainbow_stream);
        assert!(args.identify_channel.is_none());
        assert!(args.test_effect.is_none());
    }

    #[test]
    fn cli_accepts_reversing_a_segment_on_one_rainbow_channel() {
        let args = Args::try_parse_from([
            "asus-aura-lighting-test",
            "--rainbow-stream",
            "--reverse-argb-channel",
            "1",
            "--reverse-led-segment",
            "24:48",
        ])
        .unwrap();
        assert_eq!(args.reverse_argb_channel, Some(1));
        assert_eq!(args.reverse_led_segment, Some((24, 48)));
        assert_eq!(
            parse_led_segment("5:5"),
            Err("LED range must have START < END".into())
        );
    }
}

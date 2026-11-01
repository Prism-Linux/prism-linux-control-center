use anyhow::{ensure, Context, Result};
use clap::{Parser, ValueEnum};
use hidapi::{DeviceInfo, HidApi};
use lianli_devices::ene6k77::{motherboard_rgb_sync_frame, Ene6k77Firmware, Ene6k77Model};
use lianli_transport::{HidTransport, HidrawTransport};
use std::ffi::{CString, OsStr};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const VENDOR_ID: u16 = 0x0CF2;
const PRODUCT_ID: u16 = 0xA102;
const HID_INTERFACE: i32 = 1;
const REPORT_ID: u8 = 0xE0;
const RESPONSE_CAPACITY: usize = 65;
const COMMAND_DELAY: Duration = Duration::from_millis(20);
const CONTROL_RECOVERY_DEADLINE: Duration = Duration::from_secs(15);
const GROUP_UNDER_TEST: u8 = 0;
const MANUAL_TEST_PERCENT: u8 = 70;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum RgbSyncSelection {
    On,
    Off,
}

impl RgbSyncSelection {
    fn enabled(self) -> bool {
        self == Self::On
    }
}

#[derive(Debug, Parser)]
#[command(about = "SL-Infinity read-only diagnostics and isolated motherboard RGB sync control")]
struct Args {
    /// Send the two documented inquiry feature reports. Without a mode flag, only enumerate the HID interface.
    #[arg(long)]
    run_inquiries: bool,
    /// Run the bounded Group 0 PWM test, establishing motherboard sync as the final mode.
    #[arg(long, conflicts_with = "run_inquiries")]
    run_control_test: bool,
    /// Explicitly set motherboard ARGB input sync with register 0x61 only; does not touch PWM.
    #[arg(long, value_enum, conflicts_with_all = ["run_inquiries", "run_control_test"])]
    set_mb_rgb_sync: Option<RgbSyncSelection>,
    #[arg(long, hide = true)]
    recovery_guard: bool,
    #[arg(long, hide = true, requires = "recovery_guard")]
    recovery_path: Option<String>,
}

fn matching_devices(api: &HidApi) -> Vec<&DeviceInfo> {
    api.device_list()
        .filter(|info| info.vendor_id() == VENDOR_ID && info.product_id() == PRODUCT_ID)
        .collect()
}

fn print_device(info: &DeviceInfo) {
    println!(
        "HID candidate: {:04x}:{:04x} interface={} usage_page=0x{:04x} usage=0x{:04x} path={}",
        info.vendor_id(),
        info.product_id(),
        info.interface_number(),
        info.usage_page(),
        info.usage(),
        info.path().to_string_lossy()
    );
    if let Some(manufacturer) = info.manufacturer_string() {
        println!("  manufacturer: {manufacturer}");
    }
    if let Some(product) = info.product_string() {
        println!("  product: {product}");
    }
    if let Some(serial) = info.serial_number() {
        println!("  serial: {serial}");
    }
}

fn select_interface<'a>(devices: &[&'a DeviceInfo]) -> Result<&'a DeviceInfo> {
    let matches: Vec<_> = devices
        .iter()
        .copied()
        .filter(|info| info.interface_number() == HID_INTERFACE)
        .collect();
    ensure!(
        matches.len() == 1,
        "expected one {:04x}:{:04x} HID interface {HID_INTERFACE}, found {}",
        VENDOR_ID,
        PRODUCT_ID,
        matches.len()
    );
    Ok(matches[0])
}

fn feature_inquiry(
    transport: &mut HidrawTransport,
    request: [u8; 3],
    minimum_response_length: usize,
) -> Result<Vec<u8>> {
    ensure!(request[0] == REPORT_ID && request[1] == 0x50);
    let written = transport
        .send_feature_report(&request)
        .context("sending inquiry feature report")?;
    ensure!(
        written == request.len(),
        "short feature report write: {written}/{}",
        request.len()
    );

    thread::sleep(COMMAND_DELAY);
    let mut response = [0u8; RESPONSE_CAPACITY];
    response[0] = REPORT_ID;
    let read = transport
        .get_input_report(&mut response)
        .context("reading inquiry response")?;
    ensure!(
        read <= response.len(),
        "HID returned oversized response length {read}"
    );
    ensure!(
        read >= minimum_response_length,
        "short inquiry response: {read} bytes, expected at least {minimum_response_length}"
    );
    Ok(response[..read].to_vec())
}

fn decode_firmware_response(raw: &[u8]) -> Result<Ene6k77Firmware> {
    ensure!(
        raw.len() >= 6,
        "firmware response is too short: {} bytes",
        raw.len()
    );
    ensure!(
        raw[0] == REPORT_ID,
        "firmware response report ID is 0x{:02x}, expected 0x{REPORT_ID:02x}",
        raw[0]
    );
    ensure!(
        raw[1] == 0xE0,
        "firmware customer ID is 0x{:02x}, expected 0xe0",
        raw[1]
    );
    ensure!(
        raw[2] == 0x50,
        "firmware project ID is 0x{:02x}, expected 0x50",
        raw[2]
    );

    Ok(Ene6k77Firmware {
        model: Ene6k77Model::SlInfinity,
        customer_id: raw[1],
        project_id: raw[2],
        major_id: raw[3],
        minor_id: raw[4],
        fine_tune: raw[5],
    })
}

fn decode_rpm_response(raw: &[u8]) -> Result<[u16; 4]> {
    ensure!(
        raw.len() >= 9,
        "RPM response is too short: {} bytes",
        raw.len()
    );
    ensure!(
        raw[0] == REPORT_ID,
        "RPM response report ID is 0x{:02x}, expected 0x{REPORT_ID:02x}",
        raw[0]
    );
    let mut rpms = [0u16; 4];
    for (group, rpm) in rpms.iter_mut().enumerate() {
        let offset = 1 + group * 2;
        *rpm = u16::from_be_bytes([raw[offset], raw[offset + 1]]);
    }
    Ok(rpms)
}

fn mb_sync_command(group: u8, enabled: bool) -> Result<[u8; 6]> {
    ensure!(group < 4, "fan group {group} is out of range");
    let data = (1u8 << (group + 4)) | ((enabled as u8) << group);
    Ok([REPORT_ID, 0x10, 0x62, data, 0x00, 0x00])
}

fn fan_speed_command(group: u8, percent: u8) -> Result<[u8; 4]> {
    ensure!(group < 4, "fan group {group} is out of range");
    ensure!(percent <= 100, "fan speed {percent}% exceeds 100%");
    Ok([REPORT_ID, 0x20 | group, 0x00, percent])
}

fn send_control(transport: &mut HidrawTransport, command: &[u8], label: &str) -> Result<()> {
    let written = transport
        .send_feature_report(command)
        .with_context(|| format!("sending {label}"))?;
    ensure!(
        written == command.len(),
        "short {label} write: {written}/{}",
        command.len()
    );
    eprintln!(
        "{label} command: {:02x?} ({written} bytes accepted)",
        command
    );
    thread::sleep(COMMAND_DELAY);
    Ok(())
}

fn open_transport(api: &HidApi, info: &DeviceInfo) -> Result<HidrawTransport> {
    let path = info.path().to_owned();
    let path_os = OsStr::from_bytes(path.to_bytes()).to_os_string();
    let device = api
        .open_path(&path)
        .with_context(|| format!("opening HID interface {}", info.interface_number()))?;
    Ok(HidrawTransport::new(device, Some(path_os)))
}

fn read_rpm_report(transport: &mut HidrawTransport) -> Result<(Vec<u8>, [u16; 4])> {
    let raw = feature_inquiry(transport, [REPORT_ID, 0x50, 0x00], 9)?;
    let rpms = decode_rpm_response(&raw)?;
    Ok((raw, rpms))
}

struct RecoveryGuard {
    child: Child,
    stdin: Option<ChildStdin>,
}

impl RecoveryGuard {
    fn arm(executable: &std::path::Path, device_path: &str) -> Result<Self> {
        let mut child = Command::new(executable)
            .arg("--recovery-guard")
            .arg("--recovery-path")
            .arg(device_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("starting independent sync recovery process")?;
        let stdin = child
            .stdin
            .take()
            .context("recovery process stdin unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("recovery process stdout unavailable")?;
        let mut reader = BufReader::new(stdout);
        let mut ready = String::new();
        reader
            .read_line(&mut ready)
            .context("waiting for recovery process readiness")?;
        ensure!(
            ready.trim() == "READY",
            "recovery process did not become ready: {}",
            ready.trim()
        );
        println!("independent recovery process ready (15-second deadline)");
        Ok(Self {
            child,
            stdin: Some(stdin),
        })
    }

    fn complete(mut self) -> Result<()> {
        if let Some(mut stdin) = self.stdin.take() {
            stdin.write_all(b"COMPLETE")?;
            stdin.flush()?;
            drop(stdin);
        }
        let status = self.child.wait().context("waiting for recovery process")?;
        ensure!(status.success(), "recovery process exited with {status}");
        println!("recovery process canceled after verified sync restoration");
        Ok(())
    }
}

impl Drop for RecoveryGuard {
    fn drop(&mut self) {
        if self.stdin.take().is_some() {
            eprintln!(
                "probe leaving recovery process armed; it will enable Group 0 motherboard sync"
            );
        }
    }
}

fn recovery_guard(path: &str) -> Result<()> {
    let api = HidApi::new().context("enumerating HID devices for recovery")?;
    let c_path = CString::new(path.as_bytes()).context("invalid HID device path")?;
    let device = api
        .open_path(c_path.as_c_str())
        .context("opening HID interface for recovery")?;
    let path_os = OsStr::from_bytes(path.as_bytes()).to_os_string();
    let mut transport = HidrawTransport::new(device, Some(path_os));
    println!("READY");
    std::io::stdout().flush()?;

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut message = String::new();
        let result = std::io::stdin().read_to_string(&mut message);
        let _ = sender.send((result, message));
    });

    let completion = receiver.recv_timeout(CONTROL_RECOVERY_DEADLINE);
    if matches!(completion, Ok((Ok(_), message)) if message == "COMPLETE") {
        return Ok(());
    }

    let command = mb_sync_command(GROUP_UNDER_TEST, true)?;
    let mut last_error = None;
    for attempt in 1..=3 {
        match send_control(
            &mut transport,
            &command,
            "recovery: enable Group 0 motherboard PWM sync",
        ) {
            Ok(()) => {
                eprintln!(
                    "recovery process enabled Group 0 motherboard PWM sync (attempt {attempt})"
                );
                return Ok(());
            }
            Err(error) => {
                eprintln!("recovery attempt {attempt}/3 failed: {error:#}");
                last_error = Some(error);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("recovery failed without an error")))
}

fn report_rpms(label: &str, raw: &[u8], rpms: &[u16; 4]) {
    println!("{label} RPM response ({} bytes): {:02x?}", raw.len(), raw);
    for (group, rpm) in rpms.iter().enumerate() {
        println!("{label} fan group {group}: {rpm} RPM");
    }
}

fn run_control_test(info: &DeviceInfo, transport: &mut HidrawTransport) -> Result<()> {
    let firmware_raw = feature_inquiry(transport, [REPORT_ID, 0x50, 0x01], 6)?;
    let firmware = decode_firmware_response(&firmware_raw)?;
    ensure!(firmware.is_valid(), "controller firmware identity mismatch");
    println!("firmware response: {:02x?}", firmware_raw);
    let (major, minor) = firmware.version();
    println!("identified controller firmware v{major}.{minor}");

    let (initial_raw, initial_rpms) = read_rpm_report(transport)?;
    report_rpms("initial", &initial_raw, &initial_rpms);
    ensure!(
        initial_rpms[GROUP_UNDER_TEST as usize] > 0,
        "Group {GROUP_UNDER_TEST} did not report a running fan; refusing control test"
    );
    println!(
        "Group {GROUP_UNDER_TEST} confirmed populated at {} RPM; Groups 1-3 will not be written",
        initial_rpms[GROUP_UNDER_TEST as usize]
    );

    let path = info.path().to_string_lossy().into_owned();
    let executable = std::env::current_exe().context("locating diagnostic probe executable")?;
    let recovery = RecoveryGuard::arm(&executable, &path)?;

    send_control(
        transport,
        &mb_sync_command(GROUP_UNDER_TEST, true)?,
        "set Group 0 motherboard PWM sync enabled",
    )?;
    let (sync_raw, sync_rpms) = read_rpm_report(transport)?;
    report_rpms("after sync enabled", &sync_raw, &sync_rpms);
    ensure!(
        sync_rpms[GROUP_UNDER_TEST as usize] > 0,
        "Group {GROUP_UNDER_TEST} stopped reporting RPM after motherboard sync was enabled"
    );

    send_control(
        transport,
        &mb_sync_command(GROUP_UNDER_TEST, false)?,
        "set Group 0 motherboard PWM sync disabled",
    )?;
    send_control(
        transport,
        &fan_speed_command(GROUP_UNDER_TEST, MANUAL_TEST_PERCENT)?,
        "set Group 0 manual fan speed to 70%",
    )?;
    println!("holding manual test setting for 1.5 seconds");
    thread::sleep(Duration::from_millis(1500));

    let (manual_raw, manual_rpms) = read_rpm_report(transport)?;
    report_rpms("manual 70%", &manual_raw, &manual_rpms);
    ensure!(
        manual_rpms[GROUP_UNDER_TEST as usize] > 0,
        "Group {GROUP_UNDER_TEST} RPM became zero during manual test"
    );

    send_control(
        transport,
        &mb_sync_command(GROUP_UNDER_TEST, true)?,
        "restore Group 0 motherboard PWM sync enabled",
    )?;
    thread::sleep(Duration::from_millis(250));
    let (final_raw, final_rpms) = read_rpm_report(transport)?;
    report_rpms("final sync enabled", &final_raw, &final_rpms);
    ensure!(
        final_rpms[GROUP_UNDER_TEST as usize] > 0,
        "Group {GROUP_UNDER_TEST} did not report a running fan after sync restoration"
    );
    recovery.complete()?;
    println!("Group 0 motherboard PWM sync is the established final control mode");
    Ok(())
}

fn run() -> Result<()> {
    let args = Args::parse();
    if args.recovery_guard {
        let path = args
            .recovery_path
            .as_deref()
            .context("recovery device path is required")?;
        return recovery_guard(path);
    }
    let api = HidApi::new().context("enumerating HID devices")?;
    let devices = matching_devices(&api);
    ensure!(
        !devices.is_empty(),
        "SL-Infinity HID device {:04x}:{:04x} not found",
        VENDOR_ID,
        PRODUCT_ID
    );
    for info in &devices {
        print_device(info);
    }

    let info = select_interface(&devices)?;
    if !args.run_inquiries && !args.run_control_test && args.set_mb_rgb_sync.is_none() {
        println!("Selected the unique HID interface {HID_INTERFACE}; no device protocol requests were sent.");
        return Ok(());
    }

    let mut transport = open_transport(&api, info)?;

    if let Some(selection) = args.set_mb_rgb_sync {
        let command = motherboard_rgb_sync_frame(Ene6k77Model::SlInfinity, selection.enabled());
        send_control(
            &mut transport,
            &command,
            if selection.enabled() {
                "set SL-Infinity motherboard ARGB sync enabled"
            } else {
                "set SL-Infinity motherboard ARGB sync disabled"
            },
        )?;
        println!(
            "complete RGB sync report accepted; no ACK/readback exists; no 0x62 PWM command was sent"
        );
        return Ok(());
    }

    if args.run_control_test {
        return run_control_test(info, &mut transport);
    }

    let firmware_raw = feature_inquiry(&mut transport, [REPORT_ID, 0x50, 0x01], 6)?;
    println!("firmware request: e0 50 01");
    println!(
        "firmware response ({} bytes): {:02x?}",
        firmware_raw.len(),
        firmware_raw
    );
    let firmware = decode_firmware_response(&firmware_raw)?;
    let (major, minor) = firmware.version();
    println!(
        "firmware: v{major}.{minor}; IDs customer={:#04x} project={:#04x} major={:#04x} minor={:#04x} fine_tune={:#04x}; expected-ID-match={}",
        firmware.customer_id,
        firmware.project_id,
        firmware.major_id,
        firmware.minor_id,
        firmware.fine_tune,
        firmware.is_valid()
    );

    let rpm_raw = feature_inquiry(&mut transport, [REPORT_ID, 0x50, 0x00], 9)?;
    println!("RPM request: e0 50 00");
    println!("RPM response ({} bytes): {:02x?}", rpm_raw.len(), rpm_raw);
    let rpms = decode_rpm_response(&rpm_raw)?;
    for (group, rpm) in rpms.iter().enumerate() {
        println!("fan group {group}: {rpm} RPM");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("SL-Infinity probe failed: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_sl_infinity_firmware_v1_4() {
        let firmware = decode_firmware_response(&[0xE0, 0xE0, 0x50, 0x80, 0xC4, 0x0D]).unwrap();
        assert!(firmware.is_valid());
        assert_eq!(firmware.version(), (1, 4));
        assert_eq!(firmware.to_string(), "v1.4");
    }

    #[test]
    fn rejects_short_or_unexpected_firmware_responses() {
        assert!(decode_firmware_response(&[0xE0, 0xE0, 0x50, 0x80, 0xC4]).is_err());
        assert!(decode_firmware_response(&[0x00, 0xE0, 0x50, 0x80, 0xC4, 0x0D]).is_err());
        assert!(decode_firmware_response(&[0xE0, 0xE1, 0x50, 0x80, 0xC4, 0x0D]).is_err());
        assert!(decode_firmware_response(&[0xE0, 0xE0, 0x51, 0x80, 0xC4, 0x0D]).is_err());
    }

    #[test]
    fn decodes_four_big_endian_fan_group_rpms() {
        let rpms =
            decode_rpm_response(&[0xE0, 0x04, 0xB0, 0x04, 0x4C, 0x00, 0x00, 0x02, 0x58]).unwrap();
        assert_eq!(rpms, [1200, 1100, 0, 600]);
    }

    #[test]
    fn rejects_short_or_wrong_id_rpm_response() {
        assert!(decode_rpm_response(&[0xE0, 0, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(decode_rpm_response(&[0x00, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    }

    #[test]
    fn group_zero_motherboard_sync_reports_only_target_group() {
        assert_eq!(
            mb_sync_command(0, true).unwrap(),
            [0xE0, 0x10, 0x62, 0x11, 0, 0]
        );
        assert_eq!(
            mb_sync_command(0, false).unwrap(),
            [0xE0, 0x10, 0x62, 0x10, 0, 0]
        );
    }

    #[test]
    fn motherboard_rgb_sync_uses_controller_register_61_only() {
        assert_eq!(
            motherboard_rgb_sync_frame(Ene6k77Model::SlInfinity, true),
            [0xE0, 0x10, 0x61, 0x01, 0, 0]
        );
        assert_eq!(
            motherboard_rgb_sync_frame(Ene6k77Model::SlInfinity, false),
            [0xE0, 0x10, 0x61, 0x00, 0, 0]
        );
        assert_ne!(
            motherboard_rgb_sync_frame(Ene6k77Model::SlInfinity, true)[2],
            0x62
        );
    }

    #[test]
    fn manual_speed_command_is_scoped_and_percent_encoded() {
        assert_eq!(fan_speed_command(0, 70).unwrap(), [0xE0, 0x20, 0x00, 70]);
        assert!(fan_speed_command(4, 70).is_err());
        assert!(fan_speed_command(0, 101).is_err());
        assert!(mb_sync_command(4, true).is_err());
    }
}

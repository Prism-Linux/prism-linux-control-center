//! Linux SMBus backend for the second-generation ENE DRAM RGB controller.
//!
//! Discovery is deliberately limited to the Intel I801 controller and the two
//! controller addresses identified on the author's Z790-E host. The device
//! signature and DRAM zone configuration are checked before a handle is exposed.

use crate::traits::RgbDevice;
use anyhow::{bail, Context, Result};
use lianli_shared::rgb::{RgbEffect, RgbMode, RgbZoneInfo};
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const PCI_VENDOR: &str = "0x8086";
const PCI_DEVICE: &str = "0x7a23";
const ENE_NAME: &[u8] = b"AUDA0-E6K5-0101";
const KNOWN_ADDRESSES: [u8; 2] = [0x71, 0x73];
const CONFIG_TABLE: u16 = 0x1c00;
const COLOR_EFFECT_V2: u16 = 0x8160;
const COLOR_DIRECT_V2: u16 = 0x8100;
const REG_DIRECT: u16 = 0x8020;
const REG_MODE: u16 = 0x8021;
const REG_SPEED: u16 = 0x8022;
const REG_DIRECTION: u16 = 0x8023;
const REG_APPLY: u16 = 0x80a0;
const I2C_SLAVE: libc::c_ulong = 0x0703;
const I2C_SMBUS: libc::c_ulong = 0x0720;
const SMBUS_WRITE: u8 = 0;
const SMBUS_READ: u8 = 1;
const SMBUS_BYTE_DATA: u32 = 2;
const SMBUS_WORD_DATA: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
union SmbusData {
    byte: u8,
    word: u16,
    block: [u8; 34],
}

#[repr(C)]
struct SmbusIoctlData {
    read_write: u8,
    command: u8,
    size: u32,
    data: *mut SmbusData,
}

#[derive(Debug)]
struct ControllerInfo {
    address: u8,
    version: String,
    led_count: u8,
}

/// Enumerate only known ENE addresses on the matching Intel I801 adapter.
/// Discovery reads only identity/config data. SMBus word writes select the
/// indirect read pointer; no RGB data, mode, Apply, or persistent Save register
/// is written during discovery.
pub fn discover() -> Result<Vec<(String, std::sync::Arc<dyn RgbDevice>)>> {
    let mut discovered = Vec::new();
    let adapter_root = Path::new("/sys/bus/i2c/devices");
    tracing::debug!(adapter_root = %adapter_root.display(), "Starting allowlisted ENE DRAM discovery");
    let adapters = fs::read_dir(adapter_root).context("enumerating I2C adapters")?;
    for entry in adapters {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(%error, "Could not enumerate an I2C sysfs entry during ENE DRAM discovery");
                continue;
            }
        };
        let path = entry.path();
        let Some(bus_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(bus_text) = bus_name.strip_prefix("i2c-") else {
            continue;
        };
        let Ok(bus) = bus_text.parse::<u32>() else {
            continue;
        };
        let adapter_name = match fs::read_to_string(path.join("name")) {
            Ok(name) => name.trim().to_owned(),
            Err(error) => {
                tracing::debug!(bus, adapter_path = %path.display(), %error,
                    "Could not read I2C adapter name; skipping non-candidate adapter");
                continue;
            }
        };
        if !adapter_name.contains("SMBus I801 adapter") {
            continue;
        }
        tracing::info!(bus, adapter_path = %path.display(), adapter_name,
            "Found Intel I801 adapter candidate for ENE DRAM discovery");
        let resolved_path = match fs::canonicalize(&path) {
            Ok(resolved_path) => resolved_path,
            Err(error) => {
                tracing::warn!(bus, adapter_path = %path.display(), %error,
                    "Could not resolve I801 adapter sysfs path");
                continue;
            }
        };
        let Some((pci_path, vendor, device)) =
            pci_identity(&resolved_path, |directory, attribute| {
                fs::read_to_string(directory.join(attribute)).ok()
            })
        else {
            tracing::warn!(bus, adapter_path = %path.display(), resolved_path = %resolved_path.display(),
                "I801 candidate has no readable PCI ancestor; skipping ENE address probes");
            continue;
        };
        if vendor.trim() != PCI_VENDOR || device.trim() != PCI_DEVICE {
            tracing::info!(bus, adapter_path = %path.display(), resolved_path = %resolved_path.display(),
                pci_path = %pci_path.display(), pci_vendor = vendor.trim(), pci_device = device.trim(),
                expected_vendor = PCI_VENDOR, expected_device = PCI_DEVICE, allowlisted = false,
                "I801 PCI identity is outside the ENE allowlist; skipping adapter");
            continue;
        }
        let pci_id = pci_path
            .file_name()
            .and_then(|bdf| bdf.to_str())
            .map(|bdf| bdf.replace([':', '.'], "-"))
            .context("matching I801 PCI ancestor has no BDF filename")?;
        tracing::info!(bus, adapter_path = %path.display(), resolved_path = %resolved_path.display(),
            pci_path = %pci_path.display(), pci_id, pci_vendor = %vendor.trim(), pci_device = %device.trim(),
            "I801 adapter PCI identity matches the ENE DRAM allowlist");
        let dev_path = PathBuf::from(format!("/dev/i2c-{bus}"));
        let file = match OpenOptions::new().read(true).write(true).open(&dev_path) {
            Ok(file) => {
                tracing::info!(bus, device_path = %dev_path.display(), pci_id,
                    "Opened allowlisted I801 adapter for read-only ENE identity checks");
                file
            }
            Err(error) => {
                tracing::warn!(bus, device_path = %dev_path.display(), pci_id, %error,
                    "Could not open allowlisted I801 adapter for ENE identity checks");
                continue;
            }
        };
        for address in KNOWN_ADDRESSES {
            tracing::info!(
                bus,
                pci_id,
                address = format_args!("0x{address:02x}"),
                "Checking allowlisted ENE DRAM controller address"
            );
            match identify(&file, address) {
                Ok(info) => {
                    let id = format!("ene-dram-{pci_id}-{address:02x}");
                    tracing::info!(device_id = %id, address = format_args!("0x{address:02x}"),
                        controller_variant = %info.version, led_count = info.led_count,
                        protocol_variant = "AUDA0-v2", "Validated ENE DRAM controller identity and configuration");
                    let device = match EneDramRgb::new(&dev_path, info) {
                        Ok(device) => device,
                        Err(error) => {
                            tracing::warn!(device_id = %id, device_path = %dev_path.display(), %error,
                                "Validated ENE controller but could not construct its RGB device handle");
                            continue;
                        }
                    };
                    tracing::info!(device_id = %id, "Constructed ENE DRAM RGB device handle");
                    discovered.push((
                        id,
                        std::sync::Arc::new(device) as std::sync::Arc<dyn RgbDevice>,
                    ));
                }
                Err(error) => {
                    tracing::warn!(bus, pci_id, address = format_args!("0x{address:02x}"), %error,
                        "Rejected allowlisted ENE DRAM address after read-only identification");
                }
            }
        }
    }
    tracing::info!(controller_count = discovered.len(),
        device_ids = ?discovered.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        "ENE DRAM discovery completed");
    Ok(discovered)
}

fn pci_identity(
    target: &Path,
    mut read_attribute: impl FnMut(&Path, &str) -> Option<String>,
) -> Option<(PathBuf, String, String)> {
    target.ancestors().find_map(|ancestor| {
        let vendor = read_attribute(ancestor, "vendor")?;
        let device = read_attribute(ancestor, "device")?;
        Some((ancestor.to_path_buf(), vendor, device))
    })
}

fn identify(file: &File, address: u8) -> Result<ControllerInfo> {
    tracing::debug!(
        address = format_args!("0x{address:02x}"),
        register = "0x1000",
        length = 16,
        "Reading ENE firmware identity through indirect SMBus register window"
    );
    let name = read_bytes(file, address, 0x1000, 16)
        .with_context(|| format!("reading firmware identity at SMBus 0x{address:02x}"))?;
    let normalized_name = name
        .iter()
        .copied()
        .take_while(|b| *b != 0)
        .collect::<Vec<_>>();
    if normalized_name.as_slice() != ENE_NAME {
        let observed = String::from_utf8_lossy(&normalized_name).into_owned();
        bail!(
            "firmware identity rejection: expected {:?} ({} bytes), observed {:?} ({} bytes)",
            String::from_utf8_lossy(ENE_NAME),
            ENE_NAME.len(),
            observed,
            normalized_name.len()
        );
    }
    tracing::debug!(address = format_args!("0x{address:02x}"), observed_identity = %String::from_utf8_lossy(&normalized_name),
        "ENE firmware identity matched; reading DRAM configuration table");
    let config = read_bytes(file, address, CONFIG_TABLE, 64)
        .with_context(|| format!("reading ENE DRAM configuration at SMBus 0x{address:02x}"))?;
    parse_identity(address, &normalized_name, &config)
}

fn parse_identity(address: u8, name: &[u8], config: &[u8]) -> Result<ControllerInfo> {
    let name = name
        .iter()
        .copied()
        .take_while(|b| *b != 0)
        .collect::<Vec<_>>();
    if name.as_slice() != ENE_NAME {
        bail!(
            "firmware identity rejection: expected {:?}, observed {:?}",
            String::from_utf8_lossy(ENE_NAME),
            String::from_utf8_lossy(&name)
        );
    }
    anyhow::ensure!(config.len() == 64, "ENE config table must contain 64 bytes");
    let led_count = config[2];
    let dram_zones = (0..8)
        .filter(|index| {
            config[0x13 + index] == 0x05
                || config[0x13 + index] == 0x0e
                || config[0x13 + index] == 0x8a
        })
        .count();
    if led_count == 0 || led_count > 10 || dram_zones == 0 || config[3] != led_count {
        bail!("controller {:?} rejected: expected 1..=10 global LEDs, zone0 equal to global count, and at least one DRAM zone; observed global LEDs={}, zone0 LEDs={}, DRAM zones={dram_zones}",
            String::from_utf8_lossy(&name), led_count, config[3]);
    }
    Ok(ControllerInfo {
        address,
        version: String::from_utf8_lossy(&name).into_owned(),
        led_count,
    })
}

fn select_address(file: &File, address: u8) -> Result<()> {
    // SAFETY: ioctl receives the I2C_SLAVE integer address for an open i2c-dev fd.
    let result = unsafe { libc::ioctl(file.as_raw_fd(), I2C_SLAVE, libc::c_ulong::from(address)) };
    if result < 0 {
        return Err(std::io::Error::last_os_error()).context("selecting ENE SMBus address");
    }
    Ok(())
}

fn smbus(file: &File, address: u8, rw: u8, command: u8, size: u32, value: u16) -> Result<u16> {
    select_address(file, address)?;
    let mut data = SmbusData { word: value };
    let mut args = SmbusIoctlData {
        read_write: rw,
        command,
        size,
        data: &mut data,
    };
    // SAFETY: args/data match Linux's i2c_smbus_ioctl_data ABI; ioctl is synchronous.
    let result = unsafe { libc::ioctl(file.as_raw_fd(), I2C_SMBUS, &mut args) };
    if result < 0 {
        return Err(std::io::Error::last_os_error()).context("ENE SMBus transaction");
    }
    // SAFETY: the selected operation determines which union member was filled.
    Ok(unsafe {
        if size == SMBUS_WORD_DATA {
            data.word
        } else {
            u16::from(data.byte)
        }
    })
}

fn set_register_pointer(file: &File, address: u8, register: u16) -> Result<()> {
    let swapped = register_pointer_word(register);
    smbus(file, address, SMBUS_WRITE, 0x00, SMBUS_WORD_DATA, swapped).map(|_| ())
}

fn register_pointer_word(register: u16) -> u16 {
    register.swap_bytes()
}

fn ene_color_bytes(rgb: [u8; 3]) -> [u8; 3] {
    [rgb[0], rgb[2], rgb[1]]
}

fn effect_register_writes(
    effect: &RgbEffect,
    colors: &[[u8; 3]],
    led_count: u8,
    direct: bool,
) -> Result<Vec<(u16, u8)>> {
    anyhow::ensure!(
        led_count > 0 && !colors.is_empty(),
        "ENE RGB color layout is empty"
    );
    let direct = direct && !effect.disabled;
    let brightness = if effect.disabled || effect.mode == RgbMode::Off {
        0
    } else {
        effect.brightness.min(4)
    };
    let base = if direct {
        COLOR_DIRECT_V2
    } else {
        COLOR_EFFECT_V2
    };
    let mut writes = Vec::with_capacity(usize::from(led_count) * 3 + 5);
    for index in 0..usize::from(led_count) {
        let color = colors[index % colors.len()];
        for (channel, value) in ene_color_bytes(color).into_iter().enumerate() {
            let scaled = ((u16::from(value) * u16::from(brightness)) / 4) as u8;
            writes.push((base + (index * 3 + channel) as u16, scaled));
        }
    }
    let mode = if effect.disabled {
        0
    } else {
        match effect.mode {
            RgbMode::Off => 0,
            RgbMode::Static | RgbMode::Direct => 1,
            RgbMode::Breathing => 2,
            RgbMode::Rainbow => 5,
            other => bail!("ENE AUDA0 DRAM mode {other:?} is not supported"),
        }
    };
    writes.extend([
        (REG_MODE, mode),
        (REG_SPEED, 4u8.saturating_sub(effect.speed.min(4))),
        (
            REG_DIRECTION,
            u8::from(effect.direction == lianli_shared::rgb::RgbDirection::CounterClockwise),
        ),
        (REG_DIRECT, u8::from(direct)),
        (REG_APPLY, 0x01),
    ]);
    Ok(writes)
}

fn read_register(file: &File, address: u8, register: u16) -> Result<u8> {
    set_register_pointer(file, address, register).with_context(|| {
        format!("selecting indirect register 0x{register:04x} at address 0x{address:02x}")
    })?;
    Ok(
        smbus(file, address, SMBUS_READ, 0x81, SMBUS_BYTE_DATA, 0).with_context(|| {
            format!("reading indirect register 0x{register:04x} at address 0x{address:02x}")
        })? as u8,
    )
}

fn write_register(file: &File, address: u8, register: u16, value: u8) -> Result<()> {
    set_register_pointer(file, address, register)?;
    smbus(
        file,
        address,
        SMBUS_WRITE,
        0x01,
        SMBUS_BYTE_DATA,
        u16::from(value),
    )
    .map(|_| ())
}

fn read_bytes(file: &File, address: u8, start: u16, len: usize) -> Result<Vec<u8>> {
    (0..len)
        .map(|offset| read_register(file, address, start + offset as u16))
        .collect()
}

struct EneDramRgb {
    info: ControllerInfo,
    io: Mutex<File>,
}

impl EneDramRgb {
    fn new(path: &Path, info: ControllerInfo) -> Result<Self> {
        let io = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("opening ENE adapter {}", path.display()))?;
        Ok(Self {
            info,
            io: Mutex::new(io),
        })
    }

    fn apply(&self, effect: &RgbEffect, colors: &[[u8; 3]], direct: bool) -> Result<()> {
        let writes = effect_register_writes(effect, colors, self.info.led_count, direct)?;
        let io = self
            .io
            .lock()
            .map_err(|_| anyhow::anyhow!("ENE adapter lock was poisoned"))?;
        for (register, value) in writes {
            write_register(&io, self.info.address, register, value)?;
        }
        Ok(())
    }
}

impl RgbDevice for EneDramRgb {
    fn device_name(&self) -> String {
        format!(
            "G.Skill DDR5 RGB — ENE {} @ SMBus 0x{:02x}",
            self.info.version, self.info.address
        )
    }
    fn supported_modes(&self) -> Vec<RgbMode> {
        vec![
            RgbMode::Off,
            RgbMode::Static,
            RgbMode::Breathing,
            RgbMode::Rainbow,
            RgbMode::Direct,
        ]
    }
    fn zone_info(&self) -> Vec<RgbZoneInfo> {
        vec![RgbZoneInfo {
            name: "DIMM LEDs".into(),
            name_ui: None,
            led_count: u16::from(self.info.led_count),
        }]
    }
    fn supports_direct(&self) -> bool {
        true
    }
    fn set_zone_effect(&self, zone: u8, effect: &RgbEffect) -> Result<()> {
        anyhow::ensure!(zone == 0, "ENE DRAM controller has one RGB zone");
        anyhow::ensure!(
            matches!(
                effect.mode,
                RgbMode::Off
                    | RgbMode::Static
                    | RgbMode::Breathing
                    | RgbMode::Rainbow
                    | RgbMode::Direct
            ),
            "unsupported ENE DRAM effect"
        );
        let colors = if effect.colors.is_empty() {
            &[[255, 255, 255]][..]
        } else {
            &effect.colors
        };
        self.apply(effect, colors, effect.mode == RgbMode::Direct)
    }
    fn set_direct_colors(&self, zone: u8, colors: &[[u8; 3]]) -> Result<()> {
        anyhow::ensure!(zone == 0, "ENE DRAM controller has one RGB zone");
        anyhow::ensure!(
            !colors.is_empty() && colors.len() <= self.info.led_count as usize,
            "invalid ENE DRAM color count"
        );
        let effect = RgbEffect {
            mode: RgbMode::Direct,
            colors: colors.to_vec(),
            ..RgbEffect::default()
        };
        self.apply(&effect, colors, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_identity_and_register_layout_match_observed_controller() {
        assert_eq!(ENE_NAME, b"AUDA0-E6K5-0101");
        assert_eq!(KNOWN_ADDRESSES, [0x71, 0x73]);
        assert_eq!(CONFIG_TABLE + 2, 0x1c02);
        assert_eq!(COLOR_DIRECT_V2, 0x8100);
        assert_eq!(COLOR_EFFECT_V2, 0x8160);
        assert_eq!(REG_APPLY, 0x80a0);
    }

    #[test]
    fn detects_exact_controller_and_active_dram_zone() {
        let mut name = [0u8; 16];
        name[..ENE_NAME.len()].copy_from_slice(ENE_NAME);
        let mut config = [0u8; 64];
        config[2] = 8;
        config[3] = 8;
        config[0x13] = 5;
        let identified = parse_identity(0x71, &name, &config).unwrap();
        assert_eq!(identified.address, 0x71);
        assert_eq!(identified.version, "AUDA0-E6K5-0101");
        assert_eq!(identified.led_count, 8);

        let mut wrong_name = name;
        wrong_name[0] = b'X';
        let error = parse_identity(0x71, &wrong_name, &config).unwrap_err();
        assert!(error.to_string().contains("expected \"AUDA0-E6K5-0101\""));
        assert!(error.to_string().contains("observed \"XUDA0-E6K5-0101\""));

        config[0x13] = 0;
        let error = parse_identity(0x71, &name, &config).unwrap_err();
        assert!(error
            .to_string()
            .contains("observed global LEDs=8, zone0 LEDs=8, DRAM zones=0"));
    }

    #[test]
    fn reported_hardware_identity_and_config_are_accepted_at_both_allowlisted_addresses() {
        let name = b"AUDA0-E6K5-0101\0";
        let config: [u8; 64] = hex::decode(
            "0101080800000000000000080000000000000005000000000000000000000000000000000102030405060708090a0b0c0d0e0f1011121314151617ff0000ff00",
        )
        .unwrap()
        .try_into()
        .unwrap();
        for address in KNOWN_ADDRESSES {
            let controller = parse_identity(address, name, &config).unwrap();
            assert_eq!(controller.address, address);
            assert_eq!(controller.version, "AUDA0-E6K5-0101");
            assert_eq!(controller.led_count, 8);
        }
    }

    #[test]
    fn device_id_has_stable_pci_location_and_address() {
        let pci_id = "0000-00-1f-4";
        let address = 0x71;
        assert_eq!(
            format!("ene-dram-{pci_id}-{address:02x}"),
            "ene-dram-0000-00-1f-4-71"
        );
    }

    #[test]
    fn i801_discovery_walks_up_from_adapter_device_path_to_pci_function() {
        let target = Path::new("/sys/devices/pci0000:00/0000:00:1f.4/i2c-3");
        let attributes = [
            ("/sys/devices/pci0000:00/0000:00:1f.4", "vendor", PCI_VENDOR),
            ("/sys/devices/pci0000:00/0000:00:1f.4", "device", PCI_DEVICE),
        ];
        let (pci_path, vendor, device) = pci_identity(target, |directory, attribute| {
            let directory = directory.to_str()?;
            attributes
                .iter()
                .find(|(path, key, _)| *path == directory && *key == attribute)
                .map(|(_, _, value)| (*value).to_owned())
        })
        .unwrap();
        assert_eq!(vendor, PCI_VENDOR);
        assert_eq!(device, PCI_DEVICE);
        assert_eq!(
            pci_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace([':', '.'], "-"),
            "0000-00-1f-4"
        );
    }

    #[test]
    fn protocol_encodes_little_endian_register_pointer_and_rgb_order() {
        assert_eq!(register_pointer_word(0x1c02), 0x021c);
        assert_eq!(ene_color_bytes([0x11, 0x22, 0x33]), [0x11, 0x33, 0x22]);
    }

    #[test]
    fn static_effect_writes_direct_colors_then_apply_without_persistent_save() {
        let effect = RgbEffect {
            mode: RgbMode::Static,
            colors: vec![[0x11, 0x22, 0x33]],
            ..RgbEffect::default()
        };
        let writes = effect_register_writes(&effect, &effect.colors, 1, false).unwrap();
        assert_eq!(
            &writes[..3],
            &[
                (COLOR_EFFECT_V2, 0x11),
                (COLOR_EFFECT_V2 + 1, 0x33),
                (COLOR_EFFECT_V2 + 2, 0x22)
            ]
        );
        assert_eq!(writes.last(), Some(&(REG_APPLY, 0x01)));
        assert!(!writes
            .iter()
            .any(|(register, value)| *register == REG_APPLY && *value == 0xaa));
        assert_eq!(
            writes.iter().find(|(register, _)| *register == REG_MODE),
            Some(&(REG_MODE, 1))
        );
    }

    #[test]
    fn disabled_effect_recovers_to_off_and_unsupported_modes_fail_before_writes() {
        let disabled = RgbEffect {
            mode: RgbMode::Rainbow,
            disabled: true,
            ..RgbEffect::default()
        };
        let writes = effect_register_writes(&disabled, &[[255, 255, 255]], 8, true).unwrap();
        assert_eq!(
            writes.iter().find(|(register, _)| *register == REG_MODE),
            Some(&(REG_MODE, 0))
        );
        assert_eq!(
            writes.iter().find(|(register, _)| *register == REG_DIRECT),
            Some(&(REG_DIRECT, 0))
        );

        let unsupported = RgbEffect {
            mode: RgbMode::Voice,
            ..RgbEffect::default()
        };
        assert!(effect_register_writes(&unsupported, &unsupported.colors, 8, false).is_err());
    }
}

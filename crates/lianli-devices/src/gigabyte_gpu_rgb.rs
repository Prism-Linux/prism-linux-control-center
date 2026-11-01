//! Native RGB control for the specifically identified RTX 4080 XTREME WATERFORCE WB.
//!
//! Windows capture established the 7-bit endpoint (0x64), an eight-byte command,
//! and the NVIDIA PortId 1. Linux's NVIDIA driver exposes that port as the
//! adapter named "NVIDIA i2c adapter 1". The sysfs discovery below binds by PCI
//! identity and that driver port name; it never relies on a fixed i2c-N number.

use crate::traits::RgbDevice;
use anyhow::{bail, Context, Result};
use lianli_shared::rgb::{RgbEffect, RgbMode, RgbZoneInfo};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const GPU_VENDOR: &str = "0x10de";
const GPU_DEVICE: &str = "0x2704";
const SUBSYSTEM_VENDOR: &str = "0x1458";
const SUBSYSTEM_DEVICE: &str = "0x40c8";
const I2C_ADDRESS: u16 = 0x64;
const I2C_RDWR: libc::c_ulong = 0x0707;

const MODE_STATIC: u8 = 0x01;
const MODE_RAINBOW_WAVE: u8 = 0x07;
const CAPTURED_SPEED: u8 = 0x05;
const CAPTURED_BRIGHTNESS: u8 = 0x63;

#[repr(C)]
struct I2cMsg {
    addr: u16,
    flags: u16,
    len: u16,
    buf: *mut u8,
}

#[repr(C)]
struct I2cRdwrIoctlData {
    msgs: *mut I2cMsg,
    nmsgs: u32,
}

/// One logical, combined RGB target. Vendor selectors 1 and 2 are always
/// written together; their independent physical meanings are not exposed.
pub struct GigabyteGpuRgb {
    device_id: String,
    model: String,
    adapter_path: PathBuf,
    transaction_lock: Mutex<()>,
}

impl GigabyteGpuRgb {
    fn new(bdf: &str, adapter_path: PathBuf) -> Self {
        Self {
            device_id: format!("pci:{bdf}:10de:2704:1458:40c8"),
            model: "GIGABYTE AORUS RTX 4080 XTREME WATERFORCE WB (GV-N4080AORUSX WB-16GD)".into(),
            adapter_path,
            transaction_lock: Mutex::new(()),
        }
    }

    fn write_frames(&self, frames: &[[u8; 8]]) -> Result<()> {
        anyhow::ensure!(!frames.is_empty(), "empty GPU RGB transaction sequence");
        let _guard = self
            .transaction_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("GPU RGB transaction lock poisoned"))?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.adapter_path)
            .with_context(|| format!("opening GPU I²C adapter {}", self.adapter_path.display()))?;
        // Serialize this controller's requests and hold an advisory per-device
        // lock against other cooperative Lian Li controller instances.
        let lock_result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
        if lock_result != 0 {
            return Err(std::io::Error::last_os_error()).context("locking GPU RGB I²C adapter");
        }
        let transfer_result = (|| {
            for frame in frames {
                let mut msg = I2cMsg {
                    addr: I2C_ADDRESS,
                    flags: 0,
                    len: frame.len() as u16,
                    buf: frame.as_ptr() as *mut u8,
                };
                let mut transfer = I2cRdwrIoctlData {
                    msgs: &mut msg,
                    nmsgs: 1,
                };
                let result = unsafe { libc::ioctl(file.as_raw_fd(), I2C_RDWR, &mut transfer) };
                if result < 0 {
                    return Err(std::io::Error::last_os_error()).with_context(|| {
                        format!(
                            "I2C_RDWR 8-byte write to GPU RGB address 0x{I2C_ADDRESS:02x} on {}",
                            self.adapter_path.display()
                        )
                    });
                }
                anyhow::ensure!(
                    result == 1,
                    "GPU RGB I²C write returned {result} completed messages; expected 1"
                );
            }
            Ok(())
        })();
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        transfer_result
    }

    fn mode_frame(mode: u8, selector: u8) -> [u8; 8] {
        [
            0x88,
            mode,
            CAPTURED_SPEED,
            CAPTURED_BRIGHTNESS,
            0,
            selector,
            0,
            0,
        ]
    }

    fn color_frame(color: [u8; 3]) -> [u8; 8] {
        [0x40, color[0], color[1], color[2], 0x05, 0, 0, 0]
    }

    fn set_mode(&self, mode: u8) -> Result<()> {
        self.write_frames(&[Self::mode_frame(mode, 1), Self::mode_frame(mode, 2)])
    }

    fn apply_effect(&self, effect: &RgbEffect) -> Result<()> {
        if effect.disabled {
            bail!("GPU RGB off mode is not characterized");
        }
        let result = match effect.mode {
            RgbMode::Static => {
                let color = effect.colors.first().copied().unwrap_or([255, 255, 255]);
                self.write_frames(&[
                    Self::mode_frame(MODE_STATIC, 1),
                    Self::mode_frame(MODE_STATIC, 2),
                    Self::color_frame(color),
                ])
            }
            RgbMode::RainbowWave => self.set_mode(MODE_RAINBOW_WAVE),
            unsupported => bail!("GPU RGB mode {unsupported:?} is not characterized"),
        };
        result?;
        tracing::info!(
            device = %self.device_id,
            adapter = %self.adapter_path.display(),
            address = format_args!("0x{I2C_ADDRESS:02x}"),
            mode = ?effect.mode,
            "Applied captured Gigabyte GPU RGB effect"
        );
        Ok(())
    }
}

impl RgbDevice for GigabyteGpuRgb {
    fn device_name(&self) -> String {
        self.model.clone()
    }
    fn supported_modes(&self) -> Vec<RgbMode> {
        vec![RgbMode::Static, RgbMode::RainbowWave]
    }
    fn zone_info(&self) -> Vec<RgbZoneInfo> {
        vec![RgbZoneInfo {
            name: "GPU Lighting".into(),
            name_ui: None,
            led_count: 1,
        }]
    }
    fn set_zone_effect(&self, zone: u8, effect: &RgbEffect) -> Result<()> {
        anyhow::ensure!(zone == 0, "GPU exposes one combined lighting target");
        self.apply_effect(effect)
    }
    fn set_all_effects(&self, effect: &RgbEffect) -> Result<()> {
        self.apply_effect(effect)
    }
}

/// Discover only NVIDIA port 1 attached to the exact board subsystem.
/// Linux i2c-N numbering is intentionally not stored as identity.
pub fn discover() -> Result<Vec<(String, std::sync::Arc<dyn RgbDevice>)>> {
    let pci_root = Path::new("/sys/bus/pci/devices");
    let i2c_root = Path::new("/sys/bus/i2c/devices");
    let mut found = Vec::new();
    for entry in fs::read_dir(pci_root).context("enumerating PCI devices for Gigabyte GPU RGB")? {
        let entry = entry?;
        let bdf = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !read_trimmed(path.join("vendor"))?.eq_ignore_ascii_case(GPU_VENDOR)
            || !read_trimmed(path.join("device"))?.eq_ignore_ascii_case(GPU_DEVICE)
            || !read_trimmed(path.join("subsystem_vendor"))?.eq_ignore_ascii_case(SUBSYSTEM_VENDOR)
            || !read_trimmed(path.join("subsystem_device"))?.eq_ignore_ascii_case(SUBSYSTEM_DEVICE)
        {
            continue;
        }
        let pci_path = fs::canonicalize(&path).context("resolving exact GPU PCI path")?;

        let adapter_path = find_port1_adapter(&bdf, &pci_path, i2c_root)?
            .ok_or_else(|| anyhow::anyhow!("exact Gigabyte RTX 4080 found at {bdf}, but its NVIDIA port 1 I²C adapter is unavailable"))?;
        let device = std::sync::Arc::new(GigabyteGpuRgb::new(&bdf, adapter_path));
        found.push((
            device.device_id.clone(),
            device as std::sync::Arc<dyn RgbDevice>,
        ));
    }
    Ok(found)
}

fn find_port1_adapter(bdf: &str, pci_path: &Path, i2c_root: &Path) -> Result<Option<PathBuf>> {
    for bus in fs::read_dir(i2c_root).context("enumerating Linux I²C buses")? {
        let bus = bus?;
        let name = bus.file_name().to_string_lossy().into_owned();
        if !name.starts_with("i2c-") {
            continue;
        }
        let bus_path = bus.path();
        let canonical = fs::canonicalize(&bus_path).context("resolving I²C sysfs adapter")?;
        if !canonical.starts_with(pci_path) {
            continue;
        }
        let adapter_name = read_trimmed(bus_path.join("name"))?;
        let expected = nvidia_adapter_name(bdf, 1)?;
        if adapter_name == expected {
            return Ok(Some(PathBuf::from("/dev").join(name)));
        }
    }
    Ok(None)
}

fn nvidia_adapter_name(bdf: &str, port: u8) -> Result<String> {
    let (_, address) = bdf
        .split_once(':')
        .context("invalid PCI BDF (missing domain separator)")?;
    let (bus, slot_function) = address
        .split_once(':')
        .context("invalid PCI BDF (missing bus separator)")?;
    let (slot, function) = slot_function
        .split_once('.')
        .context("invalid PCI BDF (missing function separator)")?;
    let bus = u8::from_str_radix(bus, 16).context("parsing PCI bus number")?;
    let slot = u8::from_str_radix(slot, 16).context("parsing PCI slot number")?;
    let function = u8::from_str_radix(function, 16).context("parsing PCI function number")?;
    Ok(format!(
        "NVIDIA i2c adapter {port} at {bus:x}:{slot:02x}.{function:x}"
    ))
}

fn read_trimmed(path: PathBuf) -> Result<String> {
    fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))
        .map(|s| s.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_mode_packets_are_exact_eight_byte_frames() {
        assert_eq!(
            GigabyteGpuRgb::mode_frame(MODE_STATIC, 1),
            [0x88, 0x01, 0x05, 0x63, 0, 1, 0, 0]
        );
        assert_eq!(
            GigabyteGpuRgb::mode_frame(MODE_STATIC, 2),
            [0x88, 0x01, 0x05, 0x63, 0, 2, 0, 0]
        );
        assert_eq!(
            GigabyteGpuRgb::mode_frame(MODE_RAINBOW_WAVE, 1),
            [0x88, 0x07, 0x05, 0x63, 0, 1, 0, 0]
        );
        assert_eq!(
            GigabyteGpuRgb::mode_frame(MODE_RAINBOW_WAVE, 2),
            [0x88, 0x07, 0x05, 0x63, 0, 2, 0, 0]
        );
    }

    #[test]
    fn exact_subsystem_match_is_encoded_in_discovery_constants() {
        assert_eq!(
            (GPU_VENDOR, GPU_DEVICE, SUBSYSTEM_VENDOR, SUBSYSTEM_DEVICE),
            ("0x10de", "0x2704", "0x1458", "0x40c8")
        );
    }

    #[test]
    fn adapter_label_matches_nvidia_driver_pci_format() {
        assert_eq!(
            nvidia_adapter_name("0000:01:00.0", 1).unwrap(),
            "NVIDIA i2c adapter 1 at 1:00.0"
        );
    }

    #[test]
    fn unsupported_modes_fail_closed() {
        let device = GigabyteGpuRgb::new("0000:01:00.0", PathBuf::from("/dev/null"));
        let effect = RgbEffect {
            mode: RgbMode::Breathing,
            ..RgbEffect::default()
        };
        let error = device.apply_effect(&effect).unwrap_err().to_string();
        assert!(error.contains("not characterized"));
    }

    #[test]
    fn captured_static_red_color_frame_is_exact() {
        assert_eq!(
            GigabyteGpuRgb::color_frame([0xcc, 0, 0]),
            [0x40, 0xcc, 0, 0, 0x05, 0, 0, 0]
        );
    }

    #[test]
    fn transport_failures_are_reported_with_transfer_context() {
        let device = GigabyteGpuRgb::new("0000:01:00.0", PathBuf::from("/dev/null"));
        let error = device.write_frames(&[[0; 8]]).unwrap_err().to_string();
        assert!(error.contains("I2C_RDWR 8-byte write"));
        assert!(error.contains("0x64"));
    }

    #[test]
    fn discovery_selects_only_driver_port_one_below_the_exact_gpu() {
        let temp =
            std::env::temp_dir().join(format!("gigabyte-i2c-discovery-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let pci = temp.join("pci/0000:01:00.0");
        let i2c = temp.join("i2c");
        fs::create_dir_all(&pci).unwrap();
        fs::create_dir_all(&i2c).unwrap();
        let physical_bus = pci.join("i2c-12");
        fs::create_dir_all(&physical_bus).unwrap();
        fs::write(
            physical_bus.join("name"),
            "NVIDIA i2c adapter 1 at 1:00.0\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(&physical_bus, i2c.join("i2c-12")).unwrap();
        let selected = find_port1_adapter("0000:01:00.0", &pci, &i2c).unwrap();
        assert_eq!(selected, Some(PathBuf::from("/dev/i2c-12")));
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn live_discovery_registers_only_the_exact_installed_gpu() {
        let pci = Path::new("/sys/bus/pci/devices/0000:01:00.0");
        if !pci.exists()
            || !read_trimmed(pci.join("vendor")).is_ok_and(|value| value == GPU_VENDOR)
            || !read_trimmed(pci.join("device")).is_ok_and(|value| value == GPU_DEVICE)
            || !read_trimmed(pci.join("subsystem_vendor"))
                .is_ok_and(|value| value == SUBSYSTEM_VENDOR)
            || !read_trimmed(pci.join("subsystem_device"))
                .is_ok_and(|value| value == SUBSYSTEM_DEVICE)
        {
            return;
        }
        let devices = discover().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].0, "pci:0000:01:00.0:10de:2704:1458:40c8");
        assert_eq!(
            devices[0].1.device_name(),
            "GIGABYTE AORUS RTX 4080 XTREME WATERFORCE WB (GV-N4080AORUSX WB-16GD)"
        );
    }
}

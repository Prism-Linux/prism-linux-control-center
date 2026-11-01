use anyhow::{ensure, Context, Result};
use hidapi::{DeviceInfo, HidApi};
use lianli_devices::asus_aura::{
    parse_configuration, parse_firmware, read_response, validate_device_identity, Query,
    FRAME_LENGTH, PRODUCT_ID, VENDOR_ID,
};
use lianli_transport::{HidTransport, HidrawTransport};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

const AURA_USAGE_PAGE: u16 = 0xFF72;

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

fn report_descriptor(path: &std::path::Path) -> Result<Vec<u8>> {
    let name = path
        .file_name()
        .context("HID path has no device name")?
        .to_string_lossy();
    std::fs::read(format!("/sys/class/hidraw/{name}/device/report_descriptor"))
        .context("reading HID report descriptor from sysfs")
}

fn print_frame(label: &str, frame: &[u8]) {
    let raw = frame
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{label} ({}) = {raw}", frame.len());
}

fn main() -> Result<()> {
    let api = HidApi::new().context("initializing HIDAPI")?;
    let candidates: Vec<_> = api
        .device_list()
        .filter(|info| info.vendor_id() == VENDOR_ID && info.product_id() == PRODUCT_ID)
        .collect();
    ensure!(
        !candidates.is_empty(),
        "ASUS Aura USB device {VENDOR_ID:04x}:{PRODUCT_ID:04x} was not found"
    );
    let info = select_interface(&candidates)?;
    validate_device_identity(info.vendor_id(), info.product_id())?;
    let path = info.path();
    let path_os = OsStr::from_bytes(path.to_bytes()).to_os_string();
    println!(
        "Selected ASUS Aura {:04x}:{:04x}, interface {}, usage page 0x{:04x}, usage 0x{:04x}, path {}",
        info.vendor_id(),
        info.product_id(),
        info.interface_number(),
        info.usage_page(),
        info.usage(),
        path.to_string_lossy()
    );
    if let Some(product) = info.product_string() {
        println!("Product: {product}");
    }
    let descriptor = report_descriptor(std::path::Path::new(&path_os))?;
    println!(
        "HID report descriptor: {} bytes; {:02X?}",
        descriptor.len(),
        descriptor
    );

    let device = api
        .open_path(path)
        .context("opening ASUS Aura HID interface as the current user")?;
    let mut transport = HidrawTransport::new(device, Some(path_os));

    for query in [Query::Firmware, Query::Configuration] {
        let request = query.frame();
        let written = transport
            .write_timeout(&request, Duration::from_secs(1))
            .with_context(|| format!("writing ASUS Aura {query:?} inquiry"))?;
        ensure!(
            written == FRAME_LENGTH,
            "short ASUS Aura {query:?} inquiry write: {written}/{FRAME_LENGTH}"
        );
        print_frame(&format!("TX {query:?}"), &request);
        let response = read_response(&mut transport, query)
            .with_context(|| format!("reading ASUS Aura {query:?} response"))?;
        print_frame(&format!("RX {query:?}"), &response);
        match query {
            Query::Firmware => println!("Firmware: {}", parse_firmware(&response)?),
            Query::Configuration => {
                let config = parse_configuration(&response)?;
                println!(
                    "Configuration: channels reported={}, channel records parsed={}, raw LED counts={:?}, mainboard LED count={}",
                    config.reported_channel_count,
                    config.parsed_channel_count,
                    config.reported_led_counts,
                    config.mainboard_led_count
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_uses_vendor_usage_page_to_choose_aura_interface() {
        let api = HidApi::new().unwrap();
        let selected: Vec<_> = api
            .device_list()
            .filter(|info| info.vendor_id() == VENDOR_ID && info.product_id() == PRODUCT_ID)
            .collect();
        if selected.is_empty() {
            return;
        }
        let info = select_interface(&selected).unwrap();
        assert_eq!(info.usage_page(), AURA_USAGE_PAGE);
    }
}

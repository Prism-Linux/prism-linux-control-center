//! Process-wide libusb context that can be replaced at runtime.
//!
//! libusb fills its device list once at init and then follows udev events.
//! Some containers never deliver those events, so a fresh context is the only
//! way to see re-enumerated devices without restarting the process. Devices and
//! handles keep the context they came from alive, so a rescan leaves open
//! transports untouched.

use rusb::{Context, DeviceList, UsbContext as _};
use std::sync::{PoisonError, RwLock};

pub type UsbContext = Context;

static CURRENT: RwLock<Option<UsbContext>> = RwLock::new(None);

pub fn current() -> rusb::Result<UsbContext> {
    if let Some(context) = CURRENT
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        return Ok(context.clone());
    }
    let mut slot = CURRENT.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(context) = slot.as_ref() {
        return Ok(context.clone());
    }
    let context = Context::new()?;
    *slot = Some(context.clone());
    Ok(context)
}

pub fn devices() -> rusb::Result<DeviceList<UsbContext>> {
    current()?.devices()
}

/// Replaces the shared context with a newly initialised one, which rescans the
/// bus. On failure the previous context stays in place.
pub fn rescan() -> rusb::Result<UsbContext> {
    let context = Context::new()?;
    let previous = CURRENT
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .replace(context.clone());
    drop(previous);
    Ok(context)
}

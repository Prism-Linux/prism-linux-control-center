//! RGB-related IPC handlers: `GetRgbCapabilities`, `SetRgbEffect`,
//! `SetRgbDirect`, `SetRgbFrames`, `SetMbRgbSync`, `SetFanDirection`,
//! `SetLedColor`, `GetZoneColors`.

use lianli_shared::ipc::IpcResponse;
use lianli_shared::rgb::RgbEffect;
use lianli_shared::ui_message::UiMessage;

use crate::ipc::{rgb_controller_missing, SharedState};

fn failed(key: &str, label: &str, error: impl std::fmt::Display) -> IpcResponse {
    IpcResponse::error_ui(
        format!("{label}: {error}"),
        UiMessage::new(key).param("error", error),
    )
}

fn invalid_config(error: impl std::fmt::Display) -> IpcResponse {
    failed(
        "daemon.rgb.invalidConfig",
        "Invalid RGB configuration",
        error,
    )
}

fn zone_not_found(zone: u8, device_id: &str) -> IpcResponse {
    IpcResponse::error_ui(
        format!("zone {zone} not found on device {device_id}"),
        UiMessage::new("daemon.rgb.zoneNotFound")
            .param("zone", zone)
            .param("device", device_id),
    )
}

pub fn validate_saved_config(
    state: &SharedState,
    config: &lianli_shared::rgb::RgbAppConfig,
) -> Option<IpcResponse> {
    if let Err(error) = lianli_shared::rgb::validate_effect_memory(config) {
        return Some(invalid_config(error));
    }
    if state
        .lock()
        .config
        .as_ref()
        .and_then(|saved| saved.rgb.as_ref())
        == Some(config)
    {
        return None;
    }
    validate_config(state, config)
}

pub fn validate_config(
    state: &SharedState,
    config: &lianli_shared::rgb::RgbAppConfig,
) -> Option<IpcResponse> {
    if let Err(error) = lianli_shared::rgb::validate_effect_memory(config) {
        return Some(invalid_config(error));
    }
    let controller = state.lock().rgb_controller.clone()?;
    let result = controller.lock().validate_config(config);
    result.err().map(invalid_config)
}

pub fn capabilities(state: &SharedState) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        let caps = rgb.lock().capabilities();
        IpcResponse::ok(&caps)
    } else {
        IpcResponse::ok(serde_json::json!([]))
    }
}

pub fn set_effect(
    state: &SharedState,
    device_id: String,
    zone: u8,
    effect: RgbEffect,
) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        match rgb.lock().set_effect(&device_id, zone, &effect) {
            Ok(()) => IpcResponse::ok(serde_json::json!(null)),
            Err(e) => failed("daemon.rgb.effectFailed", "RGB effect error", e),
        }
    } else {
        rgb_controller_missing()
    }
}

pub fn set_direct(
    state: &SharedState,
    device_id: String,
    zone: u8,
    colors: Vec<[u8; 3]>,
) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        match rgb.lock().set_direct_colors(&device_id, zone, &colors) {
            Ok(()) => IpcResponse::ok(serde_json::json!(null)),
            Err(e) => failed("daemon.rgb.directFailed", "RGB direct error", e),
        }
    } else {
        rgb_controller_missing()
    }
}

pub fn set_frames(
    state: &SharedState,
    device_id: String,
    frames: Vec<Vec<[u8; 3]>>,
    interval_ms: u16,
) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        match rgb.lock().set_rgb_frames(&device_id, &frames, interval_ms) {
            Ok(()) => IpcResponse::ok(serde_json::json!(null)),
            Err(e) => failed("daemon.rgb.framesFailed", "RGB frames error", e),
        }
    } else {
        rgb_controller_missing()
    }
}

pub fn set_mb_sync(state: &SharedState, device_id: String, enabled: bool) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        match rgb.lock().set_mb_rgb_sync(&device_id, enabled) {
            Ok(()) => IpcResponse::ok(serde_json::json!(null)),
            Err(e) => failed("daemon.rgb.mbSyncFailed", "MB RGB sync error", e),
        }
    } else {
        rgb_controller_missing()
    }
}

pub fn set_fan_direction(
    state: &SharedState,
    device_id: String,
    zone: u8,
    swap_lr: bool,
    swap_tb: bool,
) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        match rgb
            .lock()
            .set_fan_direction(&device_id, zone, swap_lr, swap_tb)
        {
            Ok(()) => IpcResponse::ok(serde_json::json!(null)),
            Err(e) => failed("daemon.rgb.fanDirectionFailed", "Fan direction error", e),
        }
    } else {
        rgb_controller_missing()
    }
}

pub fn set_led_color(
    state: &SharedState,
    device_id: String,
    zone: u8,
    led_index: u16,
    color: [u8; 3],
) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        let mut rgb = rgb.lock();
        let mut colors = match rgb.get_zone_colors(&device_id, zone) {
            Some(c) => c,
            None => return zone_not_found(zone, &device_id),
        };
        let idx = led_index as usize;
        if idx >= colors.len() {
            return IpcResponse::error_ui(
                format!(
                    "LED index {led_index} out of range (zone has {} LEDs)",
                    colors.len()
                ),
                UiMessage::new("daemon.rgb.ledIndexOutOfRange")
                    .param("index", led_index)
                    .param("count", colors.len()),
            );
        }
        colors[idx] = color;
        match rgb.set_direct_colors(&device_id, zone, &colors) {
            Ok(()) => IpcResponse::ok(serde_json::json!(null)),
            Err(e) => failed("daemon.rgb.directFailed", "RGB direct error", e),
        }
    } else {
        rgb_controller_missing()
    }
}

pub fn get_zone_colors(state: &SharedState, device_id: String, zone: u8) -> IpcResponse {
    let controller = state.lock().rgb_controller.clone();
    if let Some(rgb) = controller {
        let rgb = rgb.lock();
        match rgb.get_zone_colors(&device_id, zone) {
            Some(colors) => IpcResponse::ok(&colors),
            None => zone_not_found(zone, &device_id),
        }
    } else {
        rgb_controller_missing()
    }
}

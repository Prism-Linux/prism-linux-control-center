//! IPC layer: Unix domain socket server + per-concern request handlers.
//!
//! The server ([`server`]) accepts connections, deserializes [`IpcRequest`]s,
//! and dispatches to one of the handler submodules:
//!
//! - [`system`] — read-only queries: ping, sensor/device enumeration, telemetry.
//! - [`config`] — config-writing handlers (`SetConfig`, `SetLcdMedia`, etc.).
//! - [`fan`] — fan-specific handlers (`SetEne6k77FanQuantity`, fan direction).
//! - [`rgb`] — RGB effect / direct-color / zone queries.
//! - [`lcd`] — display-mode switching, template preview rendering.
//! - [`wireless`] — bind / unbind RF devices.
//! - [`templates`] — LCD template CRUD.
//! - [`presets`] — RGB preset save / load / delete / apply.

mod backups;
mod catalog_cleanup;
mod event_sender;
mod installation;
mod server;
mod service_stop;

pub mod catalog;
pub mod config;
pub mod fan;
pub mod lcd;
pub mod presets;
pub mod profiles;
pub mod rgb;
pub mod system;
pub mod templates;
pub mod wireless;

pub(crate) use event_sender::EventSender;
pub use server::{build_info, start_ipc_server, DaemonState, PixelCleanState};

use lianli_shared::ipc::IpcResponse;
use lianli_shared::ui_message::UiMessage;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::service::DaemonEvent;

pub(crate) use crate::persistence::{write_config, write_rgb_presets};

/// Type alias for the shared state reference handlers receive.
pub(crate) type SharedState = Arc<Mutex<DaemonState>>;

pub(crate) fn persist_and_notify(
    state: &mut DaemonState,
    tx: &EventSender,
    label: &str,
    config: lianli_shared::config::AppConfig,
) -> IpcResponse {
    use tracing::info;
    match write_config(&state.config_path, &config) {
        Ok(()) => {
            state.config = Some(config);
            let _ = tx.send(DaemonEvent::IpcUpdate);
            info!("{label}: config persisted, notified daemon");
            IpcResponse::ok(serde_json::json!(null))
        }
        Err(e) => config_write_failed(e),
    }
}

/// Like `IpcResponse::from_error`, but with only the outermost English layer.
pub(crate) fn outer_error(error: &anyhow::Error) -> IpcResponse {
    IpcResponse::Error {
        message: error.to_string(),
        ui: lianli_shared::ui_message::ui_message_of(error),
    }
}

pub(crate) fn config_write_failed(error: impl std::fmt::Display) -> IpcResponse {
    IpcResponse::error_ui(
        format!("failed to write config: {error}"),
        UiMessage::new("daemon.config.writeFailed").param("error", error),
    )
}

pub(crate) fn no_config_loaded() -> IpcResponse {
    IpcResponse::error_ui(
        "no config loaded",
        UiMessage::new("daemon.config.notLoaded"),
    )
}

pub(crate) fn rgb_controller_missing() -> IpcResponse {
    IpcResponse::error_ui(
        "RGB controller not initialized",
        UiMessage::new("daemon.rgb.controllerMissing"),
    )
}

pub(crate) fn service_unavailable() -> IpcResponse {
    IpcResponse::error_ui(
        "daemon service not running",
        UiMessage::new("daemon.request.serviceUnavailable"),
    )
}

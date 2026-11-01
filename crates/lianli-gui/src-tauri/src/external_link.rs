use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::command_error::CommandError;
use gio::glib::{ToVariant, Variant, VariantDict, VariantTy};
use lianli_control::services::host_spawn_bridge;
use lianli_shared::installation::InstallationContext;
use lianli_shared::ui_message::UiMessage;

const EXIT_WAIT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const PORTAL_TIMEOUT_MS: i32 = 5_000;

pub fn open(url: &str) -> Result<(), CommandError> {
    validate(url)?;
    let context = InstallationContext::detect();
    if matches!(context, InstallationContext::Native) {
        return open_with_xdg(&context, url);
    }
    // Host commands from a container inherit the host's systemd user
    // environment, which can lack the desktop and Flatpak paths xdg-open needs.
    let portal_error = match open_with_portal(url) {
        Ok(()) => return Ok(()),
        Err(error) => error,
    };
    open_with_xdg(&context, url).map_err(|error| {
        CommandError::ui(
            format!("Desktop portal failed ({portal_error}); {}", error.message),
            UiMessage::new("gui.externalLink.portalFailed")
                .param("portal", &portal_error)
                .param("error", &error.message),
        )
    })
}

fn open_with_portal(url: &str) -> Result<(), String> {
    let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
        .map_err(|error| format!("no session bus: {error}"))?;
    bus.call_sync(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.OpenURI",
        "OpenURI",
        Some(&portal_arguments(url)),
        Some(VariantTy::new("(o)").expect("valid reply type")),
        gio::DBusCallFlags::NONE,
        PORTAL_TIMEOUT_MS,
        gio::Cancellable::NONE,
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn portal_arguments(url: &str) -> Variant {
    Variant::tuple_from_iter([
        "".to_variant(),
        url.to_variant(),
        VariantDict::new(None).end(),
    ])
}

fn open_with_xdg(context: &InstallationContext, url: &str) -> Result<(), CommandError> {
    let mut command = launcher(context, url, host_spawn_bridge)?;
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            CommandError::ui(
                format!("Cannot start xdg-open: {error}"),
                UiMessage::new("gui.externalLink.startFailed").param("error", error),
            )
        })?;
    wait_for_launcher(child)
}

fn validate(url: &str) -> Result<(), CommandError> {
    if !url.starts_with("https://") || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(CommandError::ui(
            "Only https links can be opened",
            UiMessage::new("gui.externalLink.httpsOnly"),
        ));
    }
    Ok(())
}

fn launcher(
    context: &InstallationContext,
    url: &str,
    bridge: impl FnOnce() -> Option<PathBuf>,
) -> Result<Command, CommandError> {
    let mut command = match context {
        InstallationContext::Distrobox { .. } => {
            let bridge = bridge().ok_or_else(|| {
                CommandError::ui(
                    "host-spawn is not installed in this box. Run distrobox-host-exec true once inside the box, then try again",
                    UiMessage::new("gui.externalLink.hostSpawnMissing"),
                )
            })?;
            let mut command = Command::new(bridge);
            command.arg("xdg-open");
            command
        }
        InstallationContext::Native | InstallationContext::UnsupportedContainer => {
            Command::new("xdg-open")
        }
    };
    command.arg(url);
    Ok(command)
}

fn wait_for_launcher(mut child: Child) -> Result<(), CommandError> {
    let deadline = Instant::now() + EXIT_WAIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(CommandError::ui(
                    format!("xdg-open failed ({status})"),
                    UiMessage::new("gui.externalLink.launcherFailed").param("status", status),
                ))
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                std::thread::spawn(move || child.wait());
                return Ok(());
            }
            Err(error) => {
                return Err(CommandError::ui(
                    format!("Cannot check xdg-open: {error}"),
                    UiMessage::new("gui.externalLink.checkFailed").param("error", error),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://example.com";

    #[test]
    fn only_plain_https_links_are_opened() {
        assert!(validate("https://github.com/Prism-Linux/prism-linux-control-center").is_ok());
        assert!(validate("http://example.com").is_err());
        assert!(validate("file:///etc/passwd").is_err());
        assert!(validate("https://example.com/a b").is_err());
        assert!(validate("https://example.com/\n--help").is_err());
    }

    #[test]
    fn portal_request_matches_the_open_uri_signature() {
        let arguments = portal_arguments(URL);
        assert_eq!(arguments.type_().as_str(), "(ssa{sv})");
        assert_eq!(arguments.child_value(1).str(), Some(URL));
    }

    #[test]
    fn native_runs_xdg_open_directly() {
        let command = launcher(&InstallationContext::Native, URL, || None).unwrap();
        assert_eq!(command.get_program(), "xdg-open");
        assert_eq!(command.get_args().collect::<Vec<_>>(), [URL]);
    }

    #[test]
    fn distrobox_routes_xdg_open_through_the_host_bridge() {
        let boxed = InstallationContext::Distrobox {
            name: "lianli".into(),
        };
        let command = launcher(&boxed, URL, || Some("/usr/bin/host-spawn".into())).unwrap();
        assert_eq!(command.get_program(), "/usr/bin/host-spawn");
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["xdg-open", URL]);
        assert!(launcher(&boxed, URL, || None).is_err());
    }
}

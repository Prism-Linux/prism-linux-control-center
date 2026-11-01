use lianli_shared::ui_message::{ui_message_of, UiMessage};
use serde::Serialize;

/// A command failure the frontend shows translated when `ui` is present.
#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiMessage>,
}

impl CommandError {
    pub fn ui(english: impl Into<String>, ui: UiMessage) -> Self {
        Self {
            message: english.into(),
            ui: Some(ui),
        }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self { message, ui: None }
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

impl From<anyhow::Error> for CommandError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            message: format!("{error:#}"),
            ui: ui_message_of(&error),
        }
    }
}

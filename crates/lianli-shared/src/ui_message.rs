use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A GUI message key under the `backend` namespace, with named parameters.
/// The English text always travels beside it as the fallback.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMessage {
    pub key: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

impl UiMessage {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn param(mut self, name: &str, value: impl ToString) -> Self {
        self.params.insert(name.to_string(), value.to_string());
        self
    }
}

/// An error that displays as English and carries the GUI message for it.
#[derive(Debug, Clone)]
pub struct UiError {
    pub message: UiMessage,
    english: String,
}

impl UiError {
    pub fn new(message: UiMessage, english: impl Into<String>) -> Self {
        Self {
            message,
            english: english.into(),
        }
    }
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.english)
    }
}

impl std::error::Error for UiError {}

/// The GUI message of an error whose outermost layer is keyed. An English layer
/// wrapping a keyed one would be lost in translation, so it yields none.
pub fn ui_message_of(error: &anyhow::Error) -> Option<UiMessage> {
    error
        .downcast_ref::<UiError>()
        .filter(|ui| ui.to_string() == error.to_string())
        .map(|ui| ui.message.clone())
}

/// Display text kept for a status field: the English and its GUI message, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiText {
    pub english: String,
    pub ui: Option<UiMessage>,
}

impl UiText {
    pub fn new(ui: UiMessage, english: impl Into<String>) -> Self {
        Self {
            english: english.into(),
            ui: Some(ui),
        }
    }

    /// The full English chain, with the GUI message of its outermost keyed layer.
    pub fn from_error(error: &anyhow::Error) -> Self {
        Self {
            english: format!("{error:#}"),
            ui: ui_message_of(error),
        }
    }

    pub fn truncated(mut self, max_chars: usize) -> Self {
        self.english = self.english.chars().take(max_chars).collect();
        if let Some(ui) = &mut self.ui {
            for value in ui.params.values_mut() {
                *value = value.chars().take(max_chars).collect();
            }
        }
        self
    }

    pub fn split(text: Option<Self>) -> (Option<String>, Option<UiMessage>) {
        text.map_or((None, None), |text| (Some(text.english), text.ui))
    }
}

impl From<String> for UiText {
    fn from(english: String) -> Self {
        Self { english, ui: None }
    }
}

impl From<&str> for UiText {
    fn from(english: &str) -> Self {
        english.to_string().into()
    }
}

impl From<UiError> for UiText {
    fn from(error: UiError) -> Self {
        Self {
            english: error.english,
            ui: Some(error.message),
        }
    }
}

impl fmt::Display for UiText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.english)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn outermost_message_wins_and_english_is_kept() {
        let inner = anyhow::Error::new(UiError::new(
            UiMessage::new("devices.offline"),
            "device is offline",
        ));
        let outer = Err::<(), _>(inner)
            .context(UiError::new(
                UiMessage::new("daemon.bindFailed").param("mac", "aa:bb"),
                "Binding aa:bb failed",
            ))
            .unwrap_err();
        assert_eq!(
            format!("{outer:#}"),
            "Binding aa:bb failed: device is offline"
        );
        let message = ui_message_of(&outer).unwrap();
        assert_eq!(message.key, "daemon.bindFailed");
        assert_eq!(message.params["mac"], "aa:bb");
        assert!(ui_message_of(&anyhow::anyhow!("plain")).is_none());
    }

    #[test]
    fn english_wrapper_over_a_keyed_error_stays_english() {
        let keyed = anyhow::Error::new(UiError::new(
            UiMessage::new("devices.offline"),
            "device is offline",
        ));
        let wrapped = Err::<(), _>(keyed).context("Binding failed").unwrap_err();
        assert!(ui_message_of(&wrapped).is_none());
    }
}

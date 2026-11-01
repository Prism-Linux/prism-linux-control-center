use lianli_shared::ui_message::{UiError, UiMessage, UiText};

pub fn text(key: &str, english: &str) -> UiText {
    UiText::new(UiMessage::new(key), english)
}

pub fn keyed(key: &str, english: impl Into<String>) -> UiError {
    UiError::new(UiMessage::new(key), english)
}

pub(crate) fn bounded(message: UiText) -> (String, Option<UiMessage>) {
    let message = message.truncated(2048);
    (message.english, message.ui)
}

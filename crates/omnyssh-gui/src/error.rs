//! Command error carried across the IPC boundary (tech-gui.md §4.2, i18n plan §3.7).
//! Commands return `Result<T, CommandError>`; the frontend surfaces a message.
//!
//! `code` is the preferred shape: an error the *frontend* authors a translation for
//! travels as a stable code plus its substitution values, so it renders in the
//! user's language instead of the backend's. `message` stays the fallback for the
//! many errors that are really the engine's own text (an I/O failure, an SSH
//! rejection) — those have no code, and the frontend shows them verbatim.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CommandError {
    /// A stable identifier the frontend maps to a translated string (e.g.
    /// `"unknownHost"`, `"invalidForward"`). `None` for messages with no code yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Values the code's translation interpolates (`{host}`, `{forward}`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, String>>,
    /// The English (or engine) text. Always present: the fallback when the frontend
    /// has no translation for `code`, and the whole story when there is no code.
    pub message: String,
}

impl CommandError {
    /// An error the frontend can translate: a stable `code` plus its values.
    pub fn coded(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.into()),
            params: None,
            message: message.into(),
        }
    }

    /// A coded error with substitution values, in the order given.
    pub fn with_params(
        code: impl Into<String>,
        message: impl Into<String>,
        params: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            code: Some(code.into()),
            params: Some(
                params
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            ),
            message: message.into(),
        }
    }

    /// An error with no code: the message is the whole story (engine text, an
    /// unclassified failure).
    pub fn plain(message: impl Into<String>) -> Self {
        Self {
            code: None,
            params: None,
            message: message.into(),
        }
    }
}

/// An `anyhow`/`Display` failure with no code — the common `map_err` target.
impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self::plain(message)
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self::plain(message)
    }
}

/// The host a command was asked about is not in the loaded list. Shared so the
/// message and its code stay identical across `keysetup` and `sftp` (§3.7).
pub fn unknown_host(name: &str) -> CommandError {
    CommandError::with_params(
        "unknownHost",
        format!("unknown host '{name}'"),
        [("host", name)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coded_errors_carry_code_and_params() {
        let err = CommandError::with_params("unknownHost", "unknown host 'web'", [("host", "web")]);
        assert_eq!(err.code.as_deref(), Some("unknownHost"));
        assert_eq!(err.params.as_ref().unwrap().get("host").unwrap(), "web");
        assert_eq!(err.message, "unknown host 'web'");
    }

    #[test]
    fn plain_errors_have_no_code() {
        let err = CommandError::plain("connection reset by peer");
        assert!(err.code.is_none() && err.params.is_none());
        assert_eq!(err.message, "connection reset by peer");
        assert_eq!(CommandError::from("boom").message, "boom");
    }

    /// The wire shape omits absent fields, so a coded error never ships a null the
    /// frontend would have to special-case.
    #[test]
    fn absent_fields_are_omitted_on_the_wire() {
        let plain = serde_json::to_value(CommandError::plain("oops")).unwrap();
        assert!(plain.get("code").is_none() && plain.get("params").is_none());

        let coded =
            serde_json::to_value(CommandError::coded("unknownHost", "unknown host")).unwrap();
        assert_eq!(coded["code"], "unknownHost");
        assert!(coded.get("params").is_none());
    }
}

//! Saved host secrets (credentials-adaptation.md §7): set, clear, and locate
//! the login password of a host. The password never crosses back out — the
//! commands only report *where* it lives, so the frontend can label it.

use omnyssh_core::secrets;
use omnyssh_core::utils::platform;

use crate::error::CommandError;

/// Resolves the app config dir, mapped to a command error.
fn config_dir() -> Result<std::path::PathBuf, CommandError> {
    platform::app_config_dir()
        .ok_or_else(|| CommandError::plain("cannot determine the app config directory"))
}

/// Saves (`Some`) or clears (`None`) the login password of `host_name`.
/// Returns where the secret now lives (`"keyring"` / `"file"`) or `None`
/// after a clear. Clearing is best effort: an absent entry is success, a real
/// backend failure surfaces here.
///
/// This only touches the store — `hosts.toml` never holds passwords, and a
/// change applies from the next connection on (the host list re-reads the
/// store on reload).
#[tauri::command]
#[specta::specta]
pub async fn set_host_secret(
    host_name: String,
    password: Option<String>,
) -> Result<Option<String>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = config_dir()?;
        match password.filter(|p| !p.is_empty()) {
            Some(pw) => secrets::save_login_password(&dir, &host_name, &pw)
                .map(|backend| Some(backend.as_str().to_string()))
                .map_err(CommandError::plain),
            None => {
                secrets::delete_login_password(&dir, &host_name);
                Ok(None)
            }
        }
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("secret save task failed: {e}")))?
}

/// Reports where the host's saved login password lives, if one does:
/// `"keyring"` or `"file"`, else `None`. Read-only.
#[tauri::command]
#[specta::specta]
pub async fn host_secret_backend(host_name: String) -> Result<Option<String>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = config_dir()?;
        secrets::login_password_backend(&dir, &host_name)
            .map(|backend| backend.map(|b| b.as_str().to_string()))
            .map_err(CommandError::plain)
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("secret probe task failed: {e}")))?
}

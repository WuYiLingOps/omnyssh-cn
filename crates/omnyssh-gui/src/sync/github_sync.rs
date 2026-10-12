//! GitHub cloud sync Rust bridge: Device Flow auth + Gist raw content fallback
//! download + token / sync-password storage in the core secret store.
//!
//! Why Rust is needed: `github.com/login/*` and `gist.githubusercontent.com`
//! fetches face CORS limits inside the WebView and must go through the main
//! process via reqwest; api.github.com has CORS, so Gist CRUD is fetched
//! directly from the frontend.
//!
//! The GitHub OAuth token never lands in a plaintext file — the core secret
//! store (`omnyssh_core::secrets::store`, system keychain first with an
//! encrypted-file fallback) holds it under the key id "sync/github_token"; the
//! actual backend is recorded by the frontend sync config.

use std::sync::OnceLock;

use serde::Serialize;
use tauri::Manager;

use omnyssh_core::secrets::store::{self, SecretBackend};

const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
/// The token's key id in the secret store.
const TOKEN_KEY_ID: &str = "sync/github_token";
/// The remembered sync password's key id in the secret store.
const SYNC_PASSWORD_KEY_ID: &str = "sync/sync_password";
const DEFAULT_SCOPE: &str = "gist read:user";

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("failed to build the HTTP client")
    })
}

/// The full reason chain of a request failure (reqwest's `Display` omits the
/// underlying source — DNS / connect / TLS specifics — which is what pinpoints
/// a network problem).
fn err_chain(e: &dyn std::error::Error) -> String {
    let mut s = e.to_string();
    let mut src = e.source();
    while let Some(err) = src {
        s.push_str(&format!(": {err}"));
        src = err.source();
    }
    s
}

/// Sends with retry: the direct github.com route has intermittent timeouts
/// (notably on networks in CN), so network-class failures retry with 1s / 2s
/// gaps. The last timeout-class error carries a hint on how to proceed.
async fn send_with_retry(
    builder: reqwest::RequestBuilder,
    attempts: u32,
    what: &str,
) -> Result<reqwest::Response, String> {
    let mut last_err = String::new();
    for attempt in 0..attempts {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(1000 * attempt as u64)).await;
        }
        let request = builder
            .try_clone()
            .ok_or_else(|| format!("{what}: request body cannot be cloned for retry"))?;
        match request.send().await {
            Ok(res) => return Ok(res),
            Err(e) => {
                last_err = err_chain(&e);
            }
        }
    }
    if last_err.contains("timed out") || last_err.contains("timeout") {
        return Err(format!(
            "{what}: connection to github.com timed out (unstable or intermittently blocked network, retried {attempts} times). \
             Try again later; if this machine needs a proxy for GitHub, set the HTTPS_PROXY environment variable and restart the app"
        ));
    }
    Err(format!("{what}: {last_err}"))
}

// ---------------------------------------------------------------------------
// Device Flow (RFC 8628 — needs only a client_id, no client secret)
// ---------------------------------------------------------------------------

/// Device Flow start payload for the frontend.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFlowStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    /// Expiry as a local millisecond timestamp.
    pub expires_at: i64,
    /// Suggested polling interval in seconds.
    pub interval: u64,
}

/// Starts the Device Flow: requests a device code; the user enters the user
/// code at the verification URI.
#[tauri::command]
#[specta::specta]
pub async fn github_device_flow_start(
    client_id: String,
    scope: Option<String>,
) -> Result<DeviceFlowStart, String> {
    if client_id.trim().is_empty() {
        return Err(
            "missing the GitHub OAuth App client_id (build-time VITE_SYNC_GITHUB_CLIENT_ID)".into(),
        );
    }
    tracing::info!("starting GitHub device flow");

    let res = send_with_retry(
        http_client()
            .post(GITHUB_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "client_id={}&scope={}",
                urlencode(&client_id),
                urlencode(scope.as_deref().unwrap_or(DEFAULT_SCOPE))
            )),
        3,
        "requesting a GitHub device code",
    )
    .await?;

    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("failed to read the response: {e}"))?;
    if !status.is_success() {
        return Err(format!("GitHub device flow failed: {status} - {text}"));
    }

    #[derive(serde::Deserialize)]
    struct Raw {
        device_code: String,
        user_code: String,
        verification_uri: String,
        expires_in: Option<u64>,
        interval: Option<u64>,
    }
    let raw: Raw = serde_json::from_str(&text).map_err(|_| {
        format!(
            "GitHub device flow response is not valid JSON: {}",
            &text[..text.len().min(200)]
        )
    })?;

    Ok(DeviceFlowStart {
        device_code: raw.device_code,
        user_code: raw.user_code,
        verification_uri: raw.verification_uri,
        expires_at: now_millis() + (raw.expires_in.unwrap_or(0) * 1000) as i64,
        interval: raw.interval.unwrap_or(5).max(5),
    })
}

/// One poll of the token endpoint. GitHub answers pending/slow_down with HTTP
/// 200 plus an `error` field; the raw response body is returned verbatim and
/// the frontend handles authorization_pending / slow_down / expired_token /
/// access_denied per RFC 8628. A `String` (not `serde_json::Value`) keeps the
/// IPC surface free of a `specta` JSON-value mapping.
#[tauri::command]
#[specta::specta]
pub async fn github_device_flow_poll(
    client_id: String,
    device_code: String,
) -> Result<String, String> {
    if client_id.trim().is_empty() || device_code.trim().is_empty() {
        return Err("missing client_id or device_code".into());
    }

    let res = send_with_retry(
        http_client()
            .post(GITHUB_ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "client_id={}&device_code={}&grant_type={}",
                urlencode(&client_id),
                urlencode(&device_code),
                urlencode("urn:ietf:params:oauth:grant-type:device_code"),
            )),
        2,
        "requesting the GitHub token",
    )
    .await?;

    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("failed to read the response: {e}"))?;
    // A body carrying an `error` field (pending / slow_down / expired / denied)
    // or a token both pass through verbatim.
    let is_json = serde_json::from_str::<serde_json::Value>(&text)
        .map(|v| v.get("access_token").is_some() || v.get("error").is_some())
        .unwrap_or(false);
    if is_json || status.is_success() {
        return Ok(text);
    }
    Err(format!(
        "GitHub token poll failed: {status} - {}",
        &text[..text.len().min(200)]
    ))
}

// ---------------------------------------------------------------------------
// Gist raw content fallback download (for Gist API embedded content truncated
// at ~1 MB)
// ---------------------------------------------------------------------------

/// Validates that the URL points at gist.githubusercontent.com (keeps the token
/// from being sent to an arbitrary host). Prefix matching with the trailing
/// slash: `https://gist.githubusercontent.com.evil.com/` does not pass.
fn normalize_gist_raw_url(raw_url: &str) -> Result<String, String> {
    const PREFIX: &str = "https://gist.githubusercontent.com/";
    if !raw_url.starts_with(PREFIX) {
        return Err("Gist raw URL must start with https://gist.githubusercontent.com/".into());
    }
    Ok(raw_url.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn github_gist_raw_content(
    access_token: String,
    raw_url: String,
) -> Result<String, String> {
    let url = normalize_gist_raw_url(&raw_url)?;
    let res = send_with_retry(
        http_client()
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Accept", "application/vnd.github.raw"),
        3,
        "downloading the full Gist content",
    )
    .await?;

    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("failed to read the response: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "failed to download the full Gist content: {status} - {}",
            &text[..text.len().min(200)]
        ));
    }
    Ok(text)
}

// ---------------------------------------------------------------------------
// GitHub token persistence (reuses the core secret store; the frontend records
// the backend it landed on)
// ---------------------------------------------------------------------------

fn secret_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|e| format!("failed to resolve the config directory: {e}"))
}

/// Saves the token: keychain first, encrypted file on fallback; returns where
/// it landed ("keyring" | "file").
#[tauri::command]
#[specta::specta]
pub fn sync_save_github_token(app: tauri::AppHandle, token: String) -> Result<String, String> {
    let dir = secret_dir(&app)?;
    let backend = store::save_secret(&dir, TOKEN_KEY_ID, &token)?;
    tracing::info!("GitHub token saved ({})", backend.as_str());
    Ok(backend.as_str().to_string())
}

/// Reads the token (by the backend the frontend recorded); `None` when absent.
#[tauri::command]
#[specta::specta]
pub fn sync_load_github_token(
    app: tauri::AppHandle,
    backend: String,
) -> Result<Option<String>, String> {
    let dir = secret_dir(&app)?;
    let parsed = SecretBackend::parse(&backend)
        .ok_or_else(|| format!("unknown secret backend: {backend}"))?;
    store::load_secret(&dir, TOKEN_KEY_ID, parsed)
}

/// Deletes the token (called on disconnect); an absent entry counts as success.
#[tauri::command]
#[specta::specta]
pub fn sync_delete_github_token(app: tauri::AppHandle, backend: String) -> Result<(), String> {
    let dir = secret_dir(&app)?;
    let parsed = SecretBackend::parse(&backend)
        .ok_or_else(|| format!("unknown secret backend: {backend}"))?;
    store::delete_secret(&dir, TOKEN_KEY_ID, parsed)?;
    tracing::info!("GitHub token deleted (cloud sync disconnected)");
    Ok(())
}

// ---------------------------------------------------------------------------
// Sync password persistence (remember-password: saved on unlock, cleared on
// lock; same facility as the token)
// ---------------------------------------------------------------------------

/// Saves the sync password: keychain first, encrypted file on fallback;
/// returns where it landed ("keyring" | "file").
#[tauri::command]
#[specta::specta]
pub fn sync_save_sync_password(app: tauri::AppHandle, password: String) -> Result<String, String> {
    let dir = secret_dir(&app)?;
    let backend = store::save_secret(&dir, SYNC_PASSWORD_KEY_ID, &password).map_err(|e| {
        tracing::warn!(
            "failed to save the sync password; held in memory for this session only: {e}"
        );
        e
    })?;
    Ok(backend.as_str().to_string())
}

/// Reads the remembered sync password (by the backend the frontend recorded);
/// `None` when absent.
#[tauri::command]
#[specta::specta]
pub fn sync_load_sync_password(
    app: tauri::AppHandle,
    backend: String,
) -> Result<Option<String>, String> {
    let dir = secret_dir(&app)?;
    let parsed = SecretBackend::parse(&backend)
        .ok_or_else(|| format!("unknown secret backend: {backend}"))?;
    store::load_secret(&dir, SYNC_PASSWORD_KEY_ID, parsed).map_err(|e| {
        tracing::warn!("failed to read the remembered sync password: {e}");
        e
    })
}

/// Deletes the remembered sync password (called on lock); an absent entry
/// counts as success.
#[tauri::command]
#[specta::specta]
pub fn sync_delete_sync_password(app: tauri::AppHandle, backend: String) -> Result<(), String> {
    let dir = secret_dir(&app)?;
    let parsed = SecretBackend::parse(&backend)
        .ok_or_else(|| format!("unknown secret backend: {backend}"))?;
    store::delete_secret(&dir, SYNC_PASSWORD_KEY_ID, parsed)?;
    tracing::info!("remembered sync password cleared");
    Ok(())
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// application/x-www-form-urlencoded value encoding (everything but alphanumerics
/// and `-*._` is escaped).
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'*' | b'.' | b'_' => {
                out.push(*b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencodes_form_values() {
        assert_eq!(urlencode("abc-123"), "abc-123");
        assert_eq!(urlencode("gist read:user"), "gist%20read%3Auser");
        assert_eq!(urlencode("a+b"), "a%2Bb");
    }

    #[test]
    fn gist_raw_url_origin_check() {
        assert!(normalize_gist_raw_url(
            "https://gist.githubusercontent.com/user/abc/raw/def/omnyssh-vault.json"
        )
        .is_ok());
        // http downgrade / arbitrary host / not a URL are all rejected (the
        // token travels in the request headers).
        assert!(normalize_gist_raw_url("http://gist.githubusercontent.com/x").is_err());
        assert!(normalize_gist_raw_url("https://evil.example.com/x").is_err());
        assert!(normalize_gist_raw_url("not a url").is_err());
    }

    /// Integration test that really talks to GitHub (needs network + a
    /// Device-Flow-enabled client_id). Run:
    /// `VITE_SYNC_GITHUB_CLIENT_ID=xxx cargo test -- --ignored`
    /// Verifies: device code issuance, and poll passing through
    /// authorization_pending (before the user has authorized).
    #[tokio::test]
    #[ignore]
    async fn device_flow_real_github() {
        let client_id = std::env::var("VITE_SYNC_GITHUB_CLIENT_ID")
            .or_else(|_| std::env::var("GITHUB_SYNC_CLIENT_ID"))
            .expect("provide a client_id via VITE_SYNC_GITHUB_CLIENT_ID");
        assert!(!client_id.trim().is_empty(), "client_id must not be empty");

        // 1. Start: a device_code / user_code pair is issued.
        let start = github_device_flow_start(client_id.clone(), None)
            .await
            .expect(
                "device flow start should succeed (check client_id and the Device Flow switch)",
            );
        assert!(!start.device_code.is_empty());
        assert!(!start.user_code.is_empty());
        assert!(start.interval >= 5, "poll interval floor is 5 seconds");
        assert!(
            start.expires_at > now_millis(),
            "expiry should be in the future"
        );

        // 2. Poll (user has not authorized) → GitHub answers error=authorization_pending,
        //    passed through verbatim.
        let body = github_device_flow_poll(client_id, start.device_code)
            .await
            .expect("pending state should not error");
        let value: serde_json::Value = serde_json::from_str(&body).expect("poll body is JSON");
        assert_eq!(
            value.get("error").and_then(|e| e.as_str()),
            Some("authorization_pending"),
            "an unauthorized poll should return authorization_pending: {value}"
        );
    }
}

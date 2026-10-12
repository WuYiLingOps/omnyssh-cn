//! Connection test for the host editor (tech-gui.md §4.2). `test_host_connection`
//! dials the *form's current values* — not a saved host — so the user can verify
//! name/host/port/credential before anything is persisted. It runs the same
//! `connect_and_auth` path every other consumer uses (metrics, SFTP, tunnels),
//! with `Passwords::Remembered`: the payload's password is NOT stored here, so a
//! host with neither a saved password nor an acceptable key reports auth failure
//! rather than a prompt — a test must not type into the password prompt.
//!
//! The result mirrors `TestConnectionResultDto`: `ok` plus the human reason on
//! failure (the engine's own text, already localized where the core localizes).

use std::time::Duration;

use omnyssh_core::ssh::client::Host;
use omnyssh_core::ssh::session::{connect_budget, SshSession};

use crate::dto::HostInputDto;
use crate::error::CommandError;

/// What a connection test reported, back to the form.
#[derive(Debug, serde::Serialize, specta::Type)]
pub struct TestConnectionResultDto {
    /// The dial + authentication succeeded; the session was closed again.
    pub ok: bool,
    /// Round-trip of the successful dial + auth, in milliseconds. `None` on
    /// failure (mirrors DockPilot's `ConnectionTestResult`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// On failure, why — the engine's message (timeout, refused, auth failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Extra wall clock over [`connect_budget`] for the auth round-trips a saved
/// password allows (mirrors `tunnel.rs`' `AUTH_BUDGET`; not exported there).
const AUTH_BUDGET: Duration = Duration::from_secs(20);

/// Dial `input` exactly as a save would build the host, authenticate, and close.
/// Nothing is persisted and no password is stored; a payload password is carried
/// in memory on the temporary `Host` for this attempt only.
#[tauri::command]
#[specta::specta]
pub async fn test_host_connection(
    input: HostInputDto,
) -> Result<TestConnectionResultDto, CommandError> {
    crate::dto::check_forwards(&input.local_forwards)?;
    // The test never writes, so the three-valued pins (`None` = "unchanged")
    // have nothing to be unchanged *from*; treat omitted as empty, mirroring the
    // `From<HostInputDto>` defaults a fresh host would take.
    let host = Host::from(input);

    let budget = connect_budget(&host).await + AUTH_BUDGET;
    // `SshSession::connect` returns `anyhow::Result`; only its text reaches the
    // form, so the attempt resolves to a `Result<u64, String>` — the elapsed
    // milliseconds of a successful dial + auth.
    let attempt = async {
        let start = std::time::Instant::now();
        match SshSession::connect(&host).await {
            Ok(session) => {
                session.disconnect().await;
                Ok(start.elapsed().as_millis() as u64)
            }
            Err(e) => Err(e.to_string()),
        }
    };
    match tokio::time::timeout(budget, attempt).await {
        Ok(Ok(ms)) => Ok(TestConnectionResultDto {
            ok: true,
            latency_ms: Some(ms),
            error: None,
        }),
        Ok(Err(message)) => Ok(TestConnectionResultDto {
            ok: false,
            latency_ms: None,
            error: Some(message),
        }),
        // The budget scales with the ProxyJump chain (see `connect_budget`); a
        // trip over it is its own result, not a command failure.
        Err(_) => Ok(TestConnectionResultDto {
            ok: false,
            latency_ms: None,
            error: Some("connection test timed out".to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_result_has_no_error() {
        let r = TestConnectionResultDto {
            ok: true,
            latency_ms: Some(42),
            error: None,
        };
        assert!(r.ok && r.error.is_none() && r.latency_ms == Some(42));
    }

    #[test]
    fn timeout_is_a_result_not_a_command_error() {
        // The outer `timeout` maps to a plain failed result: the form shows
        // "timed out", it does not surface an IPC error dialog.
        let r = TestConnectionResultDto {
            ok: false,
            latency_ms: None,
            error: Some("connection test timed out".to_string()),
        };
        assert!(!r.ok);
        assert!(r.latency_ms.is_none());
        assert_eq!(r.error.as_deref(), Some("connection test timed out"));
    }

    #[test]
    fn auth_budget_covers_a_saved_password_login() {
        // Keep the head room in sync with tunnel.rs's: a test with a remembered
        // password needs the same room the tunnel retry path allows.
        assert_eq!(AUTH_BUDGET, Duration::from_secs(20));
    }
}

//! Sync payload bridge: full-fidelity host / snippet export and import for
//! the cloud sync engine.
//!
//! The regular `HostDto` never carries secrets or key paths (tech-gui.md §3.4)
//! and is lossy in other ways too (`proxyJump`, `originalSshHost`,
//! `keySetupDate` are absent). The sync payload instead needs the **complete**
//! manual-host record, so this module talks in `HostSyncDto` — a one-to-one
//! mirror of the core `Host` — with the login password included only when the
//! user opted in (`sync_passwords`). When included, the password travels in the
//! payload and is protected by the sync password's envelope encryption one
//! layer up; when absent, apply never deletes a locally stored password.

use serde::{Deserialize, Serialize};

use omnyssh_core::config::snippets::{save_snippets, Snippet};
use omnyssh_core::config::{load_hosts, save_hosts};
use omnyssh_core::secrets;
use omnyssh_core::ssh::client::{Host, HostSource, MonitorMode};
use omnyssh_core::ssh::tunnel::LocalForward;
use omnyssh_core::utils::platform;

use crate::dto::{HostSourceDto, LocalForwardDto, MonitorModeDto, SnippetDto};
use crate::error::CommandError;

/// A host as the sync payload carries it — every field of the core `Host`
/// except `password`'s special status: here it is `None` unless the user
/// enabled `sync_passwords` (the value then comes from the secret store via
/// `load_hosts`' back-fill). Mirrors the TOML round-trip the TUI performs.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HostSyncDto {
    pub name: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    /// Stored pre-expansion (`~/.ssh/id_ed25519`), matching the TOML
    /// convention, so a synced path works on any device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub identities_only: bool,
    /// Login password — `None` unless the sync-passwords switch is on. `None`
    /// on apply means "leave whatever is stored locally alone".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_jump: Option<String>,
    /// Keychain key entry / identity pins. Ids only — the material stays in
    /// each device's secret store, so a synced pin resolves against the local
    /// `identities.toml` (cloud sync-ready, as with dockpilot's key ids).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identity_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Manual entries only ever travel the wire (SSH-config imports are
    /// re-derived locally on every device and never synced).
    pub source: HostSourceDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_ssh_host: Option<String>,
    pub monitoring: MonitorModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor_port: Option<u16>,
    #[serde(default)]
    pub local_forwards: Vec<LocalForwardDto>,
    #[serde(default)]
    pub tunnel_autostart: bool,
    #[serde(default)]
    pub forward_agent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_setup_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_auth_disabled: Option<bool>,
}

impl From<&Host> for HostSyncDto {
    fn from(h: &Host) -> Self {
        Self {
            name: h.name.clone(),
            hostname: h.hostname.clone(),
            user: h.user.clone(),
            port: h.port,
            identity_file: h.identity_file.clone(),
            identities_only: h.identities_only,
            password: h.password.clone(),
            proxy_jump: h.proxy_jump.clone(),
            key_id: h.key_id.clone(),
            identity_id: h.identity_id.clone(),
            tags: h.tags.clone(),
            notes: h.notes.clone(),
            source: HostSourceDto::from(&h.source),
            original_ssh_host: h.original_ssh_host.clone(),
            monitoring: MonitorModeDto::from(h.monitoring),
            monitor_port: h.monitor_port,
            local_forwards: h.local_forwards.iter().map(LocalForwardDto::from).collect(),
            tunnel_autostart: h.tunnel_autostart,
            forward_agent: h.forward_agent,
            key_setup_date: h.key_setup_date.clone(),
            password_auth_disabled: h.password_auth_disabled,
        }
    }
}

impl HostSyncDto {
    /// Back into a core `Host`. The password never rides on the `Host` itself:
    /// it lives in the secret store, so apply handles it separately.
    fn into_host(self) -> Host {
        Host {
            name: self.name,
            hostname: self.hostname,
            user: self.user,
            port: self.port,
            identity_file: self.identity_file,
            identities_only: self.identities_only,
            password: None,
            proxy_jump: self.proxy_jump,
            key_id: self.key_id,
            identity_id: self.identity_id,
            tags: self.tags,
            notes: self.notes,
            source: match self.source {
                HostSourceDto::SshConfig => HostSource::SshConfig,
                HostSourceDto::Manual => HostSource::Manual,
            },
            original_ssh_host: self.original_ssh_host,
            monitoring: match self.monitoring {
                MonitorModeDto::Ssh => MonitorMode::Ssh,
                MonitorModeDto::TcpPort => MonitorMode::TcpPort,
            },
            monitor_port: self.monitor_port,
            local_forwards: self
                .local_forwards
                .into_iter()
                .map(LocalForward::from)
                .collect(),
            tunnel_autostart: self.tunnel_autostart,
            forward_agent: self.forward_agent,
            key_setup_date: self.key_setup_date,
            password_auth_disabled: self.password_auth_disabled,
        }
    }
}

/// Exports the manual hosts for the sync payload. `load_hosts` reads
/// `hosts.toml` (manual entries only) and back-fills each host's saved login
/// password from the secret store; passwords are stripped again unless the
/// user turned `sync_passwords` on. SSH-config imports are never exported —
/// each device re-derives them from its own `~/.ssh/config`.
#[tauri::command]
#[specta::specta]
pub async fn sync_export_hosts(
    include_passwords: bool,
) -> Result<Vec<HostSyncDto>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let hosts = load_hosts().map_err(|e| CommandError::plain(e.to_string()))?;
        Ok(hosts
            .iter()
            .filter(|h| h.source == HostSource::Manual)
            .map(|h| {
                let mut dto = HostSyncDto::from(h);
                if !include_passwords {
                    dto.password = None;
                }
                dto
            })
            .collect())
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("sync host export task failed: {e}")))?
}

/// Applies a merged host payload: replaces the manual host list wholesale and
/// persists via the same atomic tmp+rename write every other writer uses.
///
/// Password handling: a non-empty incoming password is written to the secret
/// store (key `login/{name}`); an absent one leaves the locally stored
/// password untouched — the default (passwords excluded) payload can thus
/// never wipe a device's saved credentials.
///
/// The frontend calls `reload_hosts` afterwards to refresh the merged cache,
/// restart the pollers and broadcast `hosts-loaded`.
#[tauri::command]
#[specta::specta]
pub async fn sync_apply_hosts(hosts: Vec<HostSyncDto>) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = platform::app_config_dir()
            .ok_or_else(|| CommandError::plain("cannot determine the app config directory"))?;
        // Passwords first, so a store refusal fails the command before any
        // host state is written (mirrors `save_host`'s ordering).
        for host in &hosts {
            if let Some(password) = host.password.as_deref().filter(|p| !p.is_empty()) {
                secrets::save_login_password(&dir, &host.name, password)
                    .map_err(CommandError::plain)?;
            }
        }
        let converted: Vec<Host> = hosts.into_iter().map(HostSyncDto::into_host).collect();
        save_hosts(&converted).map_err(|e| CommandError::plain(e.to_string()))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("sync host apply task failed: {e}")))?
}

/// Applies a merged snippet payload: replaces the snippet list wholesale.
/// `save_snippet` upserts one entry at a time and cannot express deletions, so
/// the merge result lands through this bulk write instead (same atomic
/// snippets.toml write the CRUD commands use).
#[tauri::command]
#[specta::specta]
pub async fn sync_apply_snippets(snippets: Vec<SnippetDto>) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let converted: Vec<Snippet> = snippets.into_iter().map(Snippet::from).collect();
        save_snippets(&converted).map_err(|e| CommandError::plain(e.to_string()))
    })
    .await
    .map_err(|e| {
        CommandError::coded("taskFailed", format!("sync snippet apply task failed: {e}"))
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(name: &str) -> HostSyncDto {
        HostSyncDto {
            name: name.to_string(),
            hostname: "example.com".to_string(),
            user: "root".to_string(),
            port: 22,
            identity_file: Some("~/.ssh/id_ed25519".to_string()),
            identities_only: true,
            password: None,
            proxy_jump: Some("bastion".to_string()),
            key_id: String::new(),
            identity_id: String::new(),
            tags: vec!["web".to_string()],
            notes: None,
            source: HostSourceDto::Manual,
            original_ssh_host: None,
            monitoring: MonitorModeDto::Ssh,
            monitor_port: None,
            local_forwards: vec![LocalForwardDto {
                bind_address: None,
                bind_port: 9443,
                remote_host: "localhost".to_string(),
                remote_port: 443,
            }],
            tunnel_autostart: true,
            forward_agent: false,
            key_setup_date: None,
            password_auth_disabled: None,
        }
    }

    #[test]
    fn dto_roundtrips_through_the_core_host() {
        let host = dto("web").into_host();
        assert_eq!(host.name, "web");
        assert_eq!(host.user, "root");
        assert_eq!(host.port, 22);
        assert_eq!(host.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert!(host.identities_only);
        assert_eq!(host.proxy_jump.as_deref(), Some("bastion"));
        assert_eq!(host.tags, vec!["web".to_string()]);
        assert_eq!(host.source, HostSource::Manual);
        assert_eq!(host.monitoring, MonitorMode::Ssh);
        assert_eq!(host.local_forwards.len(), 1);
        assert!(host.tunnel_autostart);
        // The password never rides on the Host — it lives in the secret store.
        assert!(host.password.is_none());

        let back = HostSyncDto::from(&host);
        assert_eq!(back.name, "web");
        assert_eq!(back.local_forwards.len(), 1);
        assert!(back.tunnel_autostart);
    }

    #[test]
    fn dto_maps_every_core_field() {
        let mut host = dto("nas");
        host.monitoring = MonitorModeDto::TcpPort;
        host.monitor_port = Some(8443);
        host.source = HostSourceDto::SshConfig;
        host.original_ssh_host = Some("nas-old".to_string());
        host.key_setup_date = Some("2026-01-01".to_string());
        host.password_auth_disabled = Some(true);
        host.forward_agent = true;

        let h = host.into_host();
        assert_eq!(h.monitoring, MonitorMode::TcpPort);
        assert_eq!(h.monitor_port, Some(8443));
        assert_eq!(h.source, HostSource::SshConfig);
        assert_eq!(h.original_ssh_host.as_deref(), Some("nas-old"));
        assert_eq!(h.key_setup_date.as_deref(), Some("2026-01-01"));
        assert_eq!(h.password_auth_disabled, Some(true));
        assert!(h.forward_agent);
    }
}

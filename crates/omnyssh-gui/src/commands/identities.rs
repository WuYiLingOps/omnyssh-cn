//! Keychain key entries and reusable login identities (identity-keychain
//! plan §六): list, upsert, and delete the contents of `identities.toml`,
//! with their reference counts. The materials — PEM, key passphrase, identity
//! password — live in the secret store and never cross this boundary; the
//! save commands take them in once, the list commands never return them.
//!
//! Deletion refuses while anything still references the entry: the error
//! carries the code `keyInUse` / `identityInUse` plus the count, which the
//! frontend renders as "referenced by {n} connections".

use omnyssh_core::config::identities::{self, Identity, KeyEntry};
use omnyssh_core::config::load_hosts;
use omnyssh_core::utils::platform;

use crate::dto::{IdentityDto, KeyEntryDto, PrivateKeyFileDto};
use crate::error::CommandError;

/// One blocking snapshot of everything the commands read: the config dir, the
/// identities file, and the manual hosts (for the reference counts).
struct Snapshot {
    file: identities::IdentitiesFile,
    hosts: Vec<omnyssh_core::ssh::client::Host>,
}

fn snapshot() -> Result<Snapshot, CommandError> {
    if platform::app_config_dir().is_none() {
        return Err(CommandError::plain(
            "cannot determine the app config directory",
        ));
    }
    Ok(Snapshot {
        file: identities::load_identities().unwrap_or_default(),
        // Unreadable hosts (broken TOML) must not brick the credentials page:
        // the counts degrade to whatever the parse recovered.
        hosts: load_hosts().unwrap_or_default(),
    })
}

impl Snapshot {
    fn key_dto(&self, entry: &KeyEntry) -> KeyEntryDto {
        KeyEntryDto {
            id: entry.id.clone(),
            label: entry.label.clone(),
            fingerprint: entry.fingerprint.clone(),
            public_key: entry.public_key.clone(),
            created_at: entry.created_at,
            ref_count: identities::key_refs(&self.hosts, &self.file.identities, &entry.id),
        }
    }

    fn identity_dto(&self, identity: &Identity) -> IdentityDto {
        IdentityDto {
            id: identity.id.clone(),
            label: identity.label.clone(),
            username: identity.username.clone(),
            key_id: identity.key_id.clone(),
            created_at: identity.created_at,
            ref_count: identities::identity_refs(&self.hosts, &identity.id),
        }
    }

    /// The coded error a refused deletion becomes. `Err(n)` from the core
    /// carries the number of references still holding the entry.
    fn in_use_err(code: &str, n: usize) -> CommandError {
        CommandError::with_params(
            code,
            format!("still referenced by {n}"),
            [("count", n.to_string())],
        )
    }
}

/// Lists the keychain key entries with their reference counts. Read-only.
#[tauri::command]
#[specta::specta]
pub async fn list_key_entries() -> Result<Vec<KeyEntryDto>, CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        let snap = snapshot()?;
        Ok(snap
            .file
            .keys
            .iter()
            .map(|k| snap.key_dto(k))
            .collect())
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("identities list task failed: {e}")))?
}

/// Creates or updates a keychain key entry. `id = None` creates; an id that
/// does not exist is an error. `pem = None` keeps the stored material (a
/// rename or a passphrase change); `passphrase = None` leaves it unchanged,
/// `Some("")` clears it. An encrypted PEM must come with its passphrase,
/// which is verified before anything is stored.
#[tauri::command]
#[specta::specta]
pub async fn save_key_entry(
    id: Option<String>,
    label: String,
    pem: Option<String>,
    passphrase: Option<String>,
) -> Result<KeyEntryDto, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let entry = identities::save_key_entry(
            id.as_deref(),
            &label,
            pem.as_deref(),
            passphrase.as_deref(),
        )
        .map_err(CommandError::plain)?;
        let snap = snapshot()?;
        Ok(snap.key_dto(&entry))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("key entry save task failed: {e}")))?
}

/// Deletes a keychain key entry, refusing while any host or identity still
/// references it (`keyInUse` + the count). With no references left, the
/// metadata and the stored materials (PEM + passphrase) go together.
#[tauri::command]
#[specta::specta]
pub async fn delete_key_entry(id: String) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let hosts = load_hosts().unwrap_or_default();
        identities::delete_key_entry(&id, &hosts)
            .map_err(|n| Snapshot::in_use_err("keyInUse", n))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("key entry delete task failed: {e}")))?
}

/// Lists the reusable login identities with their reference counts. Read-only.
#[tauri::command]
#[specta::specta]
pub async fn list_identities() -> Result<Vec<IdentityDto>, CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        let snap = snapshot()?;
        Ok(snap
            .file
            .identities
            .iter()
            .map(|i| snap.identity_dto(i))
            .collect())
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("identities list task failed: {e}")))?
}

/// Creates or updates a login identity. `id = None` creates; an unknown id is
/// an error. `password`: `None` = unchanged, `Some("")` = delete, otherwise
/// store. `key_id`: `None` = unchanged, `Some("")` = unpin (password auth),
/// `Some(id)` = pin that key entry, which must exist. An identity without a
/// key must end up with a password.
#[tauri::command]
#[specta::specta]
pub async fn save_identity(
    id: Option<String>,
    label: String,
    username: String,
    password: Option<String>,
    key_id: Option<String>,
) -> Result<IdentityDto, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let identity = identities::save_identity(
            id.as_deref(),
            &label,
            &username,
            password.as_deref(),
            key_id.as_deref(),
        )
        .map_err(CommandError::plain)?;
        let snap = snapshot()?;
        Ok(snap.identity_dto(&identity))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("identity save task failed: {e}")))?
}

/// Deletes a login identity, refusing while any host still references it
/// (`identityInUse` + the count). The stored password goes with it.
#[tauri::command]
#[specta::specta]
pub async fn delete_identity(id: String) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let hosts = load_hosts().unwrap_or_default();
        identities::delete_identity(&id, &hosts)
            .map_err(|n| Snapshot::in_use_err("identityInUse", n))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("identity delete task failed: {e}")))?
}

/// Reads a private key file off disk for import, with the entry-level checks:
/// a public key is refused, the size is bounded, the content looks like a
/// PEM. Returns the file content plus whether it is encrypted — the form asks
/// for a passphrase only then. Decoding is verified in `save_key_entry`.
#[tauri::command]
#[specta::specta]
pub async fn read_private_key_file(path: String) -> Result<PrivateKeyFileDto, CommandError> {
    tauri::async_runtime::spawn_blocking(move || match identities::read_private_key_file(&path) {
        Ok(pem) => {
            let encrypted = identities::decode_pem(&pem, None)
                .err()
                .is_some_and(|e| e.contains("encrypted"));
            Ok(PrivateKeyFileDto {
                pem,
                encrypted,
                error: None,
            })
        }
        Err(e) => Ok(PrivateKeyFileDto {
            pem: String::new(),
            encrypted: false,
            error: Some(e),
        }),
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("key file read task failed: {e}")))?
}

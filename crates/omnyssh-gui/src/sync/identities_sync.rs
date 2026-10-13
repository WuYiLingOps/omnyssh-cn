//! Sync payload bridge for SSH credentials: keychain key entries, login
//! identities, and (behind the user's opt-in switch) their secret materials —
//! the PEM, the key passphrase, the identity password.
//!
//! Metadata (`identities.toml`) always travels the wire: it is non-sensitive by
//! construction and makes a synced host's key pin resolvable on a new device.
//! The materials live in the secret store and cross the wire only when the
//! user turned `sync_credentials` on — protected by the sync password's
//! envelope encryption one layer up, the same trade as host login passwords.
//!
//! Ids are the hard part: a `KeyEntry.id` is a per-device uuid, so two devices
//! hold the same key under different ids. Export rewrites everything onto
//! **fingerprint-keyed** references (`key_fp`, derived from the key material —
//! both devices agree on it); apply maps each fingerprint back to the local
//! entry id, creating the metadata entry when this device has never seen the
//! key. Hosts carry `key_fp` / `identity_label` pins in the same vocabulary and
//! `sync_apply_hosts` resolves them against the store this command just wrote.

use omnyssh_core::config::identities::{self, Identity, KeyEntry};
use omnyssh_core::secrets;
use omnyssh_core::utils::platform;

use serde::{Deserialize, Serialize};

use crate::error::CommandError;

/// A keychain key entry as the payload carries it. `keyFp` replaces the local
/// `id` as the cross-device identity.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeySyncDto {
    pub key_fp: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub public_key: String,
    /// PEM material — present only when the sync-credentials switch is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pem: Option<String>,
    /// Key passphrase for an encrypted PEM — same opt-in as `pem`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(default)]
    pub created_at: u64,
}

/// A login identity as the payload carries it. `keyFp` is empty for password
/// auth; the password is present only under the sync-credentials switch.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentitySyncDto {
    pub label: String,
    pub username: String,
    /// Fingerprint of the bound key entry; empty = password auth.
    #[serde(default)]
    pub key_fp: String,
    /// Identity password — same opt-in as the key materials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub created_at: u64,
}

fn config_dir() -> Result<std::path::PathBuf, CommandError> {
    platform::app_config_dir()
        .ok_or_else(|| CommandError::plain("cannot determine the app config directory"))
}

fn now_secs() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

/// Exports the identities file for the sync payload. Materials are backfilled
/// from the secret store only when `include_credentials` is on; with the
/// switch off the payload carries metadata exclusively. An unreadable
/// identities file degrades to an empty export (a broken file must not brick
/// sync), mirroring the credentials page.
#[tauri::command]
#[specta::specta]
pub async fn sync_export_identities(
    include_credentials: bool,
) -> Result<(Vec<KeySyncDto>, Vec<IdentitySyncDto>), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = config_dir()?;
        let file = identities::load_identities().unwrap_or_default();

        let keys = file
            .keys
            .iter()
            .map(|k| KeySyncDto {
                key_fp: k.fingerprint.clone(),
                label: k.label.clone(),
                public_key: k.public_key.clone(),
                pem: if include_credentials {
                    secrets::load_key_pem(&dir, &k.id)
                } else {
                    None
                },
                passphrase: if include_credentials {
                    secrets::load_key_passphrase(&dir, &k.id).filter(|p| !p.is_empty())
                } else {
                    None
                },
                created_at: k.created_at,
            })
            .collect();

        let identities_out = file
            .identities
            .iter()
            .map(|i| {
                // The referenced entry's fingerprint is the portable stand-in
                // for its local id; a dangling pin maps to empty = password
                // auth, which is what apply can resolve without the key.
                let key_fp = file
                    .keys
                    .iter()
                    .find(|k| k.id == i.key_id)
                    .map(|k| k.fingerprint.clone())
                    .unwrap_or_default();
                IdentitySyncDto {
                    label: i.label.clone(),
                    username: i.username.clone(),
                    key_fp,
                    password: if include_credentials {
                        secrets::load_identity_password(&dir, &i.id)
                    } else {
                        None
                    },
                    created_at: i.created_at,
                }
            })
            .collect();

        Ok((keys, identities_out))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("identity export task failed: {e}")))?
}

/// Applies a merged identity payload. Key entries are matched by fingerprint —
/// the same key already on this device is updated in place, never duplicated;
/// a new one gets a fresh uuid. Identities are matched by label (their uuid is
/// per-device too, and the label is what hosts' `identity_label` pins cite);
/// a local identity the payload does not mention stays untouched.
///
/// With credentials present, materials are stored before the metadata file is
/// written, so a store refusal fails the command before any state changes (the
/// ordering `sync_apply_hosts` uses). Without them, an existing entry's
/// material is left exactly as it is — a metadata-only payload can never wipe
/// a device's stored key.
#[tauri::command]
#[specta::specta]
pub async fn sync_apply_identities(
    keys: Vec<KeySyncDto>,
    identities_payload: Vec<IdentitySyncDto>,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = config_dir()?;
        let mut file = identities::load_identities().unwrap_or_default();

        // Key entries: fingerprint-keyed upsert.
        for key in &keys {
            let entry = match file.keys.iter().find(|k| k.fingerprint == key.key_fp) {
                Some(existing) => {
                    let mut updated = existing.clone();
                    updated.label = key.label.clone();
                    updated.public_key = key.public_key.clone();
                    updated
                }
                None => KeyEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    label: key.label.clone(),
                    fingerprint: key.key_fp.clone(),
                    public_key: key.public_key.clone(),
                    created_at: if key.created_at > 0 {
                        key.created_at
                    } else {
                        now_secs()
                    },
                },
            };
            // Materials only when the payload carries them; otherwise whatever
            // is stored stays. PEM first, then its passphrase — a half-applied
            // pair would leave an undecryptable key.
            if let Some(pem) = &key.pem {
                secrets::save_key_pem(&dir, &entry.id, pem).map_err(CommandError::plain)?;
                match &key.passphrase {
                    Some(p) if !p.is_empty() => {
                        secrets::save_key_passphrase(&dir, &entry.id, p)
                            .map_err(CommandError::plain)?;
                    }
                    _ => secrets::delete_key_passphrase(&dir, &entry.id),
                }
            }
            match file
                .keys
                .iter_mut()
                .find(|k| k.fingerprint == entry.fingerprint)
            {
                Some(slot) => *slot = entry,
                None => file.keys.push(entry),
            }
        }

        // Identities: label-keyed upsert; the key pin resolves through the
        // fingerprint map above. The password invariant ("a key-less identity
        // needs one") is enforced by `save_identity`'s UI path, not here: a
        // merged payload that leaves a password behind on purpose must apply.
        for identity in &identities_payload {
            let key_id = if identity.key_fp.is_empty() {
                String::new()
            } else {
                file.keys
                    .iter()
                    .find(|k| k.fingerprint == identity.key_fp)
                    .map(|k| k.id.clone())
                    .unwrap_or_default()
            };
            let entry = match file.identities.iter().find(|i| i.label == identity.label) {
                Some(existing) => {
                    let mut updated = existing.clone();
                    updated.username = identity.username.clone();
                    updated.key_id = key_id;
                    updated
                }
                None => Identity {
                    id: uuid::Uuid::new_v4().to_string(),
                    label: identity.label.clone(),
                    username: identity.username.clone(),
                    key_id,
                    created_at: if identity.created_at > 0 {
                        identity.created_at
                    } else {
                        now_secs()
                    },
                },
            };
            if let Some(p) = &identity.password {
                if !p.is_empty() {
                    secrets::save_identity_password(&dir, &entry.id, p)
                        .map_err(CommandError::plain)?;
                }
            }
            match file.identities.iter_mut().find(|i| i.label == entry.label) {
                Some(slot) => *slot = entry,
                None => file.identities.push(entry),
            }
        }

        identities::save_identities(&file).map_err(|e| CommandError::plain(e.to_string()))
    })
    .await
    .map_err(|e| CommandError::coded("taskFailed", format!("identity apply task failed: {e}")))?
}

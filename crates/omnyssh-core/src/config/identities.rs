//! `identities.toml` — keychain key entries and reusable login identities.
//!
//! Metadata only: PEM material, passphrases and identity passwords live in the
//! secret store (`secrets` module). The file sits next to `hosts.toml` and is
//! safe to hand-edit — everything in it is non-sensitive.
//!
//! ```toml
//! [[keys]]
//! id = "uuid"
//! label = "Tencent Cloud"
//! fingerprint = "SHA256:..."
//! public_key = "ssh-ed25519 ..."
//! created_at = 0
//!
//! [[identities]]
//! id = "uuid"
//! label = "local_vmware"
//! username = "root"
//! key_id = ""        # empty = password auth; else a keys[].id
//! created_at = 0
//! ```

#[cfg(test)]
use std::path::Path;

use anyhow::Context;
use russh::keys::ssh_key;
use russh::keys::ssh_key::private::KeypairData;
use russh::keys::{HashAlg, PrivateKey};
use serde::{Deserialize, Serialize};

use crate::secrets::{self, SecretBackend};
use crate::ssh::client::Host;
use crate::utils::platform;

/// Upper bound for an imported private key file.
pub const MAX_PEM_SIZE: usize = 256 * 1024;

/// A keychain private key entry: metadata for a PEM held in the secret store.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEntry {
    /// Stable id: the secret-store key_id route and the cloud-sync target id.
    pub id: String,
    #[serde(default)]
    pub label: String,
    /// OpenSSH `SHA256:...`, derived from the PEM at import time.
    #[serde(default)]
    pub fingerprint: String,
    /// The public half, copyable into a server's `authorized_keys`.
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub created_at: u64,
}

/// A reusable login identity: username + auth method, referenced by hosts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub username: String,
    /// Empty = password auth; else a [`KeyEntry::id`].
    #[serde(default)]
    pub key_id: String,
    #[serde(default)]
    pub created_at: u64,
}

/// The `identities.toml` container. `#[serde(default)]` everywhere so an old
/// file missing either section still parses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdentitiesFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<KeyEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<Identity>,
}

/// Loads `identities.toml`; a missing file is a blank slate.
///
/// # Errors
/// Returns an error when the file exists but cannot be read or parsed.
pub fn load_identities() -> anyhow::Result<IdentitiesFile> {
    let Some(path) = platform::identities_config_path() else {
        return Ok(IdentitiesFile::default());
    };
    if !path.exists() {
        return Ok(IdentitiesFile::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Persists `identities.toml` (tmp + rename, unix 0o600), as `hosts.toml` does.
///
/// # Errors
/// Returns an error when the directory cannot be created or the file cannot
/// be written.
pub fn save_identities(file: &IdentitiesFile) -> anyhow::Result<()> {
    let Some(path) = platform::identities_config_path() else {
        anyhow::bail!("Cannot determine identities config path");
    };
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("identities path has no parent"))?;
    std::fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create directory {}", dir.display()))?;

    let content = toml::to_string_pretty(file).context("Failed to serialise identities")?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, content).with_context(|| format!("Failed to write {}", tmp.display()))?;
    if let Err(e) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e)
            .with_context(|| format!("Failed to rename {} to {}", tmp.display(), path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reference counting (deletion guard)
// ---------------------------------------------------------------------------

/// How many hosts and identities reference the keychain entry `entry_id`.
pub fn key_refs(hosts: &[Host], identities: &[Identity], entry_id: &str) -> usize {
    hosts.iter().filter(|h| h.key_id == entry_id).count()
        + identities.iter().filter(|i| i.key_id == entry_id).count()
}

/// How many hosts reference the identity `identity_id`.
pub fn identity_refs(hosts: &[Host], identity_id: &str) -> usize {
    hosts
        .iter()
        .filter(|h| h.identity_id == identity_id)
        .count()
}

// ---------------------------------------------------------------------------
// PEM import
// ---------------------------------------------------------------------------

/// Reads a private key file for import, with the entry-level checks: a public
/// key is refused (pick the private one), the size is bounded, and the content
/// must look like PEM at all.
///
/// # Errors
/// A human-readable message for every rejection.
pub fn read_private_key_file(path: &str) -> Result<String, String> {
    if path.ends_with(".pub") {
        return Err("that is a public key — pick the private key file it belongs to".to_string());
    }
    let expanded = expand_tilde(path);
    let meta = std::fs::metadata(&expanded).map_err(|e| format!("cannot read {path}: {e}"))?;
    if meta.len() as usize > MAX_PEM_SIZE {
        return Err(format!(
            "{path} is too large for a private key (limit {} KiB)",
            MAX_PEM_SIZE / 1024
        ));
    }
    let content =
        std::fs::read_to_string(&expanded).map_err(|e| format!("cannot read {path}: {e}"))?;
    if !content.contains("-----BEGIN") {
        return Err(format!(
            "{path} does not look like a private key (no PEM header)"
        ));
    }
    Ok(content)
}

/// Decodes an imported PEM, telling encrypted keys apart: a plain key decodes
/// with no passphrase; an encrypted one needs a non-empty one. Returns the
/// decoded key for fingerprint/public-key derivation.
///
/// # Errors
/// A human-readable message for every rejection.
pub fn decode_pem(pem: &str, passphrase: Option<&str>) -> Result<PrivateKey, String> {
    match russh::keys::decode_secret_key(pem, None) {
        Ok(key) => {
            check_supported(&key)?;
            Ok(key)
        }
        Err(russh::keys::Error::KeyIsEncrypted) => {
            let pass = passphrase
                .filter(|p| !p.is_empty())
                .ok_or("this key is encrypted — enter its passphrase")?;
            let key = russh::keys::decode_secret_key(pem, Some(pass))
                .map_err(|_| "wrong passphrase, or not a valid private key".to_string())?;
            check_supported(&key)?;
            Ok(key)
        }
        Err(_) => Err("not a valid private key (or the passphrase is wrong)".to_string()),
    }
}

/// A key russh cannot sign with (FIDO, DSA) would end the whole connection —
/// reject it at import, mirroring `ssh::identity`.
pub(crate) fn check_supported(key: &PrivateKey) -> Result<(), String> {
    match key.key_data() {
        KeypairData::Ed25519(_) | KeypairData::Ecdsa(_) | KeypairData::Rsa(_) => Ok(()),
        _ => Err(format!(
            "unsupported key type {} (OmnySSH signs with Ed25519, ECDSA and RSA)",
            key.algorithm()
        )),
    }
}

/// Serialises a decoded key for storage: unencrypted, or re-encrypted under
/// `passphrase` (empty = store plain).
fn encode_private(key: &PrivateKey, passphrase: &str) -> Result<String, String> {
    let key = if passphrase.is_empty() {
        key.clone()
    } else {
        key.encrypt(&mut getrandom::SysRng, passphrase)
            .map_err(|e| format!("cannot encrypt the key with the new passphrase: {e}"))?
    };
    key.to_openssh(ssh_key::LineEnding::LF)
        .map(|s| s.to_string())
        .map_err(|e| format!("cannot serialise the private key: {e}"))
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

fn now_secs() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn config_dir() -> Result<std::path::PathBuf, String> {
    platform::app_config_dir()
        .ok_or_else(|| "cannot determine the app config directory".to_string())
}

/// Test-only hook to point the platform paths at a scratch directory. Kept in
/// `platform` for the migration tests; re-exported here for the identity
/// maintenance tests, which need real store roundtrips in isolation.
#[cfg(test)]
pub(crate) use crate::utils::platform::set_app_config_dir_for_tests;

// ---------------------------------------------------------------------------
// Key entry maintenance
// ---------------------------------------------------------------------------

/// Creates or updates a keychain entry. `id = None` creates (new uuid); an id
/// that does not exist is an error, not a create.
///
/// - `pem = Some`: validate, derive the fingerprint/public key, store the
///   material. A plain key never keeps a passphrase (a stale one is dropped);
///   an encrypted one must come with one, which is verified then stored.
/// - `pem = None`: keep the stored material (rename/relabel/passphrase
///   change); the entry must already have material in the store.
/// - `passphrase`: `None` = unchanged (only meaningful without a fresh PEM);
///   `Some("")` = delete the stored passphrase (a plain key needs none).
///
/// Blocking (store + file I/O).
///
/// # Errors
/// A human-readable message for every rejection; metadata is written only
/// after the store has accepted the material, so a failure leaves both sides
/// consistent.
pub fn save_key_entry(
    id: Option<&str>,
    label: &str,
    pem: Option<&str>,
    passphrase: Option<&str>,
) -> Result<KeyEntry, String> {
    let label = label.trim();
    if label.is_empty() {
        return Err("a label is required".to_string());
    }
    let dir = config_dir()?;
    let mut file = load_identities().map_err(|e| e.to_string())?;

    let mut entry = match id {
        Some(id) => file
            .keys
            .iter()
            .find(|k| k.id == id)
            .cloned()
            .ok_or_else(|| format!("no key entry {id}"))?,
        None if pem.is_some() => KeyEntry {
            id: new_id(),
            created_at: now_secs(),
            ..Default::default()
        },
        None => return Err("importing a key needs its PEM".to_string()),
    };
    entry.label = label.to_string();

    // The key this call ends up with — freshly imported, or the stored one
    // unlocked for a passphrase change. `None` = keep everything as-is (a
    // rename/relabel only).
    let decoded: Option<PrivateKey> = if let Some(pem) = pem {
        Some(decode_pem(pem, passphrase)?)
    } else if passphrase.is_some() {
        let stored_body = secrets::load_key_pem(&dir, &entry.id)
            .ok_or("the key material is missing from the secret store — re-import the PEM")?;
        // Unlock with what the store holds (a plain key needs nothing); the
        // new passphrase is only applied when re-encrypting below.
        let key = if russh::keys::decode_secret_key(&stored_body, None).is_ok() {
            decode_pem(&stored_body, None)?
        } else {
            let stored = secrets::load_key_passphrase(&dir, &entry.id)
                .filter(|p| !p.is_empty())
                .ok_or("the stored passphrase is missing — re-import the PEM")?;
            decode_pem(&stored_body, Some(&stored))?
        };
        Some(key)
    } else {
        None
    };

    if let Some(key) = &decoded {
        entry.fingerprint = key.public_key().fingerprint(HashAlg::Sha256).to_string();
        entry.public_key = key
            .public_key()
            .to_openssh()
            .map_err(|e| format!("cannot serialise the public key: {e}"))?;
    }

    // Storage. A fresh PEM is taken as given — a plain key never keeps a
    // passphrase (a stale one is dropped). A passphrase change re-encrypts
    // the decoded key, so the stored PEM and passphrase always match.
    match (pem, passphrase) {
        (Some(pem), _) => {
            let plain = russh::keys::decode_secret_key(pem, None).is_ok();
            secrets::save_key_pem(&dir, &entry.id, pem)?;
            if plain {
                secrets::delete_key_passphrase(&dir, &entry.id);
            } else {
                let pass = passphrase
                    .filter(|p| !p.is_empty())
                    .ok_or("this key is encrypted — enter its passphrase")?;
                secrets::save_key_passphrase(&dir, &entry.id, pass)?;
            }
        }
        (None, Some(requested)) => {
            let key = decoded.as_ref().expect("passphrase change decodes the key");
            let body = encode_private(key, requested)?;
            secrets::save_key_pem(&dir, &entry.id, &body)?;
            if requested.is_empty() {
                secrets::delete_key_passphrase(&dir, &entry.id);
            } else {
                secrets::save_key_passphrase(&dir, &entry.id, requested)?;
            }
        }
        (None, None) => {}
    }

    match file.keys.iter_mut().find(|k| k.id == entry.id) {
        Some(slot) => *slot = entry.clone(),
        None => file.keys.push(entry.clone()),
    }
    save_identities(&file).map_err(|e| e.to_string())?;
    Ok(entry)
}

/// Deletes a keychain entry, refusing while anything references it: `Err(n)`
/// carries the number of hosts + identities still pointing at it, and nothing
/// is touched. Metadata goes first; a store cleanup failure afterwards
/// surfaces as `Err(0)` (the file no longer lists the entry, but its materials
/// could not be removed — the user is told).
///
/// Blocking.
pub fn delete_key_entry(id: &str, hosts: &[Host]) -> Result<(), usize> {
    let refs = {
        let file = match load_identities() {
            Ok(f) => f,
            Err(_) => return Err(0),
        };
        key_refs(hosts, &file.identities, id)
    };
    if refs > 0 {
        return Err(refs);
    }
    let mut file = match load_identities() {
        Ok(f) => f,
        Err(_) => return Err(0),
    };
    if !file.keys.iter().any(|k| k.id == id) {
        return Ok(()); // already gone: the desired end state holds
    }
    file.keys.retain(|k| k.id != id);
    if save_identities(&file).is_err() {
        return Err(0);
    }

    let dir = match config_dir() {
        Ok(d) => d,
        Err(_) => return Err(0),
    };
    let pem = secrets::delete_secret_any(&dir, &format!("sshkeypem/{id}"), SecretBackend::Keyring);
    let pass =
        secrets::delete_secret_any(&dir, &format!("sshkeypass/{id}"), SecretBackend::Keyring);
    let first_err = pem.err().or(pass.err());
    if let Some(e) = first_err {
        tracing::warn!("the materials of key entry {id} could not be removed: {e}");
        return Err(0);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Identity maintenance
// ---------------------------------------------------------------------------

/// Creates or updates an identity. `id = None` creates; a missing id is an
/// error.
///
/// - `key_id`: `None` = unchanged; `Some("")` = clear (password auth);
///   `Some(x)` = reference the key entry `x`, which must exist.
/// - `password`: `None` = unchanged; `Some("")` = delete; otherwise store.
///
/// An identity without a key must end up with a password — a password auth
/// needs one. Validated before anything is written.
///
/// Blocking.
///
/// # Errors
/// A human-readable message for every rejection.
pub fn save_identity(
    id: Option<&str>,
    label: &str,
    username: &str,
    password: Option<&str>,
    key_id: Option<&str>,
) -> Result<Identity, String> {
    let label = label.trim();
    if label.is_empty() {
        return Err("a label is required".to_string());
    }
    let username = username.trim();
    if username.is_empty() {
        return Err("a username is required".to_string());
    }
    let dir = config_dir()?;
    let mut file = load_identities().map_err(|e| e.to_string())?;

    let mut identity = match id {
        Some(id) => file
            .identities
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or_else(|| format!("no identity {id}"))?,
        None => Identity {
            id: new_id(),
            created_at: now_secs(),
            ..Default::default()
        },
    };
    identity.label = label.to_string();
    identity.username = username.to_string();
    match key_id {
        Some("") => identity.key_id = String::new(),
        Some(x) => {
            if !file.keys.iter().any(|k| k.id == x) {
                return Err(format!("no key entry {x}"));
            }
            identity.key_id = x.to_string();
        }
        None => {}
    }

    // Invariant before anything is written: a key-less identity needs a
    // password (already stored, or provided now).
    let stored_password = secrets::load_identity_password(&dir, &identity.id).is_some();
    let will_have_password = match password {
        Some(p) => !p.is_empty(),
        None => stored_password,
    };
    if identity.key_id.is_empty() && !will_have_password {
        return Err("an identity without a key needs a password".to_string());
    }

    match password {
        Some("") => {
            secrets::delete_identity_password(&dir, &identity.id);
            Ok(())
        }
        Some(p) => {
            secrets::save_identity_password(&dir, &identity.id, p)?;
            Ok::<(), String>(())
        }
        None => Ok(()),
    }?;

    match file.identities.iter_mut().find(|i| i.id == identity.id) {
        Some(slot) => *slot = identity.clone(),
        None => file.identities.push(identity.clone()),
    }
    save_identities(&file).map_err(|e| e.to_string())?;
    Ok(identity)
}

/// Deletes an identity, refusing while hosts reference it (`Err` = the count).
/// The stored password goes with it (best effort, like every store cleanup).
///
/// Blocking.
pub fn delete_identity(id: &str, hosts: &[Host]) -> Result<(), usize> {
    let refs = identity_refs(hosts, id);
    if refs > 0 {
        return Err(refs);
    }
    let mut file = match load_identities() {
        Ok(f) => f,
        Err(_) => return Err(0),
    };
    if !file.identities.iter().any(|i| i.id == id) {
        return Ok(());
    }
    file.identities.retain(|i| i.id != id);
    if save_identities(&file).is_err() {
        return Err(0);
    }

    let dir = match config_dir() {
        Ok(d) => d,
        Err(_) => return Err(0),
    };
    secrets::delete_identity_password(&dir, id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{load_identity_password, load_key_passphrase, load_key_pem};

    /// Points the platform paths at a scratch directory for the duration of
    /// `f` (the store helpers touch the real keychain on Windows; unique
    /// scratch dirs keep tests isolated from each other and from a real
    /// install). Holds the process-global [`platform::test_config_lock`].
    fn with_scratch(f: impl FnOnce(&Path)) {
        let _guard = platform::test_config_lock();
        let dir = std::env::temp_dir().join(format!(
            "omnyssh-identities-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        set_app_config_dir_for_tests(Some(dir.clone()));
        f(&dir);
        set_app_config_dir_for_tests(None);
        std::fs::remove_dir_all(&dir).ok();
    }

    fn host(name: &str) -> Host {
        Host {
            name: name.to_string(),
            ..Host::default()
        }
    }

    fn host_ref(name: &str, key: &str, identity: &str) -> Host {
        Host {
            name: name.to_string(),
            key_id: key.to_string(),
            identity_id: identity.to_string(),
            ..Host::default()
        }
    }

    // Real Ed25519 keys, generated once with `ssh-keygen` and embedded — the
    // plain one's fingerprint is SHA256:TVyJA0nVMNkA+LytbG0O+vTaodi1otCDW/qRFaVEcBg.
    const PLAIN_PEM: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACD5iuy9V3jPnAmg20pxpgAMH95viExt85iQsIdBNMXqIgAAAJBkHR5lZB0e\nZQAAAAtzc2gtZWQyNTUxOQAAACD5iuy9V3jPnAmg20pxpgAMH95viExt85iQsIdBNMXqIg\nAAAEDMG2GI1sLG3+8qJpAil5/OVgKa8KWkwBRtozzRa6eHR/mK7L1XeM+cCaDbSnGmAAwf\n3m+ITG3zmJCwh0E0xeoiAAAADG9tbnlzc2hAdGVzdAE=\n-----END OPENSSH PRIVATE KEY-----\n";
    /// `omnyssh@test`, passphrase "correct horse".
    const ENCRYPTED_PEM: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABCA2dmfyb\nyHTaNT5q5Bd6EsAAAAGAAAAAEAAAAzAAAAC3NzaC1lZDI1NTE5AAAAIADQu8X2q/oTzMJT\noulw5ZeS9Wh9RN5WH+7dmIJ/1WSoAAAAkEp+4moApx2tOv8lGcggiRe7AfNK0fk95OucLv\nAQNTWkt3xHhCN+YMVhxensAqY1KiNrZdVuqqLYG1bw2IjWWcHzZneBilrkitiAfgOxSIW2\nMnxChcppXuEZDf23Lb4Axt4YGY+cpGirb53ZnhBzb6l0Bx1p0mgViS8yKzj5C6MUF7Jd9t\n7ERNVe0kTgzdaYrQ==\n-----END OPENSSH PRIVATE KEY-----\n";

    #[test]
    fn an_encrypted_key_needs_and_stores_its_passphrase() {
        with_scratch(|dir| {
            // Without a passphrase: refused with a clear ask.
            let err = save_key_entry(None, "enc", Some(ENCRYPTED_PEM), None).unwrap_err();
            assert!(err.contains("encrypted"), "{err}");
            // With the right one: stored.
            let entry =
                save_key_entry(None, "enc", Some(ENCRYPTED_PEM), Some("correct horse")).unwrap();
            assert_eq!(
                load_key_passphrase(dir, &entry.id).as_deref(),
                Some("correct horse")
            );
            // A wrong one is refused and nothing is stored.
            let fresh = save_key_entry(None, "enc2", Some(ENCRYPTED_PEM), Some("nope"));
            assert!(fresh.unwrap_err().contains("passphrase"));
            // Passphrase update without re-providing the PEM verifies first.
            let updated = save_key_entry(Some(&entry.id), "enc", None, Some("new horse")).unwrap();
            assert_eq!(
                load_key_passphrase(dir, &updated.id).as_deref(),
                Some("new horse")
            );
            // Clearing it (`Some("")`) removes the entry.
            save_key_entry(Some(&entry.id), "enc", None, Some("")).unwrap();
            assert_eq!(load_key_passphrase(dir, &entry.id), None);
        });
    }

    #[test]
    fn identities_file_roundtrips_and_defaults_to_empty() {
        with_scratch(|_dir| {
            // A missing file parses as empty.
            assert!(load_identities().unwrap().keys.is_empty());
            assert!(load_identities().unwrap().identities.is_empty());
            let file = IdentitiesFile {
                keys: vec![KeyEntry {
                    id: "k1".into(),
                    label: "Tencent Cloud".into(),
                    fingerprint: "SHA256:abc".into(),
                    public_key: "ssh-ed25519 AAA".into(),
                    created_at: 42,
                }],
                identities: vec![Identity {
                    id: "i1".into(),
                    label: "vm".into(),
                    username: "root".into(),
                    key_id: "k1".into(),
                    created_at: 7,
                }],
            };
            save_identities(&file).unwrap();
            let loaded = load_identities().unwrap();
            assert_eq!(loaded.keys.len(), 1);
            assert_eq!(loaded.identities[0].username, "root");
            // An old file missing a section still parses.
            let path = platform::identities_config_path().unwrap();
            std::fs::write(&path, "[[keys]]\nid = \"only\"\n").unwrap();
            let old = load_identities().unwrap();
            assert_eq!(old.keys[0].id, "only");
            assert!(old.identities.is_empty());
        });
    }

    #[test]
    fn reference_counts_cover_hosts_and_identities() {
        let hosts = vec![
            host_ref("a", "k1", "i1"),
            host_ref("b", "k1", ""),
            host("c"),
        ];
        let identities = vec![Identity {
            id: "i2".into(),
            key_id: "k1".into(),
            ..Identity::default()
        }];
        assert_eq!(
            key_refs(&hosts, &identities, "k1"),
            3,
            "2 hosts + 1 identity"
        );
        assert_eq!(key_refs(&hosts, &identities, "k2"), 0);
        assert_eq!(identity_refs(&hosts, "i1"), 1);
        assert_eq!(identity_refs(&hosts, "i2"), 0);
    }

    #[test]
    fn importing_a_plain_key_derives_fingerprint_and_skips_the_passphrase() {
        with_scratch(|dir| {
            let entry = save_key_entry(None, "Tencent Cloud", Some(PLAIN_PEM), None).unwrap();
            assert!(entry.id.contains('-'), "a uuid id");
            assert!(entry.fingerprint.starts_with("SHA256:"));
            assert!(entry.public_key.starts_with("ssh-ed25519 "));
            assert_eq!(load_key_pem(dir, &entry.id).as_deref(), Some(PLAIN_PEM));
            assert_eq!(load_key_passphrase(dir, &entry.id), None);

            // Update in place: new material replaces the old, created_at stays.
            let created = entry.created_at;
            let updated =
                save_key_entry(Some(&entry.id), "Renamed", Some(PLAIN_PEM), None).unwrap();
            assert_eq!(updated.created_at, created);
            assert_eq!(updated.label, "Renamed");

            // A missing id is an error, not a create.
            assert!(save_key_entry(Some("ghost"), "x", Some(PLAIN_PEM), None).is_err());
        });
    }

    #[test]
    fn a_pem_less_update_keeps_the_stored_material() {
        with_scratch(|dir| {
            let entry = save_key_entry(None, "k", Some(PLAIN_PEM), None).unwrap();
            let renamed = save_key_entry(Some(&entry.id), "renamed", None, None).unwrap();
            assert_eq!(renamed.fingerprint, entry.fingerprint);
            assert_eq!(load_key_pem(dir, &entry.id).as_deref(), Some(PLAIN_PEM));
        });
    }

    #[test]
    fn a_public_key_file_and_oversized_files_are_refused() {
        with_scratch(|_dir| {
            let dir = std::env::temp_dir().join("omnyssh-pem-refuse-test");
            std::fs::create_dir_all(&dir).unwrap();
            let pub_path = dir.join("key.pub");
            std::fs::write(&pub_path, "ssh-ed25519 AAAA comment").unwrap();
            assert!(read_private_key_file(pub_path.to_str().unwrap())
                .unwrap_err()
                .contains("public key"));

            let txt_path = dir.join("notes.txt");
            std::fs::write(&txt_path, "hello").unwrap();
            assert!(read_private_key_file(txt_path.to_str().unwrap())
                .unwrap_err()
                .contains("PEM header"));

            let big_path = dir.join("big.pem");
            std::fs::write(
                &big_path,
                format!("-----BEGIN X-----\n{}", "a".repeat(MAX_PEM_SIZE)),
            )
            .unwrap();
            assert!(read_private_key_file(big_path.to_str().unwrap())
                .unwrap_err()
                .contains("too large"));
            std::fs::remove_dir_all(&dir).ok();
        });
    }

    #[test]
    fn garbage_pem_is_refused_with_a_clear_error() {
        with_scratch(|_dir| {
            let err = save_key_entry(None, "k", Some("not a pem"), None).unwrap_err();
            assert!(err.contains("valid private key"), "{err}");
        });
    }

    #[test]
    fn a_keyless_identity_needs_a_password() {
        with_scratch(|_dir| {
            // New without a password: refused.
            assert!(save_identity(None, "vm", "root", None, None)
                .unwrap_err()
                .contains("needs a password"));
            // With one: created, and the password lands in the store.
            let id = save_identity(None, "vm", "root", Some("pw"), None).unwrap();
            assert!(!id.id.is_empty());
            // `None` keeps the stored password across edits.
            let edited = save_identity(Some(&id.id), "vm2", "deploy", None, None).unwrap();
            assert_eq!(edited.username, "deploy");
            // `Some("")` deletes it — after which the invariant bites again.
            assert!(save_identity(Some(&id.id), "vm", "root", Some(""), None)
                .unwrap_err()
                .contains("needs a password"));
            // A key reference must point at a real entry.
            assert!(save_identity(Some(&id.id), "vm", "root", None, Some("ghost")).is_err());
        });
    }

    #[test]
    fn a_key_bound_identity_needs_no_password() {
        with_scratch(|dir| {
            let key = save_key_entry(None, "k", Some(PLAIN_PEM), None).unwrap();
            let id = save_identity(None, "vm", "root", None, Some(&key.id)).unwrap();
            assert_eq!(id.key_id, key.id);
            assert_eq!(load_identity_password(dir, &id.id), None);
            // Clearing the key binding afterwards trips the password rule.
            assert!(save_identity(Some(&id.id), "vm", "root", None, Some("")).is_err());
        });
    }

    #[test]
    fn referenced_entries_refuse_deletion_and_free_ones_go_cleanly() {
        with_scratch(|dir| {
            let key = save_key_entry(None, "k", Some(PLAIN_PEM), None).unwrap();
            let id = save_identity(None, "vm", "root", Some("pw"), Some(&key.id)).unwrap();
            let hosts = vec![host_ref("web", &key.id, &id.id)];

            // Both are referenced: refused, and the store is untouched.
            assert_eq!(delete_key_entry(&key.id, &hosts), Err(2)); // host + identity
            assert_eq!(delete_identity(&id.id, &hosts), Err(1));
            assert!(load_key_pem(dir, &key.id).is_some());
            assert!(load_identity_password(dir, &id.id).is_some());

            // Host references cleared, but the identity still points at the
            // key: the key stays refused until the identity is gone.
            let hosts = vec![host("web")];
            delete_identity(&id.id, &hosts).unwrap();
            assert!(load_identity_password(dir, &id.id).is_none());
            delete_key_entry(&key.id, &hosts).unwrap();
            assert!(load_key_pem(dir, &key.id).is_none());
            assert!(load_key_passphrase(dir, &key.id).is_none());
            assert!(load_identity_password(dir, &id.id).is_none());
            let file = load_identities().unwrap();
            assert!(file.keys.is_empty());
            assert!(file.identities.is_empty());
        });
    }
}

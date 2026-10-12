//! Persistent secret storage for login passwords (and, later, key
//! passphrases).
//!
//! Secrets never live in `hosts.toml`; they go to the system keychain, with a
//! machine-bound encrypted file as the fallback ([`store::SecretBackend`]).
//! The in-process password cache (`ssh::password`) stays a separate, memory
//! domain — this module only pre-fills answers for it.

pub mod migration;
pub mod store;

use std::path::Path;

pub use store::{delete_secret, delete_secret_any, load_secret, load_secret_any, save_secret,
                SecretBackend};

/// Key id of a host's login password: `login/{name}`. The host name is the
/// entity id (hosts are upserted by name), so this must stay stable — the
/// frontends and any future sync layer route by it.
pub(crate) fn login_key(host_name: &str) -> String {
    format!("login/{host_name}")
}

/// Key id of a private key's passphrase: `keypass/{canonical_path}`. Reserved
/// for the passphrase persistence phase; shares the semantics of the
/// in-memory cache in `ssh::identity`.
#[allow(dead_code)]
pub(crate) fn keypass_key(canonical_path: &str) -> String {
    format!("keypass/{canonical_path}")
}

// ---------------------------------------------------------------------------
// Keychain key entries & identities (identities.toml; credentials-adaptation
// follow-up plan). Same `category/entity-id` style, dockpilot-compatible
// naming, so the cloud-sync vocabulary needs no renaming later.
// ---------------------------------------------------------------------------

/// Key id of a keychain entry's private key PEM: `sshkeypem/{entry_id}`.
pub(crate) fn sshkeypem_key(entry_id: &str) -> String {
    format!("sshkeypem/{entry_id}")
}

/// Key id of a keychain entry's passphrase: `sshkeypass/{entry_id}`. Distinct
/// from `keypass/{canonical_path}`, which caches passphrases for key *files*.
pub(crate) fn sshkeypass_key(entry_id: &str) -> String {
    format!("sshkeypass/{entry_id}")
}

/// Key id of an identity's login password: `sshidpass/{identity_id}`.
pub(crate) fn sshidpass_key(identity_id: &str) -> String {
    format!("sshidpass/{identity_id}")
}

/// Saves the keychain entry's PEM. Blocking.
pub fn save_key_pem(dir: &Path, entry_id: &str, pem: &str) -> Result<SecretBackend, String> {
    save_secret(dir, &sshkeypem_key(entry_id), pem)
}

/// Loads the keychain entry's PEM; `None` when absent. Blocking.
pub fn load_key_pem(dir: &Path, entry_id: &str) -> Option<String> {
    match load_secret_any(dir, &sshkeypem_key(entry_id), SecretBackend::Keyring) {
        Ok(pem) => pem,
        Err(e) => {
            tracing::warn!("failed to load the keychain key {entry_id}: {e}");
            None
        }
    }
}

/// Deletes the keychain entry's PEM, from both backends, best effort. Blocking.
pub fn delete_key_pem(dir: &Path, entry_id: &str) {
    if let Err(e) = delete_secret_any(dir, &sshkeypem_key(entry_id), SecretBackend::Keyring) {
        tracing::warn!("failed to delete the keychain key {entry_id}: {e}");
    }
}

/// Saves the keychain entry's passphrase. Blocking.
pub fn save_key_passphrase(
    dir: &Path,
    entry_id: &str,
    passphrase: &str,
) -> Result<SecretBackend, String> {
    save_secret(dir, &sshkeypass_key(entry_id), passphrase)
}

/// Loads the keychain entry's passphrase; `None` when absent. Blocking.
pub fn load_key_passphrase(dir: &Path, entry_id: &str) -> Option<String> {
    match load_secret_any(dir, &sshkeypass_key(entry_id), SecretBackend::Keyring) {
        Ok(pw) => pw,
        Err(e) => {
            tracing::warn!("failed to load the keychain passphrase for {entry_id}: {e}");
            None
        }
    }
}

/// Deletes the keychain entry's passphrase, from both backends, best effort.
/// Blocking.
pub fn delete_key_passphrase(dir: &Path, entry_id: &str) {
    if let Err(e) = delete_secret_any(dir, &sshkeypass_key(entry_id), SecretBackend::Keyring) {
        tracing::warn!("failed to delete the keychain passphrase for {entry_id}: {e}");
    }
}

/// Saves the identity's login password. Blocking.
pub fn save_identity_password(
    dir: &Path,
    identity_id: &str,
    password: &str,
) -> Result<SecretBackend, String> {
    save_secret(dir, &sshidpass_key(identity_id), password)
}

/// Loads the identity's login password; `None` when absent. Blocking.
pub fn load_identity_password(dir: &Path, identity_id: &str) -> Option<String> {
    match load_secret_any(dir, &sshidpass_key(identity_id), SecretBackend::Keyring) {
        Ok(pw) => pw,
        Err(e) => {
            tracing::warn!("failed to load the identity password for {identity_id}: {e}");
            None
        }
    }
}

/// Deletes the identity's login password, from both backends, best effort.
/// Blocking.
pub fn delete_identity_password(dir: &Path, identity_id: &str) {
    if let Err(e) = delete_secret_any(dir, &sshidpass_key(identity_id), SecretBackend::Keyring) {
        tracing::warn!("failed to delete the identity password for {identity_id}: {e}");
    }
}

/// Loads the login password saved for `host_name`, trying both backends.
/// Errors are logged, not propagated: a missing or unreadable secret must not
/// break loading the host list.
pub fn load_login_password(dir: &Path, host_name: &str) -> Option<String> {
    match load_secret_any(dir, &login_key(host_name), SecretBackend::Keyring) {
        Ok(pw) => pw,
        Err(e) => {
            tracing::warn!("failed to load the saved password for {host_name}: {e}");
            None
        }
    }
}

/// Saves the login password for `host_name`. Blocking (keychain I/O) — call
/// from a blocking context.
pub fn save_login_password(dir: &Path, host_name: &str, password: &str) -> Result<SecretBackend, String> {
    save_secret(dir, &login_key(host_name), password)
}

/// Deletes the login password saved for `host_name`, from both backends,
/// best effort. Blocking.
pub fn delete_login_password(dir: &Path, host_name: &str) {
    if let Err(e) = delete_secret_any(dir, &login_key(host_name), SecretBackend::Keyring) {
        tracing::warn!("failed to delete the saved password for {host_name}: {e}");
    }
}

/// Reports where the saved login password of `host_name` lives, if one does.
/// Neither backend is recorded anywhere, so both are probed: the keychain
/// first (what a save prefers, errors soft — an unusable keychain counts as a
/// miss), then the encrypted file (its errors are real).
pub fn login_password_backend(dir: &Path, host_name: &str) -> Result<Option<SecretBackend>, String> {
    let key = login_key(host_name);
    match load_secret(dir, &key, SecretBackend::Keyring) {
        Ok(Some(_)) => return Ok(Some(SecretBackend::Keyring)),
        Err(e) => tracing::debug!("keychain probe for {host_name} failed: {e}"),
        Ok(None) => {}
    }
    if load_secret(dir, &key, SecretBackend::File)?.is_some() {
        Ok(Some(SecretBackend::File))
    } else {
        Ok(None)
    }
}

/// Moves a host's saved login password to its new name. Best effort: when the
/// old entry is gone or unreadable there is nothing to move; when the write
/// fails the old entry is kept.
pub fn rename_login_password(dir: &Path, old_name: &str, new_name: &str) {
    if old_name == new_name {
        return;
    }
    let Some(password) = load_login_password(dir, old_name) else {
        return;
    };
    match save_login_password(dir, new_name, &password) {
        Ok(_) => delete_login_password(dir, old_name),
        Err(e) => tracing::warn!("failed to move the saved password of {old_name} to {new_name}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "omnyssh-secrets-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A host name unique to this test run: the keychain backend is real, so
    /// tests must never collide with real installs' entries, and every test
    /// cleans its own up.
    fn unique_host(label: &str) -> String {
        format!(
            "test-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        )
    }

    #[test]
    fn login_password_roundtrip_and_delete() {
        let dir = tempdir();
        let host = unique_host("rt");
        save_login_password(&dir, &host, "s3cr3t").unwrap();
        assert_eq!(load_login_password(&dir, &host).as_deref(), Some("s3cr3t"));
        delete_login_password(&dir, &host);
        assert_eq!(load_login_password(&dir, &host), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rename_moves_the_entry_and_leaves_no_orphan() {
        let dir = tempdir();
        let old = unique_host("old");
        let new = unique_host("new");
        save_login_password(&dir, &old, "pw").unwrap();
        rename_login_password(&dir, &old, &new);
        assert_eq!(load_login_password(&dir, &new).as_deref(), Some("pw"));
        assert_eq!(load_login_password(&dir, &old), None);
        // Renaming a host without a saved password is a no-op.
        rename_login_password(&dir, &unique_host("ghost"), &unique_host("other"));
        std::fs::remove_dir_all(&dir).ok();
    }
}

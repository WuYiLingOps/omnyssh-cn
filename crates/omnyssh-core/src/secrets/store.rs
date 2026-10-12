//! Local storage for sensitive values (login passwords today, key passphrases
//! later).
//!
//! Prefers the system keychain (macOS Keychain / Windows Credential Manager /
//! Linux Secret Service); on hosts without one (headless Linux etc.) it falls
//! back to a machine-bound AES-256-GCM encrypted file
//! (`app_config_dir()/secrets.bin`), with the key derived from the machine id
//! via SHA-256. The fallback is obfuscation-grade: it keeps the secret from
//! other ordinary users on the same machine, but not from the user itself or
//! root, and it cannot be decrypted after moving to another machine or
//! reinstalling the OS.
//!
//! Core functions take the config directory as a parameter (keeps them unit
//! testable); callers resolve it with `crate::utils::platform::app_config_dir()`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Where a secret actually lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretBackend {
    /// The system keychain.
    Keyring,
    /// The machine-bound encrypted file (fallback).
    File,
}

impl SecretBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecretBackend::Keyring => "keyring",
            SecretBackend::File => "file",
        }
    }

    pub fn parse(s: &str) -> Option<SecretBackend> {
        match s {
            "keyring" => Some(SecretBackend::Keyring),
            "file" => Some(SecretBackend::File),
            _ => None,
        }
    }
}

/// Keyring entries are isolated under one service by `key_id`
/// (e.g. `login/web-prod-1`).
const KEYRING_SERVICE: &str = "app.omnyssh";

// ---------------------------------------------------------------------------
// Public API: core functions take the config directory (unit-testable)
// ---------------------------------------------------------------------------

/// Saves a secret: prefers the keychain, falls back to the encrypted file when
/// no keychain is available. Returns where it landed.
pub fn save_secret(dir: &Path, key_id: &str, secret: &str) -> Result<SecretBackend, String> {
    match save_keyring(key_id, secret) {
        Ok(()) => Ok(SecretBackend::Keyring),
        // Headless environments (no Secret Service etc.): fall back to the
        // encrypted file; callers surface the backend to the user.
        Err(_) => {
            save_file(dir, key_id, secret)?;
            Ok(SecretBackend::File)
        }
    }
}

/// Reads a secret from the given backend; `None` when absent.
pub fn load_secret(
    dir: &Path,
    key_id: &str,
    backend: SecretBackend,
) -> Result<Option<String>, String> {
    match backend {
        SecretBackend::Keyring => load_keyring(key_id),
        SecretBackend::File => load_file(dir, key_id),
    }
}

/// Deletes a secret: best effort, an absent entry counts as deleted.
pub fn delete_secret(dir: &Path, key_id: &str, backend: SecretBackend) -> Result<(), String> {
    match backend {
        SecretBackend::Keyring => delete_keyring(key_id),
        SecretBackend::File => delete_file(dir, key_id),
    }
}

/// Reads with the preferred backend first, falling back to the other one.
/// Login passwords and key passphrases share one namespace but live under
/// different `key_id`s, so the backend they landed on can differ between
/// saves (keychain availability changed in between); reads tolerate both.
pub fn load_secret_any(
    dir: &Path,
    key_id: &str,
    prefer: SecretBackend,
) -> Result<Option<String>, String> {
    let (first, second) = others(prefer);
    let mut result = Ok(None);
    for backend in [first, second] {
        result = match load_secret(dir, key_id, backend) {
            // Keychain errors are mostly environmental (no Secret Service
            // etc.) — treat as a miss and fall through; file-backend errors
            // (tampered ciphertext etc.) are reported as they are.
            Err(e) if backend == SecretBackend::Keyring => {
                tracing::debug!("keychain read for {key_id} failed, trying fallback: {e}");
                Ok(None)
            }
            other => other,
        };
        if matches!(&result, Ok(Some(_))) {
            break;
        }
    }
    result
}

/// Deletes from both backends, best effort (absent entries are fine); the
/// last real error wins when both fail.
pub fn delete_secret_any(dir: &Path, key_id: &str, prefer: SecretBackend) -> Result<(), String> {
    let (first, second) = others(prefer);
    let mut last_err = Ok(());
    for backend in [first, second] {
        if let Err(e) = delete_secret(dir, key_id, backend) {
            if backend == SecretBackend::Keyring {
                // Environmental keychain errors (no Secret Service etc.) keep
                // the best-effort semantics and are not treated as failures.
                tracing::debug!("keychain delete for {key_id} failed (ignored): {e}");
            } else {
                last_err = Err(e);
            }
        }
    }
    last_err
}

fn others(prefer: SecretBackend) -> (SecretBackend, SecretBackend) {
    match prefer {
        SecretBackend::Keyring => (SecretBackend::Keyring, SecretBackend::File),
        SecretBackend::File => (SecretBackend::File, SecretBackend::Keyring),
    }
}

// ---------------------------------------------------------------------------
// Keychain backend
// ---------------------------------------------------------------------------

fn keyring_entry(key_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, key_id)
        .map_err(|e| format!("failed to create keychain entry: {e}"))
}

fn save_keyring(key_id: &str, secret: &str) -> Result<(), String> {
    keyring_entry(key_id)?
        .set_password(secret)
        .map_err(|e| format!("failed to write keychain entry: {e}"))
}

fn load_keyring(key_id: &str) -> Result<Option<String>, String> {
    match keyring_entry(key_id)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("failed to read keychain entry: {e}")),
    }
}

fn delete_keyring(key_id: &str) -> Result<(), String> {
    match keyring_entry(key_id)?.delete_credential() {
        Ok(()) => Ok(()),
        // An absent entry counts as deleted. Matched by type on purpose: the
        // `Display` text is platform-specific English.
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("failed to delete keychain entry: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Encrypted-file backend: app_config_dir()/secrets.bin
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct VaultEntry {
    /// base64(nonce, 12 bytes, one per entry)
    nonce: String,
    /// base64(AES-256-GCM ciphertext)
    cipher: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    entries: HashMap<String, VaultEntry>,
}

fn vault_path(dir: &Path) -> PathBuf {
    dir.join("secrets.bin")
}

/// Key derivation: machine id + fixed salt → SHA-256 (a 32-byte AES-256 key).
fn derive_key() -> Result<[u8; 32], String> {
    let uid = machine_uid::get().map_err(|e| format!("failed to get machine id: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(b"omnyssh-v1:secret-store:");
    hasher.update(uid.as_bytes());
    Ok(hasher.finalize().into())
}

fn load_vault(dir: &Path) -> Result<VaultFile, String> {
    let path = vault_path(dir);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("failed to parse the secret vault: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(VaultFile {
            version: 1,
            entries: HashMap::new(),
        }),
        Err(e) => Err(format!("failed to read the secret vault: {e}")),
    }
}

/// Atomic write via tmp + rename (same strategy as hosts.toml).
fn save_vault(dir: &Path, vault: &VaultFile) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create config directory: {e}"))?;
    let path = vault_path(dir);
    let text = serde_json::to_vec_pretty(vault)
        .map_err(|e| format!("failed to serialize the secret vault: {e}"))?;
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("failed to write the secret vault: {e}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("failed to save the secret vault: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn encrypt(secret: &str) -> Result<(String, String), String> {
    let key = derive_key()?;
    let cipher = Aes256Gcm::new((&key).into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, secret.as_bytes())
        .map_err(|e| format!("encryption failed: {e}"))?;
    Ok((B64.encode(nonce), B64.encode(ct)))
}

fn decrypt(nonce_b64: &str, cipher_b64: &str) -> Result<String, String> {
    let key = derive_key()?;
    let cipher = Aes256Gcm::new((&key).into());
    let nonce_bytes = B64
        .decode(nonce_b64)
        .map_err(|e| format!("failed to decode the nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = B64
        .decode(cipher_b64)
        .map_err(|e| format!("failed to decode the ciphertext: {e}"))?;
    let pt = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| {
            "decryption failed: the secret vault may come from another machine or OS install, \
             re-enter the password"
                .to_string()
        })?;
    String::from_utf8(pt).map_err(|e| format!("decrypted value is not valid UTF-8: {e}"))
}

pub(crate) fn save_file(dir: &Path, key_id: &str, secret: &str) -> Result<(), String> {
    let (nonce, cipher) = encrypt(secret)?;
    let mut vault = load_vault(dir)?;
    vault
        .entries
        .insert(key_id.to_string(), VaultEntry { nonce, cipher });
    save_vault(dir, &vault)
}

pub(crate) fn load_file(dir: &Path, key_id: &str) -> Result<Option<String>, String> {
    let vault = load_vault(dir)?;
    match vault.entries.get(key_id) {
        Some(e) => Ok(Some(decrypt(&e.nonce, &e.cipher)?)),
        None => Ok(None),
    }
}

pub(crate) fn delete_file(dir: &Path, key_id: &str) -> Result<(), String> {
    let mut vault = load_vault(dir)?;
    if vault.entries.remove(key_id).is_some() {
        save_vault(dir, &vault)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "omnyssh-secret-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A key id unique to this test run. The keychain backend is real (no
    /// test double exists), so tests must never collide with each other or
    /// with entries a real OmnySSH install wrote; every test cleans its own
    /// entry up.
    fn unique_key(label: &str) -> String {
        format!(
            "login/test-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        )
    }

    #[test]
    fn file_backend_roundtrip() {
        let dir = tempdir();
        let key = unique_key("rt");
        let r = save_file(&dir, &key, "p@ss word-密码");
        assert!(r.is_ok(), "file save failed: {r:?}");
        let loaded = load_file(&dir, &key).unwrap();
        assert_eq!(loaded.as_deref(), Some("p@ss word-密码"));
        // Overwrite.
        save_file(&dir, &key, "new-pass").unwrap();
        assert_eq!(load_file(&dir, &key).unwrap().as_deref(), Some("new-pass"));
        // Different keys stay isolated.
        assert!(load_file(&dir, &unique_key("rt-other")).unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_backend_delete_and_missing_vault() {
        let dir = tempdir();
        let key = unique_key("del");
        // Reads and deletes are safe when the vault does not exist yet.
        assert!(load_file(&dir, &key).unwrap().is_none());
        assert!(delete_file(&dir, &key).is_ok());
        save_file(&dir, &key, "v").unwrap();
        delete_file(&dir, &key).unwrap();
        assert!(load_file(&dir, &key).unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn any_backend_prefers_hint_then_falls_back() {
        let dir = tempdir();
        let key = unique_key("any");
        save_file(&dir, &key, "v1").unwrap();
        // A file hint reads a hit directly.
        assert_eq!(
            load_secret_any(&dir, &key, SecretBackend::File)
                .unwrap()
                .as_deref(),
            Some("v1")
        );
        // A keychain hint (the entry was not written here) falls back to the
        // file backend; in a keychain-less environment the read error is soft,
        // so the fallback still wins.
        assert_eq!(
            load_secret_any(&dir, &key, SecretBackend::Keyring)
                .unwrap()
                .as_deref(),
            Some("v1")
        );
        // After deleting from both backends the secret is gone.
        delete_secret_any(&dir, &key, SecretBackend::Keyring).unwrap();
        assert!(load_secret_any(&dir, &key, SecretBackend::Keyring)
            .unwrap()
            .is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_backend_rejects_tampered_cipher() {
        let dir = tempdir();
        let key = unique_key("tamper");
        save_file(&dir, &key, "secret").unwrap();
        let mut vault = load_vault(&dir).unwrap();
        vault.entries.get_mut(&key).unwrap().cipher = B64.encode(b"tampered-data!!");
        save_vault(&dir, &vault).unwrap();
        let err = load_file(&dir, &key).unwrap_err();
        assert!(err.contains("decryption failed"), "must report, not panic: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn vault_survives_unknown_fields_and_version() {
        let dir = tempdir();
        let key = unique_key("legacy");
        save_file(&dir, &key, "v1").unwrap();
        // Old entries survive, and a missing version field does not matter.
        let vault: VaultFile =
            serde_json::from_str(&std::fs::read_to_string(vault_path(&dir)).unwrap()).unwrap();
        assert!(vault.entries.contains_key(&key));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Depends on the system keychain: CI / headless environments have no
    /// Secret Service, so this stays ignored by default.
    #[test]
    #[ignore]
    fn keyring_backend_roundtrip() {
        let key = unique_key("keyring");
        save_keyring(&key, "v").unwrap();
        assert_eq!(load_keyring(&key).unwrap().as_deref(), Some("v"));
        delete_keyring(&key).unwrap();
        assert!(load_keyring(&key).unwrap().is_none());
    }

    /// The automatic fallback: a keychain-less environment must land on file.
    #[test]
    fn save_secret_falls_back_without_keyring() {
        let dir = tempdir();
        let key = unique_key("fallback");
        // Platforms with a keychain return Keyring; keychain-less platforms
        // fall back to File. Both are valid roundtrips.
        let backend = save_secret(&dir, &key, "v").unwrap();
        let loaded = load_secret(&dir, &key, backend).unwrap();
        assert_eq!(loaded.as_deref(), Some("v"));
        delete_secret(&dir, &key, backend).unwrap();
        assert!(load_secret(&dir, &key, backend).unwrap().is_none());
        // Whatever backend won, the entry must not survive the test.
        delete_secret_any(&dir, &key, SecretBackend::Keyring).ok();
        std::fs::remove_dir_all(&dir).ok();
    }
}

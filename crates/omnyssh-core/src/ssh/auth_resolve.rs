//! Resolves what a [`Host`] authenticates with, before the connection opens.
//!
//! A host can pin its credentials by reference: a keychain key entry
//! (`key_id`) or a reusable identity (`identity_id`) from `identities.toml`.
//! [`resolve`] turns those references into an [`AuthPlan`] — the username to
//! log in as, the decoded keychain key to offer, and the identity's saved
//! password — so the connection path never re-reads the config.
//!
//! The plan is advisory and fails soft: a dangling reference or a store miss
//! logs a warning and yields an empty plan, leaving the host's own settings
//! (agent, identity file, password) to apply. A broken reference must not
//! brick a login the host config could still complete.

use russh::keys::PrivateKey;

use crate::config::identities::{self, IdentitiesFile};
use crate::secrets;
use crate::ssh::client::Host;
use crate::utils::platform;

/// A keychain key the host (or its identity) references, decoded and ready to
/// sign with. The PEM and its passphrase come from the secret store; an
/// encrypted PEM always has its passphrase stored (import enforces it).
pub struct KeychainKey {
    /// The `identities.toml` key entry this came from.
    pub entry_id: String,
    /// Its label, for log lines and refusal messages.
    pub label: String,
    /// The decoded private key.
    pub key: PrivateKey,
}

/// What a host authenticates with, resolved ahead of dialing.
pub struct AuthPlan {
    /// The identity's username, when the host references an identity: it
    /// replaces `host.user` for the whole login.
    pub username_override: Option<String>,
    /// The referenced identity, when it exists — the drop-side of
    /// [`Self::identity_password`]: a rejected saved password is removed from
    /// the store under this id.
    pub identity_id: Option<String>,
    /// The keychain key to offer first. When present, the file keys
    /// (`identity_file`, defaults) are skipped — the user pinned this one.
    pub keychain: Option<KeychainKey>,
    /// The password stored for a key-less identity, tried before the host's
    /// own saved password. Only set when the identity has no key of its own.
    pub identity_password: Option<String>,
}

impl AuthPlan {
    /// The plan for a host with no references: everything as configured.
    pub fn empty() -> Self {
        Self {
            username_override: None,
            identity_id: None,
            keychain: None,
            identity_password: None,
        }
    }

    /// The host to dial: with a username override, the identity's user
    /// replaces the host's, prompts included.
    pub fn apply_to(&self, host: &Host) -> Host {
        let mut host = host.clone();
        if let Some(user) = &self.username_override {
            host.user = user.clone();
        }
        host
    }
}

/// Resolves `host`'s references into an [`AuthPlan`].
///
/// Blocking (config file + secret store) — call from `spawn_blocking`.
///
/// Priority, mirroring the plan document: a referenced identity decides the
/// username and either its keychain key or its saved password; a bare `key_id`
/// goes straight to the keychain. Without references, or when a reference is
/// broken, the plan is empty and the existing auth path applies unchanged.
pub fn resolve(host: &Host) -> AuthPlan {
    let mut plan = AuthPlan::empty();
    if host.identity_id.is_empty() && host.key_id.is_empty() {
        return plan;
    }
    let file = match identities::load_identities() {
        Ok(file) => file,
        Err(e) => {
            tracing::warn!("host {}: cannot read identities.toml: {e}", host.name);
            return plan;
        }
    };
    let Some(dir) = platform::app_config_dir() else {
        tracing::warn!("host {}: no app config directory, keychain auth skipped", host.name);
        return plan;
    };

    let identity = if host.identity_id.is_empty() {
        None
    } else {
        match file.identities.iter().find(|i| i.id == host.identity_id) {
            Some(identity) => {
                plan.username_override = Some(identity.username.clone());
                plan.identity_id = Some(identity.id.clone());
                Some(identity.clone())
            }
            None => {
                tracing::warn!(
                    "host {} references identity {} which no longer exists",
                    host.name,
                    host.identity_id
                );
                None
            }
        }
    };

    // The key entry to offer: the identity's, else the host's own reference.
    let key_entry_id = match &identity {
        Some(identity) => {
            if identity.key_id.is_empty() {
                // The identity is the credential: its saved password pre-read.
                plan.identity_password = secrets::load_identity_password(&dir, &identity.id);
                None
            } else {
                Some(identity.key_id.clone())
            }
        }
        None => (!host.key_id.is_empty()).then(|| host.key_id.clone()),
    };

    if let Some(entry_id) = key_entry_id {
        match keychain_key(&dir, &file, &entry_id) {
            Ok(Some(keychain)) => plan.keychain = Some(keychain),
            Ok(None) => {}
            Err(e) => tracing::warn!("host {}: keychain key unusable: {e}", host.name),
        }
    }
    plan
}

/// Loads and decodes the key entry `entry_id`. `Ok(None)` = the host has no
/// usable key material (missing from the store); `Err` = broken state worth a
/// warning. Both leave the caller with no keychain key.
fn keychain_key(
    dir: &std::path::Path,
    file: &IdentitiesFile,
    entry_id: &str,
) -> Result<Option<KeychainKey>, String> {
    let entry = file
        .keys
        .iter()
        .find(|k| k.id == entry_id)
        .ok_or_else(|| format!("key entry {entry_id} no longer exists"))?;
    let pem = secrets::load_key_pem(dir, entry_id).ok_or_else(|| {
        format!(
            "the PEM of key entry {} is missing from the secret store",
            entry.label
        )
    })?;
    let passphrase = secrets::load_key_passphrase(dir, entry_id);
    let key = russh::keys::decode_secret_key(&pem, passphrase.as_deref())
        .map_err(|e| format!("cannot decode the stored PEM of {}: {e}", entry.label))?;
    identities::check_supported(&key)?;
    Ok(Some(KeychainKey {
        entry_id: entry.id.clone(),
        label: entry.label.clone(),
        key,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use russh::keys::HashAlg;

    use crate::config::identities::{save_identity, save_key_entry};
    use crate::utils::platform::{set_app_config_dir_for_tests, test_config_lock};

    const PLAIN_PEM: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACD5iuy9V3jPnAmg20pxpgAMH95viExt85iQsIdBNMXqIgAAAJBkHR5lZB0e\nZQAAAAtzc2gtZWQyNTUxOQAAACD5iuy9V3jPnAmg20pxpgAMH95viExt85iQsIdBNMXqIg\nAAAEDMG2GI1sLG3+8qJpAil5/OVgKa8KWkwBRtozzRa6eHR/mK7L1XeM+cCaDbSnGmAAwf\n3m+ITG3zmJCwh0E0xeoiAAAADG9tbnlzc2hAdGVzdAE=\n-----END OPENSSH PRIVATE KEY-----\n";

    fn with_scratch(f: impl FnOnce()) {
        let _guard = test_config_lock();
        let dir = std::env::temp_dir().join(format!(
            "omnyssh-authresolve-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        set_app_config_dir_for_tests(Some(dir.clone()));
        f();
        set_app_config_dir_for_tests(None);
        std::fs::remove_dir_all(&dir).ok();
    }

    fn host(name: &str, key_id: &str, identity_id: &str) -> Host {
        Host {
            name: name.to_string(),
            user: "ignored".to_string(),
            key_id: key_id.to_string(),
            identity_id: identity_id.to_string(),
            ..Host::default()
        }
    }

    #[test]
    fn a_referenced_identity_decides_username_and_credential() {
        with_scratch(|| {
            let key = save_key_entry(None, "k", Some(PLAIN_PEM), None).unwrap();
            let bound = save_identity(None, "vm-key", "deploy", None, Some(&key.id)).unwrap();
            let keyless = save_identity(None, "vm-pw", "root", Some("idpw"), None).unwrap();

            // An identity with a key: username override + the decoded key.
            let plan = resolve(&host("web", "", &bound.id));
            assert_eq!(plan.username_override.as_deref(), Some("deploy"));
            let kc = plan.keychain.as_ref().expect("keychain key");
            assert_eq!(kc.entry_id, key.id);
            assert_eq!(kc.key.public_key().fingerprint(HashAlg::Sha256).to_string(), key.fingerprint);
            assert!(plan.identity_password.is_none());
            // The username override lands on the dialed host.
            let dialled = plan.apply_to(&host("web", "", &bound.id));
            assert_eq!(dialled.user, "deploy");

            // A key-less identity: username override + its saved password.
            let plan = resolve(&host("web", "", &keyless.id));
            assert_eq!(plan.username_override.as_deref(), Some("root"));
            assert_eq!(plan.identity_password.as_deref(), Some("idpw"));
            assert!(plan.keychain.is_none());
        });
    }

    #[test]
    fn a_bare_key_reference_resolves_without_touching_the_username() {
        with_scratch(|| {
            let key = save_key_entry(None, "k", Some(PLAIN_PEM), None).unwrap();
            let plan = resolve(&host("web", &key.id, ""));
            assert!(plan.username_override.is_none());
            assert!(plan.keychain.is_some());
            assert!(plan.identity_password.is_none());
        });
    }

    #[test]
    fn broken_references_fail_soft_into_an_empty_plan() {
        with_scratch(|| {
            // A dangling identity reference: nothing to apply.
            let plan = resolve(&host("web", "", "ghost"));
            assert_eq!(plan.username_override, None);
            assert!(plan.keychain.is_none());
            assert!(plan.identity_password.is_none());
            // So does a dangling key reference.
            let plan = resolve(&host("web", "ghost", ""));
            assert!(plan.keychain.is_none());
            // And a host with no references at all.
            assert!(resolve(&host("web", "", "")).keychain.is_none());
        });
    }
}

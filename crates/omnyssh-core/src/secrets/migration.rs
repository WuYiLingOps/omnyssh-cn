//! One-time migration of plaintext passwords out of `hosts.toml`.
//!
//! Older versions serialized `Host.password` straight into `hosts.toml`. On
//! startup every remaining plaintext password is moved into the secret store;
//! the field is then stripped from the file. Idempotent: a file without any
//! `password` field is left untouched, and a password whose store write fails
//! keeps its TOML field so the next startup retries. A password is never
//! dropped because its migration failed.

use std::path::Path;

use crate::secrets::store::save_secret;
use crate::utils::platform;

/// Moves every plaintext `password` in `hosts.toml` into the secret store and
/// rewrites the file clean. Returns how many passwords were migrated.
///
/// Blocking (keychain + file I/O) — call once before the first host load at
/// startup.
pub fn migrate_plaintext_passwords() -> usize {
    match (platform::app_config_dir(), platform::hosts_config_path()) {
        (Some(dir), Some(path)) => migrate_in(&dir, &path),
        _ => 0,
    }
}

/// The migration proper, over an explicit config dir and hosts file (unit
/// testable).
fn migrate_in(dir: &Path, path: &Path) -> usize {
    let Ok(content) = std::fs::read_to_string(path) else {
        // A missing or unreadable file is not a migration problem: the first
        // is "nothing to do", the second fails the regular host load anyway.
        return 0;
    };

    // Parsed untyped on purpose: tolerate any field order or extra fields, and
    // only touch what this migration owns.
    let mut file: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("hosts.toml does not parse, skipping password migration: {e}");
            return 0;
        }
    };
    let Some(hosts) = file.get_mut("hosts").and_then(|v| v.as_array_mut()) else {
        return 0;
    };

    let mut migrated = 0usize;
    for host in hosts.iter_mut() {
        let Some(table) = host.as_table_mut() else {
            continue;
        };
        let Some(password) = table
            .get("password")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(name) = table
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        else {
            tracing::warn!("hosts.toml has a password field on a nameless host, leaving it");
            continue;
        };
        match save_secret(dir, &crate::secrets::login_key(&name), &password) {
            Ok(_) => {
                table.remove("password");
                migrated += 1;
                tracing::info!("migrated the saved password of host {name} out of hosts.toml");
            }
            Err(e) => {
                // Keep the plaintext field: the next startup retries. Failing
                // the migration must never cost the password.
                tracing::warn!("could not migrate the password of host {name}, will retry: {e}");
            }
        }
    }

    if migrated > 0 {
        let text = match toml::to_string_pretty(&file) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("failed to serialize the cleaned hosts.toml: {e}");
                return migrated;
            }
        };
        let tmp = path.with_extension("toml.tmp");
        let write = std::fs::write(&tmp, &text).and_then(|()| std::fs::rename(&tmp, path));
        if let Err(e) = write {
            // The passwords are already in the store, so this only costs a
            // retry of the (idempotent) cleanup pass.
            tracing::warn!("failed to rewrite hosts.toml after migration: {e}");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
    }

    migrated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{delete_login_password, load_login_password};

    fn scratch(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "omnyssh-migration-test-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hosts.toml");
        (dir, path)
    }

    /// A host name unique to this test run: the keychain backend is real, so
    /// migrated test passwords must never collide with a real install's
    /// entries, and every test deletes its own at the end.
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
    fn plaintext_passwords_move_to_the_store_and_out_of_the_file() {
        let (dir, path) = scratch("basic");
        let host = unique_host("web");
        std::fs::write(
            &path,
            format!(
                "hosts = [\n  {{ name = \"{host}\", hostname = \"10.0.0.1\", password = \"pw-1\" }},\n]\n"
            ),
        )
        .unwrap();

        assert_eq!(migrate_in(&dir, &path), 1);
        assert_eq!(load_login_password(&dir, &host).as_deref(), Some("pw-1"));
        let cleaned = std::fs::read_to_string(&path).unwrap();
        assert!(!cleaned.contains("password"), "{cleaned}");
        // Idempotent: a second run finds nothing to do.
        assert_eq!(migrate_in(&dir, &path), 0);
        delete_login_password(&dir, &host);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn multiple_hosts_migrate_and_one_bad_entry_does_not_block_the_rest() {
        let (dir, path) = scratch("multi");
        // The nameless host keeps its field (nothing to route it by); the
        // named ones migrate.
        let a = unique_host("a");
        let b = unique_host("b");
        std::fs::write(
            &path,
            format!(
                "hosts = [\n  \
                 {{ hostname = \"10.0.0.9\", password = \"orphan\" }},\n  \
                 {{ name = \"{a}\", hostname = \"10.0.0.1\", password = \"pw-a\" }},\n  \
                 {{ name = \"{b}\", hostname = \"10.0.0.2\", password = \"pw-b\" }},\n]\n"
            ),
        )
        .unwrap();

        assert_eq!(migrate_in(&dir, &path), 2);
        assert_eq!(load_login_password(&dir, &a).as_deref(), Some("pw-a"));
        assert_eq!(load_login_password(&dir, &b).as_deref(), Some("pw-b"));
        let cleaned = std::fs::read_to_string(&path).unwrap();
        assert!(cleaned.contains("orphan"), "the unroutable entry stays");
        delete_login_password(&dir, &a);
        delete_login_password(&dir, &b);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_file_without_passwords_is_untouched() {
        let (dir, path) = scratch("clean");
        let content = "hosts = [\n  { name = \"web\", hostname = \"10.0.0.1\" },\n]\n";
        std::fs::write(&path, content).unwrap();

        assert_eq!(migrate_in(&dir, &path), 0);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            content,
            "the file must stay byte-identical"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_is_nothing_to_do() {
        let (dir, path) = scratch("missing");
        assert_eq!(migrate_in(&dir, &path), 0);
        std::fs::remove_dir_all(&dir).ok();
    }
}

//! GitHub Gist cloud sync. The engine lives in the frontend; Phase 1 syncs
//! manual hosts + snippets, with the TUI sharing the benefit through
//! hosts.toml.
//!
//! The sync engine itself lives in the frontend (`ui/src/lib/sync/`); this
//! module is the Rust bridge it needs:
//!
//! - [`github_sync`] — GitHub Device Flow auth (RFC 8628), the Gist raw-content
//!   fallback download, and token / sync-password storage in the core secret
//!   store (`omnyssh_core::secrets::store` — system keychain first, machine-bound
//!   encrypted file fallback). Secrets never land in a plaintext file.
//! - [`payload`] — full-fidelity host export/import for the sync payload. The
//!   regular `HostDto` omits secrets and key paths (tech-gui.md §3.4); the sync
//!   payload needs the complete manual-host record instead, with passwords
//!   included only when the user opts in.

pub mod github_sync;
pub mod payload;

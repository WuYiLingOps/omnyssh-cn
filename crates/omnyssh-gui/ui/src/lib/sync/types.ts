/**
 * Cloud sync domain types and constants — OmnySSH flavour (the engine is
 * ported from DockPilot's Gist encrypted sync, itself derived from Netcatty).
 *
 * Phase 1 payload: manual hosts + snippets only. Settings are out of scope —
 * the TUI and GUI keep different settings models, so a whitelist sync would
 * drag in mapping work for little gain.
 */

import type { HostSyncDto, SnippetDto, IdentitySyncDto, KeySyncDto } from "$lib/bindings";

// ============================================================================
// State machines
// ============================================================================

/** Security state: whether a sync password exists / is unlocked (memory only). */
export type SecurityState = "NO_KEY" | "LOCKED" | "UNLOCKED";

/** Sync operation state machine. */
export type SyncState = "IDLE" | "SYNCING" | "CONFLICT" | "BLOCKED" | "ERROR";

/** Conflict resolution strategy. */
export type ConflictResolution = "USE_REMOTE" | "USE_LOCAL";

// ============================================================================
// Encrypted cloud file structure (the content of omnyssh-vault.json in the Gist)
// ============================================================================

/** Plaintext metadata (for version comparison and decryption parameters). */
export interface SyncFileMeta {
  /** Every upload = remote version + 1. */
  version: number;
  /** Unix milliseconds. */
  updatedAt: number;
  deviceId: string;
  deviceName?: string;
  appVersion: string;
  /** AES-GCM initialisation vector (Base64). */
  iv: string;
  /** KDF salt (Base64). */
  salt: string;
  algorithm: "AES-256-GCM";
  kdf: "PBKDF2";
  kdfIterations: number;
}

/** The full Gist file structure. */
export interface SyncedFile {
  meta: SyncFileMeta;
  /** Base64(AES-GCM(JSON(SyncPayload))) */
  payload: string;
}

/** A synced host — exactly what the Rust `HostSyncDto` carries (camelCase).
 *  `password` is `null`/absent unless the user enabled `syncPasswords`. */
export type SyncedHost = HostSyncDto;

/** A synced snippet — the same shape `snippets.toml` holds, keyed on `name`. */
export type SyncedSnippet = SnippetDto;

/** A synced keychain key entry — fingerprint-keyed (`keyFp`), materials only
 *  under the `syncCredentials` switch. */
export type SyncedKey = KeySyncDto;

/** A synced login identity — label-keyed; the password only under the switch. */
export type SyncedIdentity = IdentitySyncDto;

/** Decrypted sync payload. Manual hosts + snippets always travel; identity
 *  metadata (keys + identities, fingerprint-keyed) travels so a synced host's
 *  key pin resolves on a new device; secret materials (PEM / passphrases /
 *  identity passwords) only under the `syncCredentials` switch. */
export interface SyncPayload {
  /** Manual entries only — SSH-config imports are re-derived per device. */
  hosts: SyncedHost[];
  snippets: SyncedSnippet[];
  /** Keychain entries, fingerprint-keyed; `pem`/`passphrase` absent unless
   *  the credentials switch is on. */
  keys: SyncedKey[];
  /** Login identities, label-keyed; `password` absent unless the switch is on. */
  identities: SyncedIdentity[];
  /** Stripped before fingerprinting. */
  syncedAt: number;
}

// ============================================================================
// Anchor and guards
// ============================================================================

/** Remote anchor: the last observed cloud file signature (localStorage). */
export interface ProviderSyncAnchor {
  /** Base64 of SHA-256(meta + ciphertext); null when unreadable. */
  signature: string | null;
  version: number;
  updatedAt: number;
  deviceId?: string;
  /** Gist id. */
  resourceId: string | null;
  observedAt: number;
}

export type SyncEntityType = "hosts" | "snippets" | "keys" | "identities";

export type ShrinkFinding =
  | { suspicious: false }
  | {
      suspicious: true;
      reason: "bulk-shrink" | "large-shrink";
      entityType: SyncEntityType;
      baseCount: number;
      outgoingCount: number;
      lost: number;
      /** base was missing; the remote payload served as the reference. */
      viaRemote?: boolean;
    };

// ============================================================================
// Sync result and conflict info
// ============================================================================

export interface SyncResult {
  success: boolean;
  action: "upload" | "download" | "merge" | "none";
  version?: number;
  error?: string;
  /** Decryption failed (the two devices use different sync passwords); the
   *  user must choose remote / local. */
  conflictDetected?: boolean;
  /** The payload a successful merge produced (already applied locally). */
  mergedPayload?: SyncPayload;
  /** Blocked by the shrink guard. */
  shrinkBlocked?: boolean;
  finding?: ShrinkFinding;
  /** Both-sides-modified fields the merge resolved local-first — informational,
   *  the sync itself succeeded; the UI surfaces it so a remote edit isn't
   *  silently swallowed. */
  mergeConflicts?: number;
}

export interface ConflictInfo {
  localVersion: number;
  localUpdatedAt: number;
  localDeviceName?: string;
  remoteVersion: number;
  remoteUpdatedAt: number;
  remoteDeviceName?: string;
}

// ============================================================================
// Master key (sync password)
// ============================================================================

/** Persisted master key config (localStorage; never contains the password). */
export interface MasterKeyConfig {
  /** Base64(SHA-256(derived key)) for unlock verification. */
  verificationHash: string;
  /** Base64 KDF salt. */
  salt: string;
  kdf: "PBKDF2";
  kdfIterations: number;
  createdAt: number;
}

/** Unlocked key state (memory only). */
export interface UnlockedMasterKey {
  derivedKey: CryptoKey;
  salt: Uint8Array;
  unlockedAt: number;
}

// ============================================================================
// GitHub auth and account
// ============================================================================

export interface DeviceFlowStart {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  expiresAt: number;
  interval: number;
}

export interface OAuthTokens {
  accessToken: string;
  tokenType: string;
  scope?: string;
}

export interface ProviderAccount {
  id: string;
  login?: string;
  name?: string;
  email?: string;
  avatarUrl?: string;
}

// ============================================================================
// Local sync config (localStorage)
// ============================================================================

export interface SyncConfig {
  /** Auto sync switch. */
  autoSync: boolean;
  /** Whether the payload carries login passwords (default off; envelope-encrypted). */
  syncPasswords: boolean;
  /** Whether the payload carries the SSH credential materials — key PEMs,
   *  key passphrases, identity passwords (default off; envelope-encrypted).
   *  Identity *metadata* travels regardless: it is non-sensitive and makes a
   *  synced host's key pin resolve. */
  syncCredentials: boolean;
  /** The discovered sync Gist id. */
  gistId: string | null;
  /** Where the GitHub token landed (keyring / encrypted file), set by Rust. */
  tokenBackend: "keyring" | "file" | null;
  /** Where the remembered sync password landed. */
  passwordBackend: "keyring" | "file" | null;
  /** OAuth account info (display only). */
  account: ProviderAccount | null;
  /** Last successful sync time (including consistency checks; persisted). */
  lastSyncAt: number | null;
  /** The cloud version the last sync reached. */
  lastSyncVersion: number | null;
}

export const DEFAULT_SYNC_CONFIG: SyncConfig = {
  autoSync: true,
  syncPasswords: false,
  syncCredentials: false,
  gistId: null,
  tokenBackend: null,
  passwordBackend: null,
  account: null,
  lastSyncAt: null,
  lastSyncVersion: null,
};

// ============================================================================
// Storage keys and constants
// ============================================================================

export const SYNC_STORAGE_KEYS = {
  SYNC_CONFIG: "omnyssh.sync.config",
  MASTER_KEY_CONFIG: "omnyssh.sync.masterKeyConfig",
  BASE_PAYLOAD: "omnyssh.sync.base",
  ANCHOR: "omnyssh.sync.anchor",
  DEVICE: "omnyssh.sync.device",
} as const;

// A literal member access on purpose: Vite statically substitutes only
// `import.meta.env.VITE_X` spelled out like this — a dynamic `import.meta.env[key]`
// survives into the production bundle, where no runtime env object exists, and
// always reads as undefined.
const GITHUB_CLIENT_ID = import.meta.env.VITE_SYNC_GITHUB_CLIENT_ID?.trim() ?? "";

export const SYNC_CONSTANTS = {
  // Encryption
  AES_KEY_LENGTH: 256,
  GCM_IV_LENGTH: 12,
  GCM_TAG_LENGTH: 128,
  SALT_LENGTH: 32,

  // PBKDF2 (OWASP-recommended floor)
  PBKDF2_ITERATIONS: 600000,
  PBKDF2_HASH: "SHA-256",

  // Gist
  SYNC_FILE_NAME: "omnyssh-vault.json",
  GIST_DESCRIPTION: "OmnySSH Encrypted Vault (DO NOT EDIT MANUALLY)",

  // GitHub — OmnySSH's own OAuth App with Device Flow enabled
  GITHUB_CLIENT_ID,
  GITHUB_API_BASE: "https://api.github.com",
  GITHUB_DEVICE_URL: "https://github.com/login/device",

  SYNC_DEVICE_NAME_PREFIX: "OmnySSH",
} as const;

// ============================================================================
// Helpers
// ============================================================================

export const generateDeviceId = (): string => crypto.randomUUID();

export const getDefaultDeviceName = (): string => {
  const platform = navigator.platform || "Unknown";
  return `${SYNC_CONSTANTS.SYNC_DEVICE_NAME_PREFIX} (${platform})`;
};

export interface DeviceInfo {
  deviceId: string;
  deviceName: string;
}

/** Reads or initialises the device identity. */
export const loadDeviceInfo = (): DeviceInfo => {
  try {
    const raw = localStorage.getItem(SYNC_STORAGE_KEYS.DEVICE);
    if (raw) {
      const parsed = JSON.parse(raw) as DeviceInfo;
      if (parsed.deviceId && parsed.deviceName) return parsed;
    }
  } catch {
    // Corrupted: rebuild
  }
  const info: DeviceInfo = { deviceId: generateDeviceId(), deviceName: getDefaultDeviceName() };
  try {
    localStorage.setItem(SYNC_STORAGE_KEYS.DEVICE, JSON.stringify(info));
  } catch {
    // localStorage unavailable: rebuild per session
  }
  return info;
};

/** Whether the payload carries any real entity data (empty-vault guard). */
export const hasSyncPayloadEntityData = (payload: SyncPayload): boolean =>
  payload.hosts.length > 0 ||
  payload.snippets.length > 0 ||
  payload.keys.length > 0 ||
  payload.identities.length > 0;

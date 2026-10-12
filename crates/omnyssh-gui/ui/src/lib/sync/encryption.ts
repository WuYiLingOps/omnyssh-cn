/**
 * Sync encryption service: PBKDF2 (600k, SHA-256) → AES-256-GCM
 * (ported from DockPilot's sync engine, itself from Netcatty).
 *
 * Security model:
 * - The sync password derives the AES-256 key via PBKDF2; the password never
 *   lands in any storage.
 * - Every encryption uses a fresh random salt + IV.
 * - `verificationHash` (SHA-256 of the derived key) verifies password
 *   correctness without storing the key.
 */

import { SYNC_CONSTANTS, type MasterKeyConfig, type SyncedFile, type SyncPayload, type UnlockedMasterKey } from "./types";

// ============================================================================
// Basics
// ============================================================================

/** TS 5.x requires an explicit ArrayBuffer cast (Uint8Array<ArrayBufferLike> → BufferSource). */
const toArrayBuffer = (bytes: Uint8Array): ArrayBuffer =>
  bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;

export const arrayBufferToBase64 = (buffer: ArrayBuffer | Uint8Array): string => {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
};

export const base64ToUint8Array = (base64: string): Uint8Array => {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
};

export const generateRandomBytes = (length: number): Uint8Array =>
  crypto.getRandomValues(new Uint8Array(length));

export const sha256 = async (data: Uint8Array): Promise<Uint8Array> => {
  const hashBuffer = await crypto.subtle.digest("SHA-256", toArrayBuffer(data));
  return new Uint8Array(hashBuffer);
};

const stringToBytes = (str: string): Uint8Array => new TextEncoder().encode(str);
const bytesToString = (bytes: Uint8Array): string => new TextDecoder().decode(bytes);

// ============================================================================
// Key derivation
// ============================================================================

export const deriveKey = async (
  password: string,
  salt: Uint8Array,
  iterations: number = SYNC_CONSTANTS.PBKDF2_ITERATIONS,
): Promise<CryptoKey> => {
  const passwordKey = await crypto.subtle.importKey(
    "raw",
    toArrayBuffer(stringToBytes(password)),
    "PBKDF2",
    false,
    ["deriveBits", "deriveKey"],
  );

  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: toArrayBuffer(salt),
      iterations,
      hash: SYNC_CONSTANTS.PBKDF2_HASH,
    },
    passwordKey,
    { name: "AES-GCM", length: SYNC_CONSTANTS.AES_KEY_LENGTH },
    true, // exportable, so the verification hash can be computed
    ["encrypt", "decrypt"],
  );
};

const exportKey = async (key: CryptoKey): Promise<Uint8Array> =>
  new Uint8Array(await crypto.subtle.exportKey("raw", key));

/** Verification hash from the derived key (checks password correctness without storing it). */
export const createVerificationHash = async (derivedKey: CryptoKey): Promise<string> =>
  arrayBufferToBase64(await sha256(await exportKey(derivedKey)));

export const verifyPassword = async (password: string, config: MasterKeyConfig): Promise<boolean> => {
  try {
    const salt = base64ToUint8Array(config.salt);
    const derivedKey = await deriveKey(password, salt, config.kdfIterations || SYNC_CONSTANTS.PBKDF2_ITERATIONS);
    return (await createVerificationHash(derivedKey)) === config.verificationHash;
  } catch {
    return false;
  }
};

// ============================================================================
// Encrypt / decrypt
// ============================================================================

const encryptRaw = async (plaintext: string, key: CryptoKey): Promise<{ ciphertext: Uint8Array; iv: Uint8Array }> => {
  const iv = generateRandomBytes(SYNC_CONSTANTS.GCM_IV_LENGTH);
  const ciphertextBuffer = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: toArrayBuffer(iv), tagLength: SYNC_CONSTANTS.GCM_TAG_LENGTH },
    key,
    toArrayBuffer(stringToBytes(plaintext)),
  );
  return { ciphertext: new Uint8Array(ciphertextBuffer), iv };
};

const decryptRaw = async (ciphertext: Uint8Array, iv: Uint8Array, key: CryptoKey): Promise<string> => {
  const plaintextBuffer = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: toArrayBuffer(iv), tagLength: SYNC_CONSTANTS.GCM_TAG_LENGTH },
    key,
    toArrayBuffer(ciphertext),
  );
  return bytesToString(new Uint8Array(plaintextBuffer));
};

// ============================================================================
// High-level API: SyncPayload ↔ SyncedFile
// ============================================================================

/** Encrypts the sync payload into an uploadable SyncedFile (fresh salt + IV
 *  every time; version = existing + 1). */
export const encryptPayload = async (
  payload: SyncPayload,
  password: string,
  deviceId: string,
  deviceName: string,
  appVersion: string,
  existingVersion?: number,
): Promise<SyncedFile> => {
  const salt = generateRandomBytes(SYNC_CONSTANTS.SALT_LENGTH);
  const key = await deriveKey(password, salt);
  const encrypted = await encryptRaw(JSON.stringify(payload), key);

  return {
    meta: {
      version: (existingVersion || 0) + 1,
      updatedAt: Date.now(),
      deviceId,
      deviceName,
      appVersion,
      iv: arrayBufferToBase64(encrypted.iv),
      salt: arrayBufferToBase64(salt),
      algorithm: "AES-256-GCM",
      kdf: "PBKDF2",
      kdfIterations: SYNC_CONSTANTS.PBKDF2_ITERATIONS,
    },
    payload: arrayBufferToBase64(encrypted.ciphertext),
  };
};

/** Decrypts a cloud SyncedFile; a wrong password throws (the caller routes to
 *  the conflict UI). */
export const decryptPayload = async (syncedFile: SyncedFile, password: string): Promise<SyncPayload> => {
  const { meta, payload } = syncedFile;
  const salt = base64ToUint8Array(meta.salt);
  const iv = base64ToUint8Array(meta.iv);
  const ciphertext = base64ToUint8Array(payload);

  const key = await deriveKey(password, salt, meta.kdfIterations || SYNC_CONSTANTS.PBKDF2_ITERATIONS);
  const decrypted = await decryptRaw(ciphertext, iv, key);
  return JSON.parse(decrypted) as SyncPayload;
};

/** Only checks that a cloud file decrypts with the given password. */
export const verifySyncedFile = async (syncedFile: SyncedFile, password: string): Promise<boolean> => {
  try {
    await decryptPayload(syncedFile, password);
    return true;
  } catch {
    return false;
  }
};

// ============================================================================
// Master key management
// ============================================================================

/** First-time sync password setup: generates the persisted config
 *  (verificationHash + salt, no password). */
export const createMasterKeyConfig = async (password: string): Promise<MasterKeyConfig> => {
  const salt = generateRandomBytes(SYNC_CONSTANTS.SALT_LENGTH);
  const key = await deriveKey(password, salt);
  return {
    verificationHash: await createVerificationHash(key),
    salt: arrayBufferToBase64(salt),
    kdf: "PBKDF2",
    kdfIterations: SYNC_CONSTANTS.PBKDF2_ITERATIONS,
    createdAt: Date.now(),
  };
};

/** Unlock: verifies the password and returns the memory-only key state. */
export const unlockMasterKey = async (
  password: string,
  config: MasterKeyConfig,
): Promise<UnlockedMasterKey | null> => {
  if (!(await verifyPassword(password, config))) return null;
  const salt = base64ToUint8Array(config.salt);
  const derivedKey = await deriveKey(password, salt, config.kdfIterations || SYNC_CONSTANTS.PBKDF2_ITERATIONS);
  return { derivedKey, salt, unlockedAt: Date.now() };
};

/** Changes the sync password (the next sync re-uploads everything under the
 *  new password). */
export const changeMasterPassword = async (
  oldPassword: string,
  newPassword: string,
  config: MasterKeyConfig,
): Promise<MasterKeyConfig | null> => {
  if (!(await verifyPassword(oldPassword, config))) return null;
  return createMasterKeyConfig(newPassword);
};

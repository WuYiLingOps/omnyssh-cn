/**
 * GitHub auth and token storage (ported from DockPilot's sync engine).
 *
 * Device Flow (RFC 8628) needs only the OAuth App's client_id:
 *   1. `github_device_flow_start` returns a device_code + user_code the user
 *      enters in the browser
 *   2. poll `github_device_flow_poll` per the server interval, handling
 *      authorization_pending / slow_down (+5s) / expired_token / access_denied
 *   3. the token is stored in the system keychain (Rust secret store); the
 *      actual backend is returned for the caller to record
 */

import { invoke } from "@tauri-apps/api/core";
import { SYNC_CONSTANTS, type DeviceFlowStart, type OAuthTokens, type ProviderAccount } from "./types";

// ---------------------------------------------------------------------------
// Device Flow
// ---------------------------------------------------------------------------

export const startDeviceFlow = async (clientId = SYNC_CONSTANTS.GITHUB_CLIENT_ID): Promise<DeviceFlowStart> => {
  return invoke<DeviceFlowStart>("github_device_flow_start", { clientId, scope: "gist read:user" });
};

export interface DeviceFlowPollEvents {
  onPending?: () => void;
  /** Notified after a slow_down response (the interval has grown by 5s). */
  onSlowDown?: () => void;
}

/**
 * Polls until a token arrives, the user denies, the code expires, or the user
 * aborts (signal). Honours the server interval, floored at 5 seconds; +5s on
 * slow_down.
 */
export const pollForToken = async (
  start: DeviceFlowStart,
  events: DeviceFlowPollEvents = {},
  signal?: AbortSignal,
): Promise<OAuthTokens> => {
  const clientId = SYNC_CONSTANTS.GITHUB_CLIENT_ID;
  let intervalMs = Math.max(start.interval, 5) * 1000;

  const sleep = (ms: number) =>
    new Promise<void>((resolve, reject) => {
      const timer = setTimeout(resolve, ms);
      signal?.addEventListener(
        "abort",
        () => {
          clearTimeout(timer);
          reject(new DOMException("GitHub auth cancelled", "AbortError"));
        },
        { once: true },
      );
    });

  while (Date.now() < start.expiresAt) {
    await sleep(intervalMs);
    if (signal?.aborted) throw new DOMException("GitHub auth cancelled", "AbortError");

    // A single poll's network failure (transient timeout etc.) does not abort
    // the flow: treat it as pending and wait for the next round. The Rust
    // command returns the raw response body (pending and success both pass
    // through verbatim).
    const raw = await invoke<string>("github_device_flow_poll", {
      clientId,
      deviceCode: start.deviceCode,
    }).catch(() => null);
    if (!raw) {
      events.onPending?.();
      continue;
    }

    let data: Record<string, unknown>;
    try {
      data = JSON.parse(raw) as Record<string, unknown>;
    } catch {
      events.onPending?.();
      continue;
    }

    if (typeof data.access_token === "string" && data.access_token) {
      return {
        accessToken: data.access_token,
        tokenType: typeof data.token_type === "string" ? data.token_type : "bearer",
        scope: typeof data.scope === "string" ? data.scope : undefined,
      };
    }

    const error = typeof data.error === "string" ? data.error : undefined;
    if (error === "authorization_pending") {
      events.onPending?.();
      continue;
    }
    if (error === "slow_down") {
      intervalMs += 5000;
      events.onSlowDown?.();
      continue;
    }
    if (error === "expired_token") {
      throw new Error("The device code has expired; reconnect to try again");
    }
    if (error === "access_denied") {
      throw new Error("The user denied the authorization");
    }
    if (error) {
      const desc = typeof data.error_description === "string" ? data.error_description : error;
      throw new Error(`GitHub auth failed: ${desc}`);
    }
  }

  throw new Error("The device code has expired; reconnect to try again");
};

// ---------------------------------------------------------------------------
// User info
// ---------------------------------------------------------------------------

interface GitHubUser {
  id: number;
  login: string;
  name: string | null;
  email: string | null;
  avatar_url: string;
}

/** GET /user (api.github.com has CORS; fetched directly). */
export const getUserInfo = async (accessToken: string): Promise<ProviderAccount> => {
  const res = await fetch(`${SYNC_CONSTANTS.GITHUB_API_BASE}/user`, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
      Accept: "application/vnd.github.v3+json",
    },
  });
  if (!res.ok) throw new Error(`Failed to fetch the GitHub account info: ${res.status} ${res.statusText}`);
  const user = (await res.json()) as GitHubUser;
  return {
    id: String(user.id),
    login: user.login,
    name: user.name || user.login,
    email: user.email || undefined,
    avatarUrl: user.avatar_url,
  };
};

/** Checks whether the token is still valid. */
export const validateToken = async (accessToken: string): Promise<boolean> => {
  try {
    const res = await fetch(`${SYNC_CONSTANTS.GITHUB_API_BASE}/user`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return res.ok;
  } catch {
    return false;
  }
};

// ---------------------------------------------------------------------------
// Token persistence (system keychain / encrypted file; the caller records the
// backend in the sync config)
// ---------------------------------------------------------------------------

export type TokenBackend = "keyring" | "file";

export const saveToken = async (token: string): Promise<TokenBackend> =>
  invoke<TokenBackend>("sync_save_github_token", { token });

export const loadToken = async (backend: TokenBackend): Promise<string | null> =>
  invoke<string | null>("sync_load_github_token", { backend });

export const deleteToken = async (backend: TokenBackend): Promise<void> => {
  await invoke("sync_delete_github_token", { backend });
};

// ---------------------------------------------------------------------------
// Sync password persistence (remember-password: saved on unlock, cleared on lock)
// ---------------------------------------------------------------------------

export const saveSyncPassword = async (password: string): Promise<TokenBackend> =>
  invoke<TokenBackend>("sync_save_sync_password", { password });

export const loadSyncPassword = async (backend: TokenBackend): Promise<string | null> =>
  invoke<string | null>("sync_load_sync_password", { backend });

export const deleteSyncPassword = async (backend: TokenBackend): Promise<void> => {
  await invoke("sync_delete_sync_password", { backend });
};

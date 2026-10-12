/**
 * GitHub Gist CRUD (ported from DockPilot's sync engine).
 *
 * api.github.com has CORS, so the WebView fetches it directly; raw content
 * download (the truncated case) goes through the Rust command
 * `github_gist_raw_content` (CORS is uncertain there + origin check).
 * The accessToken only ever flows through memory, loaded from the secret
 * store by the engine.
 */

import { invoke } from "@tauri-apps/api/core";
import { SYNC_CONSTANTS, type SyncedFile } from "./types";

// ---------------------------------------------------------------------------
// GitHub API types
// ---------------------------------------------------------------------------

export interface GitHubGistFile {
  filename: string;
  /** The Gist API's embedded content, truncated around 1 MB (truncated=true). */
  content?: string;
  truncated?: boolean;
  raw_url?: string;
  /** File size in UTF-8 bytes (a JS string's .length counts UTF-16 units). */
  size?: number;
}

interface GitHubGist {
  id: string;
  description: string;
  files: Record<string, GitHubGistFile>;
  updated_at?: string;
  history?: Array<{ version: string; committed_at: string }>;
}

function authHeaders(token: string, extra: Record<string, string> = {}): Record<string, string> {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github.v3+json",
    ...extra,
  };
}

/**
 * WebView fetch has no automatic retry: api.github.com has intermittent
 * timeouts (notably on networks in CN). Network-class failures retry
 * automatically with 1s / 2s gaps — only when the request never went out or
 * the connection dropped mid-flight; HTTP error responses (401/404 etc.) are
 * returned as-is for the caller to handle.
 */
async function fetchWithRetry(url: string, init: RequestInit, what: string): Promise<Response> {
  let lastError: unknown = null;
  for (let attempt = 0; attempt < 3; attempt++) {
    if (attempt > 0) await new Promise((r) => setTimeout(r, 1000 * attempt));
    try {
      return await fetch(url, init);
    } catch (e) {
      lastError = e;
    }
  }
  const detail = lastError instanceof Error ? lastError.message : String(lastError);
  throw new Error(
    `${what} failed: network error (retried 3 times): ${detail}. Try again later; if this machine needs a proxy for GitHub, set the HTTPS_PROXY environment variable and restart the app`,
  );
}

async function gistResponse<T>(res: Response, what: string): Promise<T> {
  if (!res.ok) {
    throw new Error(`${what} failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as T;
}

// ---------------------------------------------------------------------------
// Gist operations
// ---------------------------------------------------------------------------

/** Finds the OmnySSH sync vault in the user's Gist list. */
export async function findSyncGist(accessToken: string): Promise<string | null> {
  const res = await fetchWithRetry(`${SYNC_CONSTANTS.GITHUB_API_BASE}/gists?per_page=100`, { headers: authHeaders(accessToken) }, "fetching the Gist list");
  const gists = await gistResponse<GitHubGist[]>(res, "fetching the Gist list");
  const syncGist = gists.find(
    (g) => g.description === SYNC_CONSTANTS.GIST_DESCRIPTION && g.files[SYNC_CONSTANTS.SYNC_FILE_NAME],
  );
  return syncGist?.id ?? null;
}

/** Creates a secret sync Gist and returns its id. */
export async function createSyncGist(accessToken: string, syncedFile: SyncedFile): Promise<string> {
  const res = await fetchWithRetry(
    `${SYNC_CONSTANTS.GITHUB_API_BASE}/gists`,
    {
      method: "POST",
      headers: authHeaders(accessToken, { "Content-Type": "application/json" }),
      body: JSON.stringify({
        description: SYNC_CONSTANTS.GIST_DESCRIPTION,
        public: false,
        files: { [SYNC_CONSTANTS.SYNC_FILE_NAME]: { content: JSON.stringify(syncedFile, null, 2) } },
      }),
    },
    "creating the sync Gist",
  );
  const gist = await gistResponse<GitHubGist>(res, "creating the sync Gist");
  return gist.id;
}

/** Updates an existing sync Gist. */
export async function updateSyncGist(accessToken: string, gistId: string, syncedFile: SyncedFile): Promise<void> {
  const res = await fetchWithRetry(
    `${SYNC_CONSTANTS.GITHUB_API_BASE}/gists/${gistId}`,
    {
      method: "PATCH",
      headers: authHeaders(accessToken, { "Content-Type": "application/json" }),
      body: JSON.stringify({
        files: { [SYNC_CONSTANTS.SYNC_FILE_NAME]: { content: JSON.stringify(syncedFile, null, 2) } },
      }),
    },
    "updating the sync Gist",
  );
  if (!res.ok) throw new Error(`updating the sync Gist failed: ${res.status} ${res.statusText}`);
}

const utf8ByteLength = (value: string): number => new TextEncoder().encode(value).byteLength;

/** Truncation double-check: the truncated flag, or a size (UTF-8 bytes) larger
 *  than the embedded content's actual byte count. */
function gistFileNeedsRawFetch(file: GitHubGistFile): boolean {
  if (file.truncated) return true;
  if (
    typeof file.size === "number" &&
    typeof file.content === "string" &&
    file.size > utf8ByteLength(file.content)
  ) {
    return true;
  }
  return false;
}

const parseSyncedFile = (raw: string): SyncedFile => {
  const parsed = JSON.parse(raw) as SyncedFile;
  // Structure check: malformed data (a hand-edited Gist / truncation residue)
  // gets a clear error instead of a "meta undefined" TypeError burning all the
  // way into the conflict branch.
  if (
    !parsed ||
    typeof parsed !== "object" ||
    typeof parsed.payload !== "string" ||
    !parsed.meta ||
    typeof parsed.meta.version !== "number" ||
    typeof parsed.meta.salt !== "string" ||
    typeof parsed.meta.iv !== "string"
  ) {
    throw new Error("The cloud sync file is malformed (missing a valid meta/payload; it may have been hand-edited or corrupted)");
  }
  return parsed;
};

/** Downloads one Gist file and parses it into a SyncedFile; falls back to the
 *  Rust raw_url fetch when truncated. */
async function parseSyncedFileFromGistFile(
  accessToken: string,
  file: GitHubGistFile | undefined,
): Promise<SyncedFile | null> {
  if (!file) return null;

  if (gistFileNeedsRawFetch(file)) {
    if (!file.raw_url) {
      throw new Error("The sync file is truncated without a raw_url; the content may exceed the Gist size limit");
    }
    return parseSyncedFile(
      await invoke<string>("github_gist_raw_content", { accessToken, rawUrl: file.raw_url }),
    );
  }

  if (!file.content) return null;
  try {
    return parseSyncedFile(file.content);
  } catch (error) {
    // Defensive path: the embedded JSON is incomplete but truncated is not set.
    if (error instanceof SyntaxError && file.raw_url) {
      return parseSyncedFile(
        await invoke<string>("github_gist_raw_content", { accessToken, rawUrl: file.raw_url }),
      );
    }
    throw error;
  }
}

/** Downloads the sync file; null when the Gist does not exist. */
export async function downloadSyncGist(accessToken: string, gistId: string): Promise<SyncedFile | null> {
  const res = await fetchWithRetry(
    `${SYNC_CONSTANTS.GITHUB_API_BASE}/gists/${gistId}`,
    { headers: authHeaders(accessToken) },
    "downloading the sync file",
  );
  if (res.status === 404) return null;
  const gist = await gistResponse<GitHubGist>(res, "downloading the sync file");
  return parseSyncedFileFromGistFile(accessToken, gist.files[SYNC_CONSTANTS.SYNC_FILE_NAME]);
}

/** Deletes the sync Gist (disconnect / reset). */
export async function deleteSyncGist(accessToken: string, gistId: string): Promise<void> {
  const res = await fetchWithRetry(
    `${SYNC_CONSTANTS.GITHUB_API_BASE}/gists/${gistId}`,
    { method: "DELETE", headers: authHeaders(accessToken) },
    "deleting the sync Gist",
  );
  if (!res.ok && res.status !== 404) {
    throw new Error(`deleting the sync Gist failed: ${res.status} ${res.statusText}`);
  }
}

/** The Gist's built-in revision history (for version rollback), newest first. */
export async function getGistHistory(
  accessToken: string,
  gistId: string,
): Promise<Array<{ version: string; date: Date }>> {
  const res = await fetchWithRetry(
    `${SYNC_CONSTANTS.GITHUB_API_BASE}/gists/${gistId}`,
    { headers: authHeaders(accessToken) },
    "fetching the Gist history",
  );
  const gist = await gistResponse<GitHubGist>(res, "fetching the Gist history");
  return (gist.history ?? []).map((h) => ({ version: h.version, date: new Date(h.committed_at) }));
}

/** Downloads one historical revision's sync file (GET /gists/{id}/{sha}),
 *  handling the truncation fallback too; null when the file is absent. */
export async function downloadGistRevision(
  accessToken: string,
  gistId: string,
  sha: string,
): Promise<SyncedFile | null> {
  const res = await fetchWithRetry(
    `${SYNC_CONSTANTS.GITHUB_API_BASE}/gists/${gistId}/${sha}`,
    { headers: authHeaders(accessToken) },
    "downloading the historical revision",
  );
  if (res.status === 404) return null;
  const gist = await gistResponse<GitHubGist>(res, "downloading the historical revision");
  return parseSyncedFileFromGistFile(accessToken, gist.files[SYNC_CONSTANTS.SYNC_FILE_NAME]);
}

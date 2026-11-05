/**
 * Lyrics providers (P7) — frontend-side.
 *
 * Primary and only wired provider: **LRCLIB** (https://lrclib.net) — free,
 * no API key, returns `syncedLyrics` (LRC text) and/or `plainLyrics`.
 *
 * Behaviour contract:
 * - lookup by `{ artist, title, album?, durationSecs? }`: GET `/api/get`
 *   first (exact metadata query), then GET `/api/search` with the best
 *   duration match as fallback.
 * - in-memory cache keyed `${artist}::${title}` with a 6 h TTL, mirrored to
 *   `localStorage` so lyrics survive offline re-open (latest 50 entries).
 * - client-side rate limiter: max 4 requests/minute, serialised queue,
 *   in-flight duplicates coalesced into one request.
 * - a small descriptive User-Agent is sent so LRCLIB can identify the client.
 *
 * Live LRCLIB response shape coded against (`/api/get` → one object,
 * `/api/search` → array of the same):
 * ```json
 * { "id": 7242265, "name": "Yellow", "trackName": "Yellow",
 *   "artistName": "Coldplay", "albumName": "...", "duration": 271.0,
 *   "instrumental": false, "hasWordSync": false,
 *   "plainLyrics": "…", "syncedLyrics": "[00:33.44] …",
 *   "lyricsfile": "…" }
 * ```
 * 404 on /api/get when there is no match; `duration` is a float in seconds.
 * The extra `name` / `lyricsfile` fields are ignored.
 */

export const LRCLIB_BASE = "https://lrclib.net/api";
export const CLIENT_UA =
  "BoomingDesktop/0.1.0 (https://github.com/davisville/BoomingMusic)";

export interface LyricsQuery {
  artist?: string | null;
  title?: string | null;
  album?: string | null;
  durationSecs?: number | null;
}

export interface LyricsResult {
  found: boolean;
  /** Track is known to be instrumental — render a "no lyrics" notice. */
  instrumental: boolean;
  /** LRC text (may contain embedded enhanced `<mm:ss.xx>` word tags). */
  syncedLyrics: string | null;
  plainLyrics: string | null;
  source: string;
}

interface LrclibRecord {
  id?: number;
  trackName?: string;
  artistName?: string;
  albumName?: string;
  duration?: number;
  instrumental?: boolean;
  hasWordSync?: boolean;
  plainLyrics?: string | null;
  syncedLyrics?: string | null;
}

interface CacheEntry {
  result: LyricsResult;
  expiresAt: number;
  cachedAt: number;
}

const CACHE_TTL_MS = 6 * 60 * 60 * 1000; // 6 h for real results
const MISS_TTL_MS = 10 * 60 * 1000; // short TTL for "nothing found"
const CACHE_KEY_PREFIX = "booming.lyrics.cache.v1";
const CACHE_MAX_ENTRIES = 50;

const NOT_FOUND: LyricsResult = {
  found: false,
  instrumental: false,
  syncedLyrics: null,
  plainLyrics: null,
  source: "lrclib",
};

const memoryCache = new Map<string, CacheEntry>();
const inflight = new Map<string, Promise<LyricsResult>>();
let localStorageReady = false;

/* ---------------------------------------------------------------- cache -- */

function cacheKey(q: LyricsQuery): string {
  const norm = (v: string | null | undefined) => (v ?? "").trim().toLowerCase();
  return `${norm(q.artist)}::${norm(q.title)}`;
}

function cacheGet(key: string): LyricsResult | null {
  const entry = memoryCache.get(key);
  if (!entry) return null;
  if (entry.expiresAt <= Date.now()) {
    memoryCache.delete(key);
    return null;
  }
  return entry.result;
}

function cachePut(key: string, result: LyricsResult): void {
  if (key === "::") return; // no usable identity
  const now = Date.now();
  const entry: CacheEntry = {
    result,
    cachedAt: now,
    expiresAt: now + (result.found ? CACHE_TTL_MS : MISS_TTL_MS),
  };
  memoryCache.set(key, entry);
  // prune expired siblings and persist the latest slice
  for (const [k, v] of [...memoryCache.entries()]) {
    if (v.expiresAt <= now) memoryCache.delete(k);
  }
  persistCache();
}

function loadCacheFromLocalStorage(): void {
  if (localStorageReady) return;
  localStorageReady = true;
  try {
    if (typeof localStorage === "undefined") return;
    const data = JSON.parse(localStorage.getItem(CACHE_KEY_PREFIX) ?? "null");
    if (!data || typeof data !== "object" || !data.entries) return;
    for (const [key, entry] of Object.entries(data.entries as Record<string, CacheEntry>)) {
      if (entry && typeof entry.expiresAt === "number" && entry.expiresAt > Date.now()) {
        memoryCache.set(key, entry);
      }
    }
  } catch {
    // corrupt cache — drop it, lookups just hit the network instead
  }
}

function persistCache(): void {
  try {
    if (typeof localStorage === "undefined") return;
    const entries: Record<string, CacheEntry> = {};
    const sorted = [...memoryCache.entries()]
      .sort((a, b) => b[1].cachedAt - a[1].cachedAt)
      .slice(0, CACHE_MAX_ENTRIES);
    for (const [key, entry] of sorted) entries[key] = entry;
    localStorage.setItem(CACHE_KEY_PREFIX, JSON.stringify({ version: 1, entries }));
  } catch {
    // quota / private mode — memory cache still works
  }
}

/** Test/debug helper: wipe every layer of the lyric cache. */
export function clearLyricsCache(): void {
  memoryCache.clear();
  inflight.clear();
  try {
    if (typeof localStorage !== "undefined") localStorage.removeItem(CACHE_KEY_PREFIX);
  } catch {
    /* ignore */
  }
  localStorageReady = false;
}

/* ---------------------------------------------------------- rate limiter -- */

const MAX_REQUESTS = 4;
const WINDOW_MS = 60_000;

/**
 * Sliding-window limiter, serialised through a promise chain so queued jobs
 * never burst: each `run()` waits for a free slot, then executes.
 */
class RateLimiter {
  private slots: number[] = [];
  private chain: Promise<void> = Promise.resolve();

  run<T>(fn: () => Promise<T>): Promise<T> {
    const task = this.chain.then(() => this.acquire()).then(fn);
    // Errors must not break the chain for later callers.
    this.chain = task.then(
      () => undefined,
      () => undefined,
    );
    return task;
  }

  private get used(): number {
    const now = Date.now();
    this.slots = this.slots.filter((t) => now - t < WINDOW_MS);
    return this.slots.length;
  }

  private wait(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms + 25));
  }

  private async acquire(): Promise<void> {
    for (;;) {
      const now = Date.now();
      if (this.used < MAX_REQUESTS) {
        this.slots.push(now);
        return;
      }
      await this.wait(WINDOW_MS - (now - this.slots[0]));
    }
  }
}

const limiter = new RateLimiter();

/* ------------------------------------------------------------- transport -- */

const REQUEST_TIMEOUT_MS = 10_000;

async function lrclibFetch(path: string, params: URLSearchParams): Promise<Response> {
  const url = `${LRCLIB_BASE}${path}?${params.toString()}`;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    // User-Agent may be clamped by some webviews; harmless to try. Being a
    // tiny, named client is what LRCLIB asks for.
    return await fetch(url, {
      headers: { "User-Agent": CLIENT_UA, Accept: "application/json" },
      signal: controller.signal,
    });
  } finally {
    clearTimeout(timer);
  }
}

function toRecord(json: unknown): LrclibRecord | null {
  if (!json || typeof json !== "object" || Array.isArray(json)) return null;
  return json as LrclibRecord;
}

/* --------------------------------------------------------------- lookups -- */

async function queryGet(params: URLSearchParams): Promise<LrclibRecord | null> {
  const res = await lrclibFetch("/get", params);
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`lrclib get failed: ${res.status}`);
  return toRecord(await res.json());
}

async function querySearch(params: URLSearchParams): Promise<LrclibRecord[]> {
  const search = new URLSearchParams();
  for (const key of ["artist_name", "track_name", "album_name"]) {
    const value = params.get(key);
    if (value) search.set(key, value);
  }
  const res = await lrclibFetch("/search", search);
  if (res.status === 404) return [];
  if (!res.ok) throw new Error(`lrclib search failed: ${res.status}`);
  const json = await res.json();
  return Array.isArray(json) ? json.filter(toRecord) : [];
}

/** Prefer synced lyrics, then plain, then nearest duration. */
function bestRecord(records: LrclibRecord[], durationSecs: number | null): LrclibRecord | null {
  let best: LrclibRecord | null = null;
  let bestScore = Number.POSITIVE_INFINITY;
  for (const record of records) {
    const hasLyrics = !!record.syncedLyrics || !!record.plainLyrics || !!record.instrumental;
    if (!hasLyrics) continue;
    const quality = record.syncedLyrics ? 0 : record.plainLyrics ? 1 : 2;
    const durationGap =
      durationSecs && typeof record.duration === "number"
        ? Math.abs(record.duration - durationSecs)
        : 3600;
    const score = quality * 10_000 + durationGap;
    if (score < bestScore) {
      bestScore = score;
      best = record;
    }
  }
  return best;
}

function fromRecord(record: LrclibRecord | null): LyricsResult {
  if (!record) return NOT_FOUND;
  const synced = record.syncedLyrics || null;
  const plain = record.plainLyrics || null;
  const instrumental = !!record.instrumental && !synced && !plain;
  const found = !!synced || !!plain || instrumental;
  if (!found) return NOT_FOUND;
  return { found: true, instrumental, syncedLyrics: synced, plainLyrics: plain, source: "lrclib" };
}

async function queryProvider(q: LyricsQuery): Promise<LyricsResult> {
  const params = new URLSearchParams();
  params.set("artist_name", String(q.artist));
  params.set("track_name", String(q.title));
  if (q.album) params.set("album_name", q.album);
  if (q.durationSecs && isFinite(q.durationSecs)) {
    params.set("duration", String(Math.round(q.durationSecs)));
  }

  let record: LrclibRecord | null = null;
  try {
    record = await queryGet(params);
  } catch (err) {
    console.warn("[lyrics] lrclib /get failed", err);
    return NOT_FOUND;
  }
  if (!record) {
    try {
      record = bestRecord(await querySearch(params), q.durationSecs ?? null);
    } catch (err) {
      console.warn("[lyrics] lrclib /search failed", err);
      return NOT_FOUND;
    }
  }
  return fromRecord(record);
}

/**
 * Fetch lyrics for a song. Preference inside this function is LRCLIB's
 * synced-over-plain result; caller-side preference (sidecar .lrc → provider)
 * lives in the UI layer.
 */
export async function fetchLyrics(q: LyricsQuery): Promise<LyricsResult> {
  loadCacheFromLocalStorage();
  if (!q.artist?.trim() || !q.title?.trim()) return NOT_FOUND;

  const key = cacheKey(q);
  const cached = cacheGet(key);
  if (cached) return cached;

  const job = inflight.get(key);
  if (job) return job; // drop-duplicates: coalesce into the same request

  const request = limiter
    .run(() => queryProvider(q))
    .catch(() => NOT_FOUND)
    .then((result) => {
      // Only memorable results occupy the long-lived cache layers.
      if (result.found) cachePut(key, result);
      return result;
    })
    .finally(() => {
      inflight.delete(key);
    });
  inflight.set(key, request);
  return request;
}

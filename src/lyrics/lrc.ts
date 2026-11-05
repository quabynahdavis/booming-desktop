/**
 * LRC parser (TS mirror of `src-tauri/src/lyrics/lrc.rs`).
 *
 * Same grammar and semantics, byte-for-byte behavior:
 * - line stamps `mm:ss.xx` / `mm:ss.xxx` / `mm:ss:xxx` / `mm:ss`,
 *   multiple stamps per line, `key:value` metadata tags,
 * - `[offset:+ms]` applied to every stamp (clamped at 0),
 * - enhanced/karaoke inline tags `word<mm:ss.xx>word` → word timings,
 *   last word of a line stays open (`endMs: null`),
 * - bare text lines kept as untimed (`timeMs: 0`) entries in file order,
 * - plain (no-timestamp) content degrades to line-level rendering.
 */

export interface WordTiming {
  startMs: number;
  /** End of the word = next inline tag; `null` when open-ended. */
  endMs: number | null;
  word: string;
}

export interface LyricLine {
  timeMs: number;
  text: string;
  /** Word-level karaoke timings; `null` for plain lines. */
  words: WordTiming[] | null;
}

export interface ParsedLyrics {
  metadata: Record<string, string>;
  lines: LyricLine[];
}

/** Parse a stamp body (inside `[..]` / `<..>`): mm:ss(.|:)frac? in ms. */
export function parseLrcTimestamp(inner: string): number | null {
  const s = inner.trim();
  if (!s.length) return null;

  const digits0 = matchDigits(s, 0);
  if (!digits0) return null;
  const mins = parseInt(s.slice(0, digits0), 10);
  let i = digits0;
  if (s[i] !== ":") return null;
  i += 1;
  if (i >= s.length) return null;

  const secsStart = i;
  const secsDigits = matchDigits(s, i);
  if (!secsDigits) return null;
  const secs = parseInt(s.slice(secsStart, secsDigits), 10);
  i = secsDigits;

  // Optional fraction after '.' or ':' — `[00:12.45]` / `[00:12:456]`.
  let fracMs = 0;
  if ((s[i] === "." || s[i] === ":") && isDigit(s[i + 1] ?? "")) {
    const fracStart = i + 1;
    let fracEnd = fracStart;
    while (fracEnd < s.length && isDigit(s[fracEnd])) fracEnd += 1;
    // more than 3 fractional digits are truncated (rust side mirrors this)
    const frac = s.slice(fracStart, fracStart + 3);
    i = fracEnd;
    fracMs = fractionToMs(frac);
  }

  // Reject `[00:12 junk]` — only whitespace may trail the stamp.
  if (s.slice(i).trim().length) return null;

  if (mins > 999 || secs > 599) return null;
  const total = (mins * 60 + secs) * 1000 + fracMs;
  return total <= 0xffffffff ? total : null;
}

function isDigit(c: string): boolean {
  return c >= "0" && c <= "9";
}

function matchDigits(s: string, from: number): number {
  let end = from;
  while (end < s.length && isDigit(s[end])) end += 1;
  return end === from ? -1 : end;
}

function fractionToMs(frac: string): number {
  const num = parseInt(frac, 10) || 0;
  if (frac.length <= 0) return 0;
  if (frac.length === 1) return num * 100;
  if (frac.length === 2) return num * 10;
  return num;
}

interface Segment {
  kind: "text" | "tag";
  value: string | number;
}

/** `<…>` tags and text between them; `<` that isn't a full stamp stays text. */
function scanInline(content: string): Segment[] {
  const segments: Segment[] = [];
  let textStart = 0;
  let i = 0;
  while (i < content.length) {
    if (content[i] !== "<") {
      i += 1;
      continue;
    }
    const close = content.indexOf(">", i + 1);
    if (close === -1) break;
    const stamp = parseLrcTimestamp(content.slice(i + 1, close));
    if (stamp === null) {
      i = close + 1;
      continue;
    }
    if (textStart < i) segments.push({ kind: "text", value: content.slice(textStart, i) });
    segments.push({ kind: "tag", value: stamp });
    textStart = close + 1;
    i = close + 1;
  }
  if (textStart < content.length) segments.push({ kind: "text", value: content.slice(textStart) });
  return segments;
}

/** Strip inline tags, collect word timings when the line has any. */
function splitInline(content: string, lineTimeMs: number): { text: string; words: WordTiming[] | null } {
  const segments = scanInline(content);
  if (!segments.some((s) => s.kind === "tag")) return { text: content, words: null };

  const words: WordTiming[] = [];
  let pendingStart: number | null = null;
  let plain = "";
  for (const seg of segments) {
    if (seg.kind === "tag") {
      const t = seg.value as number;
      const last = words[words.length - 1];
      if (last && last.endMs === null) last.endMs = t;
      pendingStart = t;
    } else {
      const chunk = seg.value as string;
      if (!chunk.length) continue;
      // Join chunks so removing tags cannot glue words together:
      // `All<00:12.5>of` still reads `All of`.
      if (plain.length && !/\s$/.test(plain) && !/^\s/.test(chunk)) plain += " ";
      plain += chunk; // keep raw spacing for the plain text
      const trimmed = chunk.trim();
      if (!trimmed.length) continue;
      words.push({ startMs: pendingStart ?? lineTimeMs, endMs: null, word: trimmed });
      pendingStart = null;
    }
  }
  return { text: plain, words: words.length ? words : null };
}

/** Parse a whole LRC document. Never throws; unknown parts degrade gracefully. */
export function parseLrc(text: string): ParsedLyrics {
  const cleaned = text.replace(/^\uFEFF/, "");
  const metadata: Record<string, string> = {};
  let offsetMs = 0;

  interface RawLine {
    times: number[];
    content: string;
  }
  const rawLines: RawLine[] = [];

  for (const raw of cleaned.split(/\r\n|\r|\n/)) {
    let rest = raw.trim();
    const times: number[] = [];

    // Bracket headers: timestamps and `key:value` tags, in order, until the
    // first bracket that is neither — from there it is literal lyric text.
    while (rest.startsWith("[") && rest.includes("]")) {
      const close = rest.indexOf("]");
      const inner = rest.slice(1, close);
      const stamp = parseLrcTimestamp(inner);
      if (stamp !== null) {
        times.push(stamp);
        rest = rest.slice(close + 1);
        continue;
      }
      const colon = inner.indexOf(":");
      if (colon <= 0) break; // no key → literal text from here on
      const key = inner.slice(0, colon).trim().toLowerCase();
      const value = inner.slice(colon + 1).trim();
      if (key === "offset") {
        const digits = value.replace(/^[+-]/, "");
        offsetMs = /^\d+$/.test(digits) ? Number(value) : 0;
        metadata["offset"] = `${offsetMs < 0 ? "-" : "+"}${Math.abs(offsetMs)}`;
      } else {
        metadata[key] = value;
      }
      rest = rest.slice(close + 1);
    }

    const content = rest.trim();
    if (!times.length && !content.length) continue; // blank or metadata-only
    rawLines.push({ times, content });
  }

  const lines: LyricLine[] = [];
  for (const { times, content } of rawLines) {
    if (!times.length) {
      const { text: plain, words } = splitInline(content, 0);
      lines.push({ timeMs: 0, text: plain, words });
    } else {
      const clamped = times.map((t) => Math.min(0xffffffff, Math.max(0, t + offsetMs)));
      const { text: plain, words } = splitInline(content, clamped[0]);
      for (const stamp of clamped) {
        lines.push({ timeMs: stamp, text: plain, words });
      }
    }
  }
  lines.sort((a, b) => a.timeMs - b.timeMs); // stable enough for equal stamps

  return { metadata, lines };
}

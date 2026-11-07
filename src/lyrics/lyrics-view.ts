/**
 * Synced lyrics view (P7) — frontend.
 *
 * Renders `{ metadata, lines }` from `src/lyrics/lrc.ts`:
 * - synced mode: active line highlighted in the accent color, lines before it
 *   dimmed, upcoming ones muted, auto-scrolled to the current line. Tapping a
 *   line seeks to `line.timeMs` (the caller decides how to seek).
 * - karaoke mode: when lines carry word timings (enhanced LRC), words inside
 *   the active line paint progressively as their timestamps pass. Simple
 *   per-word fill, one style write per word/tick; scale/animation polish is
 *   deferred.
 * - plain mode: static text block, no highlight, no per-line seeking.
 */

import type { LyricLine, ParsedLyrics } from "./lrc.js";

export interface LyricsView {
  el: HTMLElement;
  /** Header line showing the loaded track. */
  setHeader(title: string | null, artist: string | null): void;
  /**
   * (Re)render with fresh lyrics. `parsed` wins when it carries synced
   * lines; otherwise `rawText` is shown as a plain block; otherwise empty.
   */
  setLyrics(parsed: ParsedLyrics | null, rawText: string | null, source: string | null): void;
  /** Show a transient helper message ("Loading…", "Instrumental track"…). */
  setStatus(message: string | null): void;
  /** Feed playback position (seconds) — call on every poll tick. */
  update(positionSecs: number): void;
}

interface Mode {
  kind: "synced" | "plain" | "empty";
  lines: LyricLine[];
}

export function createLyricsView(onSeekMs?: (timeMs: number) => void): LyricsView {
  const el = document.createElement("div");
  el.className = "lyrics-pane";

  // Header
  const header = document.createElement("div");
  header.className = "lyrics-header";
  const titleEl = document.createElement("div");
  titleEl.className = "lyrics-title";
  const sourceLine = document.createElement("div");
  sourceLine.className = "lyrics-source";
  header.append(titleEl, sourceLine);

  // Status chip ("Loading…", "Instrumental track", errors)
  const statusEl = document.createElement("div");
  statusEl.className = "lyrics-status";

  // Scrollable lyric body (transparent surface text, app-like)
  const body = document.createElement("div");
  body.className = "lyrics-body";
  el.append(header, body, statusEl);

  let mode: Mode = { kind: "empty", lines: [] };
  let lineEls: HTMLElement[] = [];
  let activeIndex = -1;
  let plainRaw: string | null = null;
  let plainSource: string | null = null;
  let posMs = 0;

  function setHeader(title: string | null, artist: string | null): void {
    titleEl.textContent = title ?? "";
    sourceLine.textContent = artist ?? "";
  }

  function setStatus(message: string | null): void {
    statusEl.textContent = message ?? "";
  }

  function setLyrics(parsed: ParsedLyrics | null, rawText: string | null, source: string | null): void {
    const hasSynced = !!parsed?.lines.some((l) => l.timeMs > 0);
    if (hasSynced) {
      mode = { kind: "synced", lines: parsed!.lines };
    } else if (rawText?.trim()) {
      mode = { kind: "plain", lines: [] };
      plainRaw = rawText;
    } else {
      mode = { kind: "empty", lines: [] };
      plainRaw = null;
    }
    plainSource = source;
    renderSourceLabel();
    renderAll();
  }

  function renderSourceLabel(): void {
    const decorated = plainSource
      ? `${mode.kind === "synced" ? "Synced" : "Plain"} · ${plainSource}`
      : "";
    sourceLine.textContent = titleEl.textContent ? (decorated ? decorated : "") : decorated;
  }

  function renderAll(): void {
    body.textContent = "";
    lineEls = [];
    activeIndex = -1;
    if (mode.kind === "synced") {
      mode.lines.forEach((line) => {
        const row = document.createElement("div");
        row.className = "lyrics-line upcoming";
        row.setAttribute("data-time-ms", String(line.timeMs));
        if (line.words?.length) {
          line.words.forEach((word, wi) => {
            // Two-layer per-word fill (works without background-clip:text):
            // base text stays muted; an absolutely-positioned accent overlay
            // of growing width reveals the word as its timings pass.
            const wrap = document.createElement("span");
            wrap.className = "lyrics-word";
            const base = document.createElement("span");
            base.textContent = word.word;
            const fill = document.createElement("span");
            fill.className = "lyrics-word-fill";
            fill.textContent = word.word;
            wrap.append(base, fill);
            row.appendChild(wrap);
            if (wi < line.words!.length - 1) row.appendChild(document.createTextNode(" "));
          });
        } else {
          row.textContent = line.text || "♪";
        }
        row.addEventListener("click", () => {
          if (onSeekMs) onSeekMs(line.timeMs);
        });
        body.appendChild(row);
        lineEls.push(row);
      });
    } else if (mode.kind === "plain" && plainRaw) {
      const block = document.createElement("pre");
      block.className = "lyrics-plain";
      block.textContent = plainRaw;
      body.appendChild(block);
      lineEls = [block];
    }
  }

  /** One style sweep for the active-line change; cheap at ≤ a few hundred rows. */
  function restyleRows(): void {
    for (let index = 0; index < lineEls.length; index++) {
      const row = lineEls[index];
      if (index === activeIndex) {
        row.className = "lyrics-line active"; // highlighted + karaoke fill runs in accents
      } else if (index < activeIndex) {
        row.className = "lyrics-line past"; // lines before the current one are dimmed
      } else {
        row.className = "lyrics-line upcoming"; // upcoming lines stay muted
      }
    }
  }

  /** Karaoke fill: the accent overlay width grows from word.start → word.end. */
  function paintWords(line: LyricLine | null): void {
    if (!line?.words?.length) return;
    const wraps = lineEls[activeIndex]?.querySelectorAll<HTMLElement>(".lyrics-word");
    if (!wraps) return;
    for (let wi = 0; wi < wraps.length && wi < line.words.length; wi++) {
      const word = line.words[wi];
      let fillPct = 0;
      if (word.endMs !== null && posMs >= word.endMs) {
        fillPct = 100;
      } else if (posMs >= word.startMs && word.endMs !== null && word.endMs > word.startMs) {
        fillPct = Math.min(100, ((posMs - word.startMs) / (word.endMs - word.startMs)) * 100);
      }
      const fillSpan = wraps[wi].firstElementChild as HTMLElement | null;
      if (fillSpan) fillSpan.style.width = `${fillPct}%`;
    }
  }

  /** Last line whose timestamp is ≤ playhead. */
  function findActiveIndex(posMilliseconds: number): number {
    const lines = mode.lines;
    let lo = 0;
    let hi = lines.length - 1;
    let found = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (lines[mid].timeMs <= posMilliseconds) {
        found = mid;
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    return found;
  }

  function scrollToActive(instant: boolean): void {
    const target = lineEls[activeIndex];
    if (!target) return;
    const top = target.offsetTop - body.clientHeight * 0.38;
    if (instant) {
      body.style.scrollBehavior = "auto";
      body.scrollTo({ top, behavior: "auto" });
      body.style.scrollBehavior = "smooth";
    } else {
      body.scrollTo({ top, behavior: "smooth" });
    }
  }

  function update(positionSecs: number): void {
    if (mode.kind !== "synced" || lineEls.length === 0) return;
    posMs = Math.max(0, positionSecs) * 1000;
    const index = findActiveIndex(posMs);
    if (index !== activeIndex) {
      const bigJump = Math.abs(index - activeIndex) > 8;
      activeIndex = index;
      restyleRows();
      if (index >= 0) scrollToActive(bigJump);
    }
    paintWords(mode.lines[activeIndex] ?? null);
  }

  return { el, setHeader, setLyrics, setStatus, update };
}

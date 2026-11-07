/**
 * Dynamic color — generates a Material 3 accent scheme from the current
 * track's cover art, mirroring what the Booming Music Android app does
 * ("colors from album cover").
 *
 * Pipeline per data URL:
 *   1. decode → downscale canvas (64×64) → RGB pixels
 *   2. QuantizerCelebi.quantize → dominant color clusters (color → count)
 *   3. Score.score over the cluster map → source color (notable hue, avoids
 *      low-chroma / mid-tone candidates, as in the Android dynamic color)
 *   4. SchemeContent (source color lands in primaryContainer) for both the
 *      dark and the light scheme, standard contrast level
 *   5. publish as `--m3-dynamic-*` custom properties for each scheme (see
 *      tokens.css), so aliases like `--md-primary` and the tinted chrome
 *      resolve per active theme.
 *
 * When no cover exists (or decoding/quantizing fails) the overrides are
 * removed and every `--m3-dynamic-*` token falls back to the static brand
 * scheme from tokens.css. Only this file and tokens.css are permitted to
 * contain raw color literals (theming CI gate).
 */

import {
  Hct,
  QuantizerCelebi,
  Score,
  SchemeContent,
  hexFromArgb,
} from "@material/material-color-utilities";

const STYLE_ID = "m3-dynamic-vars";
const DEDUP_PREFIX = 160;
const SIDE = 64;
const QUANTIZER_COLORS = 16;
const MIN_PIXELS = 16;
/** Chrome tint strength: how far chrome surfaces blend toward the accent. */
const CHROME_TINT = 0.12;
/** Chrome surface bases (surface-container-low) matching tokens.css. */
const CHROME_BASE_DARK = 0xff1a1b21;
const CHROME_BASE_LIGHT = 0xfff4f3fa;

/** Opaque luminance estimate (Rec.601) of an argb int, in 0..255. */
function luminance(argb: number): number {
  const r = (argb >> 16) & 0xff;
  const g = (argb >> 8) & 0xff;
  const b = argb & 0xff;
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

/** Linear blend of `tint` toward `base` by `amount` (0..1), opaque result. */
function blendToward(tintArgb: number, baseArgb: number, amount: number): number {
  const mix = (t: number, b: number) => Math.round(b + (t - b) * amount);
  const r = mix(tintArgb >> 16 & 0xff, baseArgb >> 16 & 0xff);
  const g = mix(tintArgb >> 8 & 0xff, baseArgb >> 8 & 0xff);
  const b = mix(tintArgb & 0xff, baseArgb & 0xff);
  return 0xff000000 | (r << 16) | (g << 8) | b;
}

/** Highest-count cluster that is neither near-black nor near-white. */
function dominantFallback(clusters: Map<number, number>): number | null {
  let best: number | null = null;
  let bestCount = -1;
  for (const [argb, count] of clusters) {
    const lum = luminance(argb);
    if (lum > 40 && lum < 215 && count > bestCount) {
      bestCount = count;
      best = argb;
    }
  }
  return best;
}

async function coverSourceColor(dataUrl: string): Promise<number | null> {
  const img = new Image();
  img.decoding = "async";
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error("cover decode failed"));
    img.src = dataUrl;
  });
  const canvas = document.createElement("canvas");
  canvas.width = SIDE;
  canvas.height = SIDE;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, SIDE, SIDE);
  const { data } = ctx.getImageData(0, 0, SIDE, SIDE);
  const pixels: number[] = [];
  for (let i = 0; i + 3 < data.length; i += 4) {
    if (data[i + 3] < 96) continue; // skip transparent pixels
    pixels.push((data[i] << 16) | (data[i + 1] << 8) | data[i + 2]);
  }
  if (pixels.length < MIN_PIXELS) return null;
  const clusters = QuantizerCelebi.quantize(pixels, QUANTIZER_COLORS);
  if (clusters.size === 0) return null;
  // Score expects the color→population map directly; `Score.score` always
  // returns at least one color, using `fallbackColorARGB` when nothing is
  // suitable, so seed it with the real dominant cluster, not Google Blue.
  const fallback = dominantFallback(clusters);
  if (fallback === null) return null;
  const ranked = Score.score(clusters, { desired: 4, fallbackColorARGB: fallback, filter: true });
  return ranked.length > 0 ? ranked[0] : fallback;
}

function rulesForScheme(scheme: SchemeContent, chromeBase: number): string {
  const roles: Array<[string, number]> = [
    ["--m3-dynamic-primary", scheme.primary],
    ["--m3-dynamic-on-primary", scheme.onPrimary],
    ["--m3-dynamic-primary-container", scheme.primaryContainer],
    ["--m3-dynamic-on-primary-container", scheme.onPrimaryContainer],
    ["--m3-dynamic-secondary-container", scheme.secondaryContainer],
    ["--m3-dynamic-on-secondary-container", scheme.onSecondaryContainer],
    ["--m3-dynamic-primary-fixed", scheme.primaryFixed],
    ["--m3-dynamic-surface-tint", scheme.primary],
  ];
  const lines = roles.map(([name, argb]) => `  ${name}: ${hexFromArgb(argb)};`);
  lines.push(
    `  --m3-dynamic-chrome-surface: ${hexFromArgb(blendToward(scheme.primary, chromeBase, CHROME_TINT))};`,
  );
  return lines.join("\n");
}

function ensureStyleElement(): HTMLStyleElement {
  let el = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement("style");
    el.id = STYLE_ID;
    document.head.appendChild(el);
  }
  return el;
}

export function clearDynamicCover(): void {
  const el = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
  if (el) el.textContent = "";
}

let lastCoverKey: string | null = null;
let workToken = 0;

/**
 * Feed the current track's cover (data URL) — or `null` when there is no
 * artwork — and keep the dynamic accent scheme in sync. Safe to call on
 * every poll tick; work is deduplicated by cover key.
 */
export function applyDynamicCover(dataUrl: string | null): void {
  const key = dataUrl ? dataUrl.slice(0, DEDUP_PREFIX) + ":" + String(dataUrl.length) : null;
  if (key === lastCoverKey) return;
  lastCoverKey = key;
  const token = ++workToken;
  if (!dataUrl) {
    clearDynamicCover();
    return;
  }
  void (async () => {
    let source: number | null = null;
    try {
      source = await coverSourceColor(dataUrl);
    } catch {
      source = null;
    }
    if (token !== workToken) return; // a newer cover superseded this one
    if (source === null) {
      clearDynamicCover();
      return;
    }
    try {
      const dark = new SchemeContent(Hct.fromInt(source), true, 0);
      const light = new SchemeContent(Hct.fromInt(source), false, 0);
      const style = ensureStyleElement();
      if (style) {
        style.textContent =
          `:root, :root[data-theme="dark"] {\n${rulesForScheme(dark, CHROME_BASE_DARK)}\n}\n` +
          `:root[data-theme="light"] {\n${rulesForScheme(light, CHROME_BASE_LIGHT)}\n}`;
      }
    } catch {
      clearDynamicCover();
    }
  })();
}

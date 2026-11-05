/**
 * Default cover used when a song has no embedded artwork.
 * SVG lives beside the other static assets (resources/) so the only raw
 * color literals in `src/` stay inside the theme token stylesheet.
 */
import fallbackCoverUrl from "../../resources/fallback-cover.svg";

export const FALLBACK_COVER = fallbackCoverUrl;

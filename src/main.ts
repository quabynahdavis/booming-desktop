// Booming Desktop — engine probe smoke test (M0).
//
// `invoke` only reaches the Rust side when the page runs inside the Tauri
// webview; plain-browser dev (vite alone) fails gracefully below.
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";
import '@material/web/button/filled-tonal-button.js';
import { styles as typescaleStyles } from '@material/web/typography/md-typescale-styles.js';

if (typeof document !== 'undefined' && typescaleStyles?.styleSheet) {
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, typescaleStyles.styleSheet];
}

interface EngineStatus {
  engine: "mpv" | "null";
  loaded: boolean;
  version: string | null;
  error: string | null;
}

const button = document.querySelector<HTMLElement>("#check-engine");
const resultPanel = document.querySelector<HTMLElement>("#engine-result");
const summary = document.querySelector<HTMLParagraphElement>("#engine-summary");
const output = document.querySelector<HTMLPreElement>("#engine-json");

if (!button || !resultPanel || !summary || !output) {
  throw new Error("engine probe UI is malformed (missing DOM anchors)");
}

button.addEventListener("click", async () => {
  button.setAttribute("disabled", "");
  summary.className = "";
  summary.textContent = "Probing playback engine…";
  resultPanel.hidden = false;
  output.textContent = "";

  try {
    const status = await invoke<EngineStatus>("engine_probe");
    output.textContent = JSON.stringify(status, null, 2);

    if (status.engine === "mpv" && status.loaded) {
      summary.className = "ok";
      summary.textContent = `mpv backend ready: ${status.version ?? "unknown version"}`;
    } else if (status.engine === "null") {
      summary.className = "warn";
      summary.textContent =
        "null fallback active — libmpv is not available on this machine";
    } else {
      summary.className = "warn";
      summary.textContent = "mpv backend degraded — details below";
    }
  } catch (error) {
    summary.className = "err";
    summary.textContent = "engine_probe failed — running outside Tauri?";
    output.textContent = JSON.stringify({ error: String(error) }, null, 2);
  } finally {
    button.removeAttribute("disabled");
  }
});

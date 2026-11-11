import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { t } from "../i18n/index.js";

const BAND_CENTERS_HZ = [
  25, 40, 63, 100, 160, 250, 400, 630, 1000, 1600, 2500, 4000, 6300, 10000, 16000,
];

export function equalizerScreen(): HTMLElement {
  const container = document.createElement("div");
  container.className = "screen-container";

  const header = document.createElement("div");
  header.className = "screen-header";
  const h1 = document.createElement("h1");
  h1.textContent = t('screen.equalizer');
  header.appendChild(h1);
  container.appendChild(header);

  // Enable switch
  const enableWrap = document.createElement("div");
  enableWrap.style.marginBottom = "16px";
  const enableSwitch = document.createElement("md-switch");
  enableSwitch.setAttribute("id", "eq-enable");
  const enableLabel = document.createElement("label");
  enableLabel.htmlFor = "eq-enable";
  enableLabel.textContent = t('equalizer.EnableEqualizer');
  enableLabel.style.marginLeft = "12px";
  enableLabel.style.cursor = "pointer";
  enableWrap.append(enableSwitch, enableLabel);
  container.appendChild(enableWrap);

  // Sliders grid
  const slidersWrap = document.createElement("div");
  slidersWrap.style.display = "grid";
  slidersWrap.style.gap = "16px";
  slidersWrap.style.gridTemplateColumns = "repeat(auto-fill, minmax(200px, 1fr))";
  container.appendChild(slidersWrap);

  const sliderElements: HTMLElement[] = [];

  for (let i = 0; i < 15; i++) {
    const freq = BAND_CENTERS_HZ[i];
    const labelText = freq < 1000 ? `${freq} Hz` : `${freq / 1000}k Hz`;

    const col = document.createElement("div");
    col.style.display = "flex";
    col.style.flexDirection = "column";
    col.style.alignItems = "center";
    col.style.gap = "8px";

    const freqLabel = document.createElement("div");
    freqLabel.style.fontSize = "12px";
    freqLabel.style.color = "var(--md-sys-color-on-surface-variant)";
    freqLabel.textContent = labelText;
    col.appendChild(freqLabel);

    const slider = document.createElement("md-slider");
    slider.setAttribute("min", "-15");
    slider.setAttribute("max", "15");
    slider.setAttribute("step", "0.1");
    slider.setAttribute("value", "0");
    slider.setAttribute("data-band-index", String(i));
    (slider as any).label = `${freqText(freq)} dB`;
    slider.style.width = "100%";
    slider.style.maxWidth = "220px";

    const valueLabel = document.createElement("div");
    valueLabel.style.fontSize = "13px";
    valueLabel.style.fontWeight = "500";
    valueLabel.style.fontVariantNumeric = "tabular-nums";
    valueLabel.textContent = "0.0 dB";

    col.appendChild(slider);
    col.appendChild(valueLabel);
    slidersWrap.appendChild(col);
    sliderElements.push(slider);
  }

  // Presets row
  const presetsWrap = document.createElement("div");
  presetsWrap.style.display = "flex";
  presetsWrap.style.justifyContent = "center";
  presetsWrap.style.gap = "12px";
  presetsWrap.style.marginTop = "24px";
  container.appendChild(presetsWrap);

  const presets = [t('equalizer.Flat'), t('equalizer.BassBoost'), t('equalizer.TrebleBoost')];
  for (const name of presets) {
    const btn = document.createElement("md-text-button");
    btn.textContent = name;
    btn.addEventListener("click", async () => {
      try {
        await invoke("load_eq_preset", { name });
        await refreshState();
      } catch {}
    });
    presetsWrap.appendChild(btn);
  }

  // Reset button
  const resetBtn = document.createElement("md-text-button");
  resetBtn.textContent = t('equalizer.Reset');
  resetBtn.style.marginLeft = "auto";
  resetBtn.addEventListener("click", async () => {
    try {
      for (let i = 0; i < 15; i++) {
        await invoke("set_eq_bands", { band_index: i, value: 0 });
      }
      await refreshState();
    } catch {}
  });
  presetsWrap.appendChild(resetBtn);

  // AutoEq import
  const importWrap = document.createElement("div");
  importWrap.style.marginTop = "24px";
  importWrap.style.paddingTop = "16px";
  importWrap.style.borderTop = "1px solid var(--md-sys-color-outline-variant)";
  const importBtn = document.createElement("md-filled-tonal-button");
  importBtn.textContent = t('equalizer.ImportAutoEq');
  importBtn.addEventListener("click", async () => {
    try {
      const filePath = await open({
        filters: [{ name: "AutoEq JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (filePath) {
        await invoke("import_autoeq", { path: filePath });
        await refreshState();
      }
    } catch {}
  });
  importWrap.appendChild(importBtn);
  container.appendChild(importWrap);

  // State refresh: poll get_eq_state
  async function refreshState() {
    try {
      const state = await invoke<{ enabled: boolean; bands: number[] }>("get_eq_state");
      (enableSwitch as any).checked = state.enabled;
      for (let i = 0; i < 15; i++) {
        if (sliderElements[i] && state.bands[i] !== undefined) {
          const v = state.bands[i];
          (sliderElements[i] as any).value = v;
          const valueLabel = sliderElements[i].parentElement?.querySelector("div:last-child");
          if (valueLabel) valueLabel.textContent = `${v >= 0 ? "+" : ""}${v.toFixed(1)} dB`;
        }
      }
    } catch {}
  }

  // Event listeners
  enableSwitch.addEventListener("change", async () => {
    try {
      await invoke("set_eq_enabled", { enabled: (enableSwitch as any).checked });
      await refreshState();
    } catch {}
  });

  for (const slider of sliderElements) {
    slider.addEventListener("input", () => {
      const value = Number((slider as any).value);
      const valueLabel = slider.parentElement?.querySelector("div:last-child");
      if (valueLabel) valueLabel.textContent = `${value >= 0 ? "+" : ""}${value.toFixed(1)} dB`;
    });
    slider.addEventListener("change", async () => {
      const bandIndex = Number(slider.getAttribute("data-band-index"));
      const value = Number((slider as any).value);
      try {
        await invoke("set_eq_band", { band_index: bandIndex, value });
      } catch {}
    });
  }

  // Initial refresh
  refreshState();

  return container;
}

function freqText(hz: number): string {
  return hz < 1000 ? String(hz) : `${hz / 1000}k`;
}
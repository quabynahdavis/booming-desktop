import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";
import { t } from "../i18n/index.js";

export function foldersScreen() {
  const gridEl = grid();
  const container = screenContainer(t('screen.folders'), gridEl);
  invoke('list_folders', { root: '' })
    .then((folders: any) => {
      gridEl.innerHTML = '';
      for (const f of folders as any[]) {
        const c = card('📁', f.name || t('screen.Unknown'), `${f.song_count ?? 0} ${t('screen.songs')}`);
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = `<div>${t('screen.NoFolders')}</div>`;
    });
  return container;
}

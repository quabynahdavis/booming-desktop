import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";

export function foldersScreen() {
  const gridEl = grid();
  const container = screenContainer('Folders', gridEl);
  invoke('list_folders', { root: '' })
    .then((folders: any) => {
      gridEl.innerHTML = '';
      for (const f of folders as any[]) {
        const c = card('📁', f.name || 'Unknown', `${f.song_count ?? 0} songs`);
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = '<div>No folders</div>';
    });
  return container;
}

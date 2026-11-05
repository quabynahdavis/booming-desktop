import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";
import { Router } from "../router.js";

export function albumsScreen() {
  const gridEl = grid();
  const container = screenContainer('Albums', gridEl);
  invoke('list_albums', { sort: 'title', page: 0, pageSize: 50 })
    .then((albums: any) => {
      gridEl.innerHTML = '';
      for (const a of albums as any[]) {
        const c = card('🎵', a.title || 'Unknown', `${a.song_count ?? 0} songs`);
        c.addEventListener('click', () => {
          Router.navigate('album-detail', { id: a.id });
        });
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = '<div>No albums</div>';
    });
  return container;
}

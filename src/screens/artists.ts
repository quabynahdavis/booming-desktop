import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";
import { Router } from "../router.js";

export function artistsScreen() {
  const gridEl = grid();
  const container = screenContainer('Artists', gridEl);
  invoke('list_artists', { sort: 'artist', page: 0, pageSize: 50 })
    .then((artists: any) => {
      gridEl.innerHTML = '';
      for (const a of artists as any[]) {
        const c = card('👤', a.name || 'Unknown', `${a.song_count ?? 0} songs`);
        c.addEventListener('click', () => {
          Router.navigate('artist-detail', { name: a.name });
        });
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = '<div>No artists</div>';
    });
  return container;
}

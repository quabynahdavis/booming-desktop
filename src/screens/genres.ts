import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";

export function genresScreen() {
  const gridEl = grid();
  const container = screenContainer('Genres', gridEl);
  invoke('list_genres', { sort: 'title', page: 0, pageSize: 50 })
    .then((genres: any) => {
      gridEl.innerHTML = '';
      for (const g of genres as any[]) {
        const c = card('🎶', g.name || 'Unknown', `${g.song_count ?? 0} songs`);
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = '<div>No genres</div>';
    });
  return container;
}

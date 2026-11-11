import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";
import { t } from "../i18n/index.js";

export function genresScreen() {
  const gridEl = grid();
  const container = screenContainer(t('screen.genres'), gridEl);
  invoke('list_genres', { sort: 'title', page: 0, pageSize: 50 })
    .then((genres: any) => {
      gridEl.innerHTML = '';
      for (const g of genres as any[]) {
        const c = card('🎶', g.name || t('screen.Unknown'), `${g.song_count ?? 0} ${t('screen.songs')}`);
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = `<div>${t('screen.NoGenres')}</div>`;
    });
  return container;
}

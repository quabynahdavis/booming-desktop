import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";
import { Router } from "../router.js";
import { t } from "../i18n/index.js";

export function artistsScreen() {
  const gridEl = grid();
  const container = screenContainer(t('screen.artists'), gridEl);
  invoke('list_artists', { sort: 'artist', page: 0, pageSize: 50 })
    .then((artists: any) => {
      gridEl.innerHTML = '';
      for (const a of artists as any[]) {
        const c = card('👤', a.name || t('screen.Unknown'), `${a.song_count ?? 0} ${t('screen.songs')}`);
        c.addEventListener('click', () => {
          Router.navigate('artist-detail', { name: a.name });
        });
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = `<div>${t('screen.NoArtists')}</div>`;
    });
  return container;
}

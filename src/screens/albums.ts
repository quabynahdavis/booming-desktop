import { invoke } from "@tauri-apps/api/core";
import { screenContainer, grid, card } from "./common.js";
import { Router } from "../router.js";
import { t } from "../i18n/index.js";

export function albumsScreen() {
  const gridEl = grid();
  const container = screenContainer(t('screen.albums'), gridEl);
  invoke('list_albums', { sort: 'title', page: 0, pageSize: 50 })
    .then((albums: any) => {
      gridEl.innerHTML = '';
      for (const a of albums as any[]) {
        const c = card('🎵', a.title || t('screen.Unknown'), `${a.song_count ?? 0} ${t('screen.songs')}`);
        c.addEventListener('click', () => {
          Router.navigate('album-detail', { id: a.id });
        });
        gridEl.appendChild(c);
      }
    })
    .catch(() => {
      gridEl.innerHTML = `<div>${t('screen.NoAlbums')}</div>`;
    });
  return container;
}

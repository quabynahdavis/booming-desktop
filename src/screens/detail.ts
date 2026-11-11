import { invoke } from "@tauri-apps/api/core";
import { screenContainer } from "./common.js";
import { RouteParams } from "../router.js";
import { t } from "../i18n/index.js";

export function detailScreen(params: RouteParams) {
  const container = screenContainer(params.name || t('screen.detail'), document.createElement('div'));
  const list = document.createElement('div');
  list.className = 'song-list-detail';

  const loadSongs = async () => {
    try {
      let songs: any[] = [];
      if (params.id && typeof params.id === 'string' && params.id.includes('::')) {
        songs = await invoke('get_album_songs', { albumId: params.id });
      } else if (params.name) {
        songs = await invoke('get_artist_songs', { name: params.name });
      }
      list.innerHTML = '';
      if (!songs || songs.length === 0) {
        list.innerHTML = `<div class="song-row-empty" style="padding:16px;color:var(--md-sys-color-on-surface-variant)">${t('screen.NoTracksFound')}</div>`;
        return;
      }
      for (const s of songs) {
        const row = document.createElement('div');
        row.className = 'song-row';
        const title = document.createElement('span');
        title.textContent = `${s.track ? s.track + '. ' : ''}${s.title ?? t('screen.Unknown')}`;
        const artist = document.createElement('span');
        artist.className = 'song-row-secondary';
        artist.textContent = ` — ${s.artist ?? ''}`;
        row.append(title, artist);
        row.addEventListener('click', async () => {
          try { await invoke('play_song', { songId: s.id }); } catch {}
        });
        list.appendChild(row);
      }
    } catch (e) {
      list.innerHTML = `<div class="song-row-empty" style="padding:16px;color:var(--md-sys-color-error)">${t('screen.ErrorLoadingSongs')}</div>`;
    }
  };

  loadSongs();
  container.appendChild(list);
  return container;
}

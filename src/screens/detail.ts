import { invoke } from "@tauri-apps/api/core";
import { screenContainer } from "./common.js";
import { RouteParams } from "../router.js";

export function detailScreen(params: RouteParams) {
  const container = screenContainer(params.name || `Detail: ${params.id ?? ''}`, document.createElement('div'));
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
        list.innerHTML = '<div style="padding:16px;color:var(--text-muted)">No tracks found</div>';
        return;
      }
      for (const s of songs) {
        const row = document.createElement('div');
        row.style.padding = '8px 12px';
        row.style.cursor = 'pointer';
        row.style.borderBottom = '1px solid var(--border)';
        row.textContent = `${s.track ? s.track + '. ' : ''}${s.title ?? 'Unknown'} — ${s.artist ?? ''}`;
        row.addEventListener('click', async () => {
          try { await invoke('play_song', { song_id: s.id }); } catch {}
        });
        list.appendChild(row);
      }
    } catch (e) {
      list.innerHTML = `<div style="padding:16px;color:var(--err)">Error loading songs</div>`;
    }
  };

  loadSongs();
  container.appendChild(list);
  return container;
}

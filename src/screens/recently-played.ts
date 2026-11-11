import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n/index.js";

export function recentlyPlayedScreen() {
  const container = document.createElement('div');
  container.className = 'screen-container';
  const header = document.createElement('div');
  header.className = 'screen-header';
  const h1 = document.createElement('h1');
  h1.textContent = t('screen.recentlyPlayed');
  header.appendChild(h1);
  container.appendChild(header);
  const grid = document.createElement('div');
  container.appendChild(grid);
  invoke('get_recently_played', { limit: 200 }).then((songs: any) => {
    const arr = songs as any[];
    for (const s of arr) {
      const div = document.createElement('div');
      div.className = 'song-row';
      const title = document.createElement('span');
      title.textContent = `${s.title ?? t('screen.Unknown')}`;
      const secondary = document.createElement('span');
      secondary.className = 'song-row-secondary';
      secondary.textContent = `— ${s.artist ?? ''} • ${s.album ?? ''}`;
      div.append(title, secondary);
      div.addEventListener('click', async () => {
        try { await invoke('play_song', { songId: s.id }); } catch {}
      });
      grid.appendChild(div);
    }
  });
  return container;
}

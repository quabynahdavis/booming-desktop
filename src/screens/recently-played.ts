import { invoke } from "@tauri-apps/api/core";

export function recentlyPlayedScreen() {
  const container = document.createElement('div');
  container.className = 'screen-container';
  const header = document.createElement('div');
  header.className = 'screen-header';
  const h1 = document.createElement('h1');
  h1.textContent = 'Recently Played';
  header.appendChild(h1);
  container.appendChild(header);
  const grid = document.createElement('div');
  container.appendChild(grid);
  invoke('get_recently_played', { limit: 200 }).then((songs: any) => {
    const arr = songs as any[];
    for (const s of arr) {
      const div = document.createElement('div');
      div.style.padding = '8px 12px';
      div.style.cursor = 'pointer';
      div.style.borderBottom = '1px solid var(--border)';
      div.textContent = `${s.title ?? 'Unknown'} — ${s.artist ?? ''} • ${s.album ?? ''}`;
      div.addEventListener('click', async () => {
        try { await invoke('play_song', { song_id: s.id }); } catch {}
      });
      grid.appendChild(div);
    }
  });
  return container;
}

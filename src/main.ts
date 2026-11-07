import { invoke } from "@tauri-apps/api/core";
import "./styles.css";
import "./theme/tokens.css";
import "@fontsource/material-symbols-outlined/latin-400.css";
import "@material/web/typography/md-typescale-styles.js";
import { FALLBACK_COVER } from "./theme/fallback-cover.js";
import { applyDynamicCover } from "./theme/dynamic-color.js";
import { Router } from "./router.js";
import { songsScreen } from "./screens/songs.js";
import { albumsScreen } from "./screens/albums.js";
import { artistsScreen } from "./screens/artists.js";
import { foldersScreen } from "./screens/folders.js";
import { playlistsScreen } from "./screens/playlists.js";
import { recentlyPlayedScreen } from "./screens/recently-played.js";
import { mostPlayedScreen } from "./screens/most-played.js";
import { settingsScreen } from "./screens/settings.js";
import { nowPlayingScreen } from "./screens/now-playing.js";
import { equalizerScreen } from "./screens/equalizer.js";

Router.register("songs", songsScreen);
Router.register("albums", albumsScreen);
Router.register("artists", artistsScreen);
Router.register("folders", foldersScreen);
Router.register("playlists", playlistsScreen);
Router.register("recently-played", recentlyPlayedScreen);
Router.register("most-played", mostPlayedScreen);
Router.register("now-playing", nowPlayingScreen);
Router.register("equalizer", equalizerScreen);
Router.register("settings", settingsScreen);

Router.init();

interface Song {
  id: number;
  title?: string;
  artist?: string;
  album?: string;
  duration?: number;
  path: string;
}

interface PlaybackState {
  state: 'playing' | 'paused' | 'stopped';
  position: number;
  duration: number;
  current_song_id?: number;
  current_song?: Song & { cover?: number[] };
  queue: number[];
}

// Player panel logic
const panel = document.querySelector<HTMLElement>('#player-panel');
const handle = document.querySelector<HTMLElement>('.player-handle');
const playBtn = document.querySelector<HTMLButtonElement>('#player-play');
const prevBtn = document.querySelector<HTMLButtonElement>('#player-prev');
const nextBtn = document.querySelector<HTMLButtonElement>('#player-next');
const titleEl = document.querySelector<HTMLElement>('#player-title');
const artistEl = document.querySelector<HTMLElement>('#player-artist');
const coverEl = document.querySelector<HTMLImageElement>('#player-cover');
let lastSongId: number | null = null;
const progressEl = document.querySelector<HTMLInputElement>('#player-progress');
const queueEl = document.querySelector<HTMLElement>('#player-queue-list');

let dragging = false;
let startY = 0;
let startHeight = 0;
handle?.addEventListener('pointerdown', (e) => {
  dragging = true;
  startY = e.clientY;
  startHeight = panel?.offsetHeight ?? 72;
  (e.target as HTMLElement).setPointerCapture(e.pointerId);
});
window.addEventListener('pointermove', (e) => {
  if (!dragging || !panel) return;
  const dy = startY - e.clientY;
  const newHeight = Math.min(window.innerHeight * 0.9, Math.max(72, startHeight + dy));
  panel.style.height = `${newHeight}px`;
  if (newHeight > 300) {
    panel.classList.add('expanded');
  } else {
    panel.classList.remove('expanded');
  }
});
window.addEventListener('pointerup', () => { dragging = false; });

playBtn?.addEventListener('click', async () => {
  try {
    const state = await invoke<PlaybackState>('get_playback_state');
    if (state.state === 'playing') {
      await invoke('pause_playback');
    } else {
      await invoke('resume_playback');
    }
  } catch {}
});

prevBtn?.addEventListener('click', async () => {
  try { await invoke('previous_track'); } catch {}
});
nextBtn?.addEventListener('click', async () => {
  try { await invoke('next_track'); } catch {}
});

async function pollState() {
  try {
    const state = await invoke<PlaybackState>('get_playback_state');
    if (titleEl) titleEl.textContent = state.current_song?.title ?? 'No track';
    if (artistEl) artistEl.textContent = state.current_song?.artist ?? '';
    if (playBtn) playBtn.textContent = state.state === 'playing' ? '⏸' : '▶';
    if (progressEl && state.duration > 0) {
      const pct = Math.min(100, Math.max(0, (state.position / state.duration) * 100));
      progressEl.value = String(pct);
    }
    if (coverEl && state.current_song_id) {
      if (lastSongId !== state.current_song_id) {
        lastSongId = state.current_song_id;
        let coverUrl: string | null = null;
        try {
          coverUrl = await invoke<string | null>('get_cover_data_url', { songId: state.current_song_id });
        } catch {
          coverUrl = null;
        }
        coverEl.src = coverUrl || FALLBACK_COVER;
        // Dynamic accent scheme: real artwork → accent from art; none → static brand scheme.
        applyDynamicCover(coverUrl);
      }
    }
    if (queueEl) {
      queueEl.innerHTML = '';
      state.queue.forEach(id => {
        const div = document.createElement('div');
        div.className = 'queue-row';
        div.textContent = `Song ${id}`;
        queueEl.appendChild(div);
      });
    }
  } catch {}
}
setInterval(pollState, 500);
pollState();

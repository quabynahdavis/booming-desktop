import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

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

const listEl = document.querySelector<HTMLElement>('#song-list');
const searchEl = document.querySelector<HTMLInputElement>('#search');
const emptyEl = document.querySelector<HTMLElement>('#empty-state');

let songs: Song[] = [];

function formatDuration(sec?: number) {
  if (!sec) return '—';
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${m}:${s.toString().padStart(2,'0')}`;
}

function render() {
  if (!listEl) return;
  listEl.innerHTML = '';
  const q = (searchEl?.value ?? '').toLowerCase();
  const filtered = songs.filter(s => {
    if (!q) return true;
    return [s.title, s.artist, s.album].filter(Boolean).some(v => v!.toLowerCase().includes(q));
  });
  if (filtered.length === 0) {
    if (emptyEl) emptyEl.hidden = false;
    return;
  }
  if (emptyEl) emptyEl.hidden = true;
  for (const s of filtered.slice(0, 200)) {
    const div = document.createElement('div');
    div.textContent = `${s.title ?? 'Unknown'} — ${s.artist ?? ''} • ${s.album ?? ''} • ${formatDuration(s.duration)}`;
    div.style.padding = '8px';
    div.style.cursor = 'pointer';
    div.addEventListener('click', async () => {
      try { await invoke('play_song', { song_id: s.id }); } catch {}
    });
    listEl.appendChild(div);
  }
}

async function loadSongs() {
  songs = [];
  render();
}

document.querySelector<HTMLElement>('#add-folder')?.addEventListener('click', async () => {
  try {
    await invoke('scan_start', { root: '/tmp/music' });
    await loadSongs();
  } catch {}
});

let timer: number;
searchEl?.addEventListener('input', () => {
  clearTimeout(timer);
  timer = window.setTimeout(render, 200);
});

loadSongs();

// Player panel logic
const panel = document.querySelector<HTMLElement>('#player-panel');
const handle = document.querySelector<HTMLElement>('.player-handle');
const playBtn = document.querySelector<HTMLButtonElement>('#player-play');
const prevBtn = document.querySelector<HTMLButtonElement>('#player-prev');
const nextBtn = document.querySelector<HTMLButtonElement>('#player-next');
const titleEl = document.querySelector<HTMLElement>('#player-title');
const artistEl = document.querySelector<HTMLElement>('#player-artist');
const coverEl = document.querySelector<HTMLImageElement>('#player-cover');
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
    if (queueEl) {
      queueEl.innerHTML = '';
      state.queue.forEach(id => {
        const div = document.createElement('div');
        div.textContent = `Song ${id}`;
        div.style.padding = '4px';
        queueEl.appendChild(div);
      });
    }
  } catch {}
}
setInterval(pollState, 500);
pollState();

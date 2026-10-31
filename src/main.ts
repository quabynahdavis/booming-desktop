import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./styles.css";
import '@material/web/top-app-bar/md-top-app-bar.js';
import '@material/web/icon-button/md-icon-button.js';
import '@material/web/textfield/md-text-field.js';
import '@material/web/list/md-list.js';
import '@material/web/list/md-list-item.js';
import '@material/web/button/filled-tonal-button.js';
import '@material/web/icon/md-icon.js';
import { styles as typescaleStyles } from '@material/web/typography/md-typescale-styles.js';

if (typeof document !== 'undefined' && typescaleStyles?.styleSheet) {
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, typescaleStyles.styleSheet];
}

interface Song {
  id: number;
  title?: string;
  artist?: string;
  album?: string;
  duration?: number;
  path: string;
}

const listEl = document.querySelector<HTMLElement>('#song-list');
const searchEl = document.querySelector<HTMLElement>('md-text-field');
const emptyEl = document.querySelector<HTMLElement>('#empty-state');
const addBtn = document.querySelector<HTMLElement>('#add-folder');

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
  const filtered = songs.filter(s => {
    const q = (searchEl as any)?.value?.toLowerCase() ?? '';
    if (!q) return true;
    return [s.title, s.artist, s.album].filter(Boolean).some(v => v!.toLowerCase().includes(q));
  });
  if (filtered.length === 0) {
    if (emptyEl) emptyEl.hidden = false;
    return;
  }
  if (emptyEl) emptyEl.hidden = true;
  for (const s of filtered.slice(0, 200)) {
    const item = document.createElement('md-list-item');
    item.setAttribute('headline', s.title ?? 'Unknown');
    item.setAttribute('supporting-text', `${s.artist ?? ''} • ${s.album ?? ''} • ${formatDuration(s.duration)}`);
    item.addEventListener('click', () => console.log('play', s.path));
    listEl.appendChild(item);
  }
}

async function loadSongs() {
  try {
    // Stub: future IPC list_songs
    songs = [];
    render();
  } catch {}
}

if (addBtn) {
  addBtn.addEventListener('click', async () => {
    try {
      const dir = await open({ directory: true, multiple: false });
      if (typeof dir === 'string') {
        await invoke('scan_start', { root: dir });
        // reload after scan
        await loadSongs();
      }
    } catch {}
  });
}

if (searchEl) {
  let timer: number;
  searchEl.addEventListener('input', () => {
    clearTimeout(timer);
    timer = window.setTimeout(render, 200);
  });
}

loadSongs();

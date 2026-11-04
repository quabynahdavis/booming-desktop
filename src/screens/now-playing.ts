import { invoke } from "@tauri-apps/api/core";
import { Router } from "../router.js";

const FALLBACK_COVER =
  "data:image/svg+xml;utf8," +
  encodeURIComponent(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\">" +
      "<defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">" +
      "<stop offset=\"0\" stop-color=\"#7c3aed\"/><stop offset=\"1\" stop-color=\"#06b6d4\"/>" +
      "</linearGradient></defs>" +
      "<rect width=\"64\" height=\"64\" fill=\"url(#g)\"/>" +
      "<text x=\"32\" y=\"38\" fill=\"#fff\" font-size=\"26\" text-anchor=\"middle\" font-family=\"system-ui\">&#x266B;</text>" +
    "</svg>"
  );

interface PlaybackState {
  state: 'playing' | 'paused' | 'stopped';
  position: number;
  duration: number;
  current_song_id?: number;
  current_song?: { title?: string; artist?: string; album?: string };
  queue: number[];
}

export function nowPlayingScreen() {
  const container = document.createElement('div');
  container.className = 'screen-container now-playing';
  container.style.display = 'flex';
  container.style.flexDirection = 'column';
  container.style.alignItems = 'center';
  container.style.padding = '32px';
  container.style.gap = '24px';
  container.style.minHeight = '60vh';

  // cover art
  const coverWrapper = document.createElement('div');
  coverWrapper.style.position = 'relative';
  coverWrapper.style.width = '320px';
  coverWrapper.style.height = '320px';
  coverWrapper.style.borderRadius = '24px';
  coverWrapper.style.overflow = 'hidden';
  coverWrapper.style.background = 'linear-gradient(135deg,#7c3aed,#06b6d4)';
  coverWrapper.style.display = 'flex';
  coverWrapper.style.alignItems = 'center';
  coverWrapper.style.justifyContent = 'center';

  const coverImg = document.createElement('img');
  coverImg.style.width = '100%';
  coverImg.style.height = '100%';
  coverImg.style.objectFit = 'cover';
  coverImg.src = FALLBACK_COVER;
  coverWrapper.appendChild(coverImg);

  // metadata
  const meta = document.createElement('div');
  meta.style.textAlign = 'center';
  meta.style.color = 'white';
  meta.style.textShadow = '0 2px 8px rgba(0,0,0,0.5)';

  const titleEl = document.createElement('div');
  titleEl.style.fontSize = '28px';
  titleEl.style.fontWeight = '600';
  titleEl.textContent = 'No track';
  meta.appendChild(titleEl);

  const artistEl = document.createElement('div');
  artistEl.style.fontSize = '18px';
  artistEl.style.opacity = '0.85';
  meta.appendChild(artistEl);

  const albumEl = document.createElement('div');
  albumEl.style.fontSize = '14px';
  albumEl.style.opacity = '0.7';
  meta.appendChild(albumEl);

  // progress
  const progressRow = document.createElement('div');
  progressRow.style.width = '100%';
  progressRow.style.maxWidth = '720px';
  progressRow.style.display = 'flex';
  progressRow.style.flexDirection = 'column';
  progressRow.style.gap = '8px';

  const timeRow = document.createElement('div');
  timeRow.style.display = 'flex';
  timeRow.style.justifyContent = 'space-between';
  timeRow.style.fontSize = '12px';
  timeRow.style.opacity = '0.8';
  const currentTime = document.createElement('span');
  const totalTime = document.createElement('span');
  timeRow.appendChild(currentTime);
  timeRow.appendChild(totalTime);

  const slider = document.createElement('md-slider');
  slider.setAttribute('min', '0');
  slider.setAttribute('max', '100');
  slider.setAttribute('value', '0');
  slider.style.width = '100%';

  progressRow.appendChild(timeRow);
  progressRow.appendChild(slider);

  // controls
  const controls = document.createElement('div');
  controls.style.display = 'flex';
  controls.style.alignItems = 'center';
  controls.style.gap = '12px';

  const shuffleBtn = document.createElement('md-icon-button');
  shuffleBtn.setAttribute('aria-label', 'shuffle');
  shuffleBtn.innerHTML = '<md-icon>shuffle</md-icon>';

  const prevBtn = document.createElement('md-icon-button');
  prevBtn.setAttribute('aria-label', 'previous');
  prevBtn.innerHTML = '<md-icon>skip_previous</md-icon>';

  const playBtn = document.createElement('md-icon-button');
  playBtn.setAttribute('aria-label', 'play-pause');
  playBtn.innerHTML = '<md-icon>play_arrow</md-icon>';

  const nextBtn = document.createElement('md-icon-button');
  nextBtn.setAttribute('aria-label', 'next');
  nextBtn.innerHTML = '<md-icon>skip_next</md-icon>';

  const repeatBtn = document.createElement('md-icon-button');
  repeatBtn.setAttribute('aria-label', 'repeat');
  repeatBtn.innerHTML = '<md-icon>repeat</md-icon>';

  controls.append(shuffleBtn, prevBtn, playBtn, nextBtn, repeatBtn);

  // bottom bar for volume etc (placeholder)
  const bottomBar = document.createElement('div');
  bottomBar.style.marginTop = '32px';
  bottomBar.style.width = '100%';
  bottomBar.style.maxWidth = '720px';
  bottomBar.style.display = 'flex';
  bottomBar.style.alignItems = 'center';
  bottomBar.style.gap = '16px';
  bottomBar.style.padding = '12px';
  bottomBar.style.background = 'rgba(0,0,0,0.2)';
  bottomBar.style.borderRadius = '16px';

  const volumeSlider = document.createElement('md-slider');
  volumeSlider.setAttribute('id', 'np-volume');
  volumeSlider.setAttribute('min', '0');
  volumeSlider.setAttribute('max', '100');
  volumeSlider.setAttribute('value', '100');
  volumeSlider.style.flex = '1';

  const timerBtn = document.createElement('md-icon-button');
  timerBtn.setAttribute('aria-label', 'sleep timer');
  timerBtn.innerHTML = '<md-icon>timer</md-icon>';

  bottomBar.appendChild(volumeSlider);
  bottomBar.appendChild(timerBtn);

  container.append(coverWrapper, meta, progressRow, controls, bottomBar);

  // state
  let lastSongId: number | null = null;
  let currentDuration = 0;
  let repeatMode: 'none' | 'one' | 'all' = 'none';
  let shuffle = false;

  async function updateState() {
    try {
      const state = await invoke<PlaybackState>('get_playback_state');
      titleEl.textContent = state.current_song?.title ?? 'No track';
      artistEl.textContent = state.current_song?.artist ?? '';
      albumEl.textContent = state.current_song?.album ?? '';

      if (state.duration > 0) {
        currentDuration = state.duration;
        const pct = Math.min(100, Math.max(0, (state.position / state.duration) * 100));
        (slider as any).value = pct;
        currentTime.textContent = formatTime(state.position);
        totalTime.textContent = formatTime(state.duration);
      }

      const isPlaying = state.state === 'playing';
      playBtn.innerHTML = `<md-icon>${isPlaying ? 'pause' : 'play_arrow'}</md-icon>`;

      if (state.current_song_id && state.current_song_id !== lastSongId) {
        lastSongId = state.current_song_id;
        try {
          const dataUrl = await invoke<string | null>('get_cover_data_url', { song_id: state.current_song_id });
          coverImg.src = dataUrl || FALLBACK_COVER;
        } catch {
          coverImg.src = FALLBACK_COVER;
        }
      }
    } catch {}
  }

  function formatTime(sec: number) {
    if (!isFinite(sec)) return '0:00';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
  }

  // wire controls
  playBtn.addEventListener('click', async () => {
    try {
      const state = await invoke<PlaybackState>('get_playback_state');
      if (state.state === 'playing') await invoke('pause_playback');
      else await invoke('resume_playback');
    } catch {}
  });
  prevBtn.addEventListener('click', async () => { try { await invoke('previous_track'); } catch {} });
  nextBtn.addEventListener('click', async () => { try { await invoke('next_track'); } catch {} });
  shuffleBtn.addEventListener('click', async () => {
    try {
      shuffle = !shuffle;
      await invoke('set_shuffle', { enabled: shuffle });
      shuffleBtn.style.color = shuffle ? 'var(--md-sys-color-primary)' : '';
    } catch {}
  });
  repeatBtn.addEventListener('click', async () => {
    try {
      repeatMode = repeatMode === 'none' ? 'one' : repeatMode === 'one' ? 'all' : 'none';
      const modeMap = { none: 'None', one: 'One', all: 'All' } as const;
      await invoke('set_repeat_mode', { mode: modeMap[repeatMode] });
      repeatBtn.style.color = repeatMode !== 'none' ? 'var(--md-sys-color-primary)' : '';
      repeatBtn.innerHTML = `<md-icon>${repeatMode === 'one' ? 'repeat_one' : 'repeat'}</md-icon>`;
    } catch {}
  });

  // seek
  let seeking = false;
  slider.addEventListener('input', () => { seeking = true; });
  slider.addEventListener('change', async () => {
    if (!seeking || currentDuration === 0) return;
    const pct = Number((slider as any).value);
    const secs = (pct / 100) * currentDuration;
    try { await invoke('seek_playback', { secs }); } catch {}
    seeking = false;
  });

  // poll
  setInterval(updateState, 500);
  updateState();

  return container;
}

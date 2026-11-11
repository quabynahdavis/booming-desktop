import { invoke } from "@tauri-apps/api/core";
import type { MdDialog } from '@material/web/dialog/dialog.js';
import { parseLrc } from "../lyrics/lrc.js";
import { fetchLyrics } from "../lyrics/providers.js";
import { createLyricsView } from "../lyrics/lyrics-view.js";
import type { LyricsView } from "../lyrics/lyrics-view.js";
import { FALLBACK_COVER } from "../theme/fallback-cover.js";
import { applyDynamicCover } from "../theme/dynamic-color.js";
import { t } from "../i18n/index.js";

interface PlaybackState {
  state: 'playing' | 'paused' | 'stopped';
  position: number;
  duration: number;
  current_song_id?: number;
  current_song?: { title?: string; artist?: string; album?: string; duration?: number; path?: string };
  queue: number[];
}
export function nowPlayingScreen() {
  const container = document.createElement('div');
  container.className = 'np-screen';

  // Top app bar: pane label + lyrics toggle (accent when open)
  const topbar = document.createElement('div');
  topbar.className = 'np-topbar';
  const barTitle = document.createElement('div');
  barTitle.className = 'np-bar-title';
  barTitle.textContent = t('nowPlaying.NowPlaying');

  // Two-column layout: main playback column + (hidden until toggled) lyrics pane
  const layout = document.createElement('div');
  layout.className = 'np-body';

  const mainCol = document.createElement('div');
  mainCol.className = 'np-main';

  const lyricsWrap = document.createElement('div');
  lyricsWrap.className = 'np-lyrics';
  lyricsWrap.style.display = 'none';

  // cover art block: cover + left-aligned metadata below it (app look)
  const coverBlock = document.createElement('div');
  coverBlock.className = 'np-cover-block';

  const coverWrapper = document.createElement('div');
  coverWrapper.className = 'np-cover';

  const coverImg = document.createElement('img');
  coverImg.src = FALLBACK_COVER;
  coverWrapper.appendChild(coverImg);

  // metadata
  const meta = document.createElement('div');
  meta.className = 'np-meta';

  const titleEl = document.createElement('div');
  titleEl.className = 'np-title';
  titleEl.textContent = t('screen.noTrack');

  const subtitleRow = document.createElement('div');
  subtitleRow.className = 'np-subtitle';
  const artistEl = document.createElement('span');
  const sepEl = document.createElement('span');
  sepEl.className = 'np-separator';
  sepEl.textContent = ' • ';
  const albumEl = document.createElement('span');
  subtitleRow.append(artistEl, sepEl, albumEl);

  meta.append(titleEl, subtitleRow);
  coverBlock.append(coverWrapper, meta);

  // progress
  const progressRow = document.createElement('div');
  progressRow.className = 'np-progress';

  const timeRow = document.createElement('div');
  timeRow.className = 'np-times';
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
  controls.className = 'np-controls';

  const shuffleBtn = document.createElement('md-icon-button');
  shuffleBtn.setAttribute('aria-label', 'shuffle');
  shuffleBtn.innerHTML = '<md-icon>shuffle</md-icon>';

  const prevBtn = document.createElement('md-icon-button');
  prevBtn.className = 'np-skip';
  prevBtn.setAttribute('aria-label', 'previous');
  prevBtn.innerHTML = '<md-icon>skip_previous</md-icon>';

  const playBtn = document.createElement('md-icon-button');
  playBtn.className = 'np-play';
  playBtn.setAttribute('aria-label', 'play-pause');
  playBtn.innerHTML = '<md-icon>play_arrow</md-icon>';

  const nextBtn = document.createElement('md-icon-button');
  nextBtn.className = 'np-skip';
  nextBtn.setAttribute('aria-label', 'next');
  nextBtn.innerHTML = '<md-icon>skip_next</md-icon>';

  const repeatBtn = document.createElement('md-icon-button');
  repeatBtn.setAttribute('aria-label', 'repeat');
  repeatBtn.innerHTML = '<md-icon>repeat</md-icon>';

  const lyricsBtn = document.createElement('md-icon-button');
  lyricsBtn.setAttribute('aria-label', 'lyrics');
  lyricsBtn.setAttribute('title', t('nowPlaying.Lyrics'));
  lyricsBtn.innerHTML = '<md-icon>lyrics</md-icon>';

  topbar.append(barTitle, lyricsBtn);
  controls.append(shuffleBtn, prevBtn, playBtn, nextBtn, repeatBtn);
  mainCol.append(coverBlock, progressRow, controls);

  // bottom bar for volume etc
  const bottomBar = document.createElement('div');
  bottomBar.className = 'np-bottombar';

  const volumeSlider = document.createElement('md-slider');
  volumeSlider.setAttribute('id', 'np-volume');
  volumeSlider.setAttribute('min', '0');
  volumeSlider.setAttribute('max', '100');
  volumeSlider.setAttribute('value', '100');
  volumeSlider.style.flex = '1';

  const balanceSlider = document.createElement('md-slider');
  balanceSlider.setAttribute('id', 'np-balance');
  balanceSlider.setAttribute('min', '-100');
  balanceSlider.setAttribute('max', '100');
  balanceSlider.setAttribute('value', '0');
  balanceSlider.style.width = '120px';

  const speedWrap = document.createElement('div');
  speedWrap.className = 'np-speeds';
  const speeds = [0.5,0.75,1.0,1.25,1.5];
  speeds.forEach(s => {
    const b = document.createElement('md-text-button');
    b.textContent = `${s}x`;
    b.dataset.speed = String(s);
    speedWrap.appendChild(b);
  });

  const timerBtn = document.createElement('md-icon-button');
  timerBtn.setAttribute('aria-label', 'sleep timer');
  timerBtn.innerHTML = '<md-icon>timer</md-icon>';

  bottomBar.appendChild(volumeSlider);
  bottomBar.appendChild(balanceSlider);
  bottomBar.appendChild(speedWrap);
  bottomBar.appendChild(timerBtn);

  // sleep timer dialog
  const dialog = document.createElement('md-dialog') as MdDialog;
  dialog.id = 'sleep-timer-dialog';
  const dialogTitle = document.createElement('div');
  dialogTitle.slot = 'headline';
  dialogTitle.textContent = t('nowPlaying.SleepTimer');
  const dialogContent = document.createElement('div');
  dialogContent.innerHTML = `
    <div style="display:flex;gap:8px;margin:8px 0;">
      <button data-preset="15">15m</button>
      <button data-preset="30">30m</button>
      <button data-preset="60">60m</button>
    </div>
    <md-text-field id="sleep-minutes" type="number" label="${t('nowPlaying.Minutes')}" value="30"></md-text-field>
    <div style="margin-top:8px;display:flex;gap:8px;justify-content:flex-end;">
      <md-text-button id="sleep-cancel">${t('nowPlaying.Cancel')}</md-text-button>
      <md-filled-button id="sleep-set">Set</md-filled-button>
    </div>
  `;
  const dialogActions = document.createElement('div');
  dialogActions.slot = 'actions';
  dialog.append(dialogTitle, dialogContent);

  // lyrics pane (created below, wired here so it exists for the layout row)
  const lyricsView: LyricsView = createLyricsView((timeMs) => {
    invoke('seek_playback', { secs: timeMs / 1000 }).catch(() => {});
  });
  lyricsWrap.appendChild(lyricsView.el);
  layout.append(mainCol, lyricsWrap);
  container.append(topbar, layout, bottomBar, dialog);

  // state
  let lastSongId: number | null = null;
  let currentDuration = 0;
  let repeatMode: 'none' | 'one' | 'all' = 'none';
  let shuffle = false;

  // lyrics state
  interface CurrentSong { title?: string; artist?: string; album?: string; duration?: number; path?: string }
  let currentSong: CurrentSong | null = null;
  let loadedRaw: string | null = null; // best raw LRC text for the editor
  let loadedFrom: string | null = null;
  let lyricsLoadToken = 0;

  async function loadLyrics(song: CurrentSong | undefined, fallbackDuration: number) {
    const token = ++lyricsLoadToken;
    currentSong = song ?? null;
    loadedRaw = null;
    loadedFrom = null;
    lyricsView.setHeader(song?.title ?? null, song?.artist ?? null);
    if (!song?.title || !song?.artist) {
      lyricsView.setLyrics(null, null, null);
      lyricsView.setStatus(t('screen.NoTrackPlaying'));
      return;
    }
    lyricsView.setLyrics(null, null, null);

    // Preference order (P7 contract): sidecar .lrc → LRCLIB → nothing.
    if (song.path) {
      let sidecarText: string | null = null;
      try {
        sidecarText = await invoke<string | null>('load_lrc_sidecar', { songPath: song.path });
      } catch { sidecarText = null; }
      if (token !== lyricsLoadToken) return;
      if (sidecarText && sidecarText.trim()) {
        loadedRaw = sidecarText;
        loadedFrom = 'sidecar .lrc';
        applyLyricsText(sidecarText, loadedFrom);
        return;
      }
    }

    lyricsView.setStatus(t('nowPlaying.LoadingLyrics'));
    const durationSecs = song.duration || (fallbackDuration > 0 ? fallbackDuration : null);
    let res: Awaited<ReturnType<typeof fetchLyrics>> | null = null;
    try {
      res = await fetchLyrics({ artist: song.artist, title: song.title, album: song.album ?? null, durationSecs });
    } catch {
      res = null;
    }
    if (token !== lyricsLoadToken) return; // a newer load superseded this one
    if (!res || !res.found) {
      lyricsView.setLyrics(null, null, null);
      lyricsView.setStatus(t('screen.NoLyricsFound'));
      return;
    }
    if (res.instrumental) {
      lyricsView.setLyrics(null, null, null);
      lyricsView.setStatus(t('screen.InstrumentalTrack'));
      return;
    }
    if (res.syncedLyrics) {
      loadedRaw = res.syncedLyrics;
      loadedFrom = 'LRCLIB';
      applyLyricsText(res.syncedLyrics, loadedFrom);
    } else {
      loadedRaw = res.plainLyrics;
      loadedFrom = 'LRCLIB (plain)';
      applyLyricsText(res.plainLyrics ?? '', loadedFrom);
    }
  }

  /** Parse + render LRC (or plain) text; keeps `loadedRaw` for the editor. */
  function applyLyricsText(raw: string, source: string) {
    if (!raw.trim()) {
      lyricsView.setLyrics(null, null, null);
      lyricsView.setStatus(t('screen.LyricsFileEmpty'));
      return;
    }
    const parsed = parseLrc(raw);
    if (parsed.lines.some((l) => l.timeMs > 0)) {
      lyricsView.setLyrics(parsed, null, source);
      lyricsView.setStatus(null);
    } else {
      lyricsView.setLyrics(null, raw, source === 'sidecar .lrc' ? `${source} (plain)` : source);
      lyricsView.setStatus(null);
    }
  }

  // lyrics editor (pencil → md-dialog with a textarea)
  const editBtn = document.createElement('md-icon-button');
  editBtn.setAttribute('aria-label', 'edit lyrics');
  editBtn.setAttribute('title', 'Edit lyrics');
  editBtn.innerHTML = '<md-icon>edit</md-icon>';
  editBtn.style.cssText = 'position:absolute;top:4px;right:0;';

  const editDialog = document.createElement('md-dialog') as MdDialog;
  editDialog.id = 'lyrics-editor-dialog';
  const editTitle = document.createElement('div');
  editTitle.slot = 'headline';
  editTitle.textContent = t('nowPlaying.EditLyrics');
  const editContent = document.createElement('div');
  editContent.style.cssText = 'display:flex;flex-direction:column;gap:8px;min-width:560px;';
  const editBar = document.createElement('div');
  editBar.style.cssText = 'font-size:12px;color:var(--md-sys-color-on-surface-variant);';
  const editArea = document.createElement('textarea');
  editArea.rows = 16;
  editArea.spellcheck = false;
  editArea.style.cssText =
    'width:100%;font-family:ui-monospace,Menlo,monospace;font-size:12.5px;line-height:1.5;' +
    'background:var(--md-sys-color-surface-container);color:var(--md-sys-color-on-surface);border:1px solid var(--md-sys-color-outline-variant);' +
    'border-radius:10px;padding:10px;resize:vertical;';
  editContent.append(editBar, editArea);
  const editActions = document.createElement('div');
  editActions.slot = 'actions';
  const editCancel = document.createElement('md-text-button');
  editCancel.textContent = t('nowPlaying.Cancel');
  const editSave = document.createElement('md-filled-button');
  editSave.textContent = t('nowPlaying.SaveSidecar');
  editActions.append(editCancel, editSave);
  editDialog.append(editTitle, editContent, editActions);
  container.append(editDialog);

  // the pencil lives in the pane's top-right corner; hidden with the pane
  lyricsWrap.style.position = 'relative';
  lyricsWrap.appendChild(editBtn);

  async function openLyricsEditor() {
    if (!currentSong?.title) {
      return;
    }
    editBar.textContent = `${currentSong.title}${currentSong.artist ? ` — ${currentSong.artist}` : ''} · source: ${loadedFrom ?? 'none'}`;
    editArea.value = loadedRaw ?? '';
    editDialog.show();
  }

  editBtn.addEventListener('click', () => { void openLyricsEditor(); });
  editCancel.addEventListener('click', () => { editDialog.close(); });
  editSave.addEventListener('click', async () => {
    const song = currentSong;
    if (!song?.path) {
      editBar.textContent = 'Cannot save: the current track has no known path on disk.';
      return;
    }
    try {
      await invoke('save_lrc_sidecar', { songPath: song.path, lrcText: editArea.value });
      editDialog.close();
      // reload so the view reflects exactly what was written ("sidecar → provider")
      if (lastSongId !== null) await loadLyrics(song, currentDuration);
    } catch (err) {
      editBar.textContent = `Save failed: ${String(err)}`;
    }
  });

  async function updateState() {
    try {
      const state = await invoke<PlaybackState>('get_playback_state');
      titleEl.textContent = state.current_song?.title ?? t('screen.noTrack');
      artistEl.textContent = state.current_song?.artist ?? '';
      albumEl.textContent = state.current_song?.album ?? '';
      sepEl.style.display = state.current_song?.album ? '' : 'none';

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
        let coverUrl: string | null = null;
        try {
          coverUrl = await invoke<string | null>('get_cover_data_url', { songId: state.current_song_id });
        } catch {
          coverUrl = null;
        }
        coverImg.src = coverUrl || FALLBACK_COVER;
        // Same dedup'd hook as main.ts — harmless double feed on song change.
        applyDynamicCover(coverUrl);
        void loadLyrics(state.current_song, state.duration);
      }
      lyricsView.update(state.position);
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
      shuffleBtn.toggleAttribute('selected', shuffle);
    } catch {}
  });
  repeatBtn.addEventListener('click', async () => {
    try {
      repeatMode = repeatMode === 'none' ? 'one' : repeatMode === 'one' ? 'all' : 'none';
      const modeMap = { none: 'None', one: 'One', all: 'All' } as const;
      await invoke('set_repeat_mode', { mode: modeMap[repeatMode] });
      repeatBtn.toggleAttribute('selected', repeatMode !== 'none');
      repeatBtn.innerHTML = `<md-icon>${repeatMode === 'one' ? 'repeat_one' : 'repeat'}</md-icon>`;
    } catch {}
  });

  // volume / balance / speed
  volumeSlider.addEventListener('input', () => {
    const v = Number((volumeSlider as any).value) / 100;
    invoke('set_volume', { vol: v }).catch(() => {});
  });
  balanceSlider.addEventListener('input', () => {
    const b = Number((balanceSlider as any).value) / 100;
    invoke('set_balance', { balance: b }).catch(() => {});
  });
  speedWrap.querySelectorAll('md-text-button').forEach((btn: any) => {
    btn.addEventListener('click', () => {
      const s = Number(btn.dataset.speed);
      invoke('set_speed', { speed: s }).catch(() => {});
    });
  });
  // keyboard bindings
  const keyHandler = (e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.code === 'Space') {
      e.preventDefault();
      playBtn.click();
    } else if (e.code === 'ArrowLeft') {
      e.preventDefault();
      invoke('seek_playback', { secs: -5 }).catch(() => {});
    } else if (e.code === 'ArrowRight') {
      e.preventDefault();
      invoke('seek_playback', { secs: 5 }).catch(() => {});
    } else if (e.code === 'ArrowUp') {
      e.preventDefault();
      const cur = Number((volumeSlider as any).value);
      const next = Math.min(100, cur + 5);
      (volumeSlider as any).value = next;
      invoke('set_volume', { vol: next / 100 }).catch(() => {});
    } else if (e.code === 'ArrowDown') {
      e.preventDefault();
      const cur = Number((volumeSlider as any).value);
      const next = Math.max(0, cur - 5);
      (volumeSlider as any).value = next;
      invoke('set_volume', { vol: next / 100 }).catch(() => {});
    }
  };
  document.addEventListener('keydown', keyHandler);

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

  // sleep timer dialog
  let sleepPoll: number | undefined;
  const minutesField = dialogContent.querySelector('#sleep-minutes') as HTMLInputElement;
  const setBtn = dialogContent.querySelector('#sleep-set') as HTMLButtonElement;
  const cancelBtn = dialogContent.querySelector('#sleep-cancel') as HTMLButtonElement;
  const presetBtns = dialogContent.querySelectorAll('button[data-preset]');

  timerBtn.addEventListener('click', async () => {
    try {
      const status = await invoke<{ enabled: boolean; remaining_secs: number }>('get_sleep_timer_status');
      if (status.enabled) {
        minutesField.value = String(Math.ceil(status.remaining_secs / 60));
      }
      dialog.show();
      dialogTitle.textContent = status.enabled ? t('nowPlaying.SleepTimerActive', { time: formatTime(status.remaining_secs) }) : t('nowPlaying.SleepTimer');
      if (sleepPoll) clearInterval(sleepPoll);
      sleepPoll = window.setInterval(async () => {
        try {
          const s = await invoke<{ enabled: boolean; remaining_secs: number }>('get_sleep_timer_status');
          if (s.enabled) {
            dialogTitle.textContent = `Sleep Timer (${formatTime(s.remaining_secs)})`;
          }
        } catch {}
      }, 1000);
    } catch {}
  });

  setBtn.addEventListener('click', async () => {
    const mins = Number(minutesField.value) || 0;
    try {
      await invoke('set_sleep_timer', { minutes: mins });
      dialog.close();
    } catch {}
  });
  cancelBtn.addEventListener('click', async () => {
    try {
      await invoke('cancel_sleep_timer');
      dialog.close();
    } catch {}
  });
  presetBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      minutesField.value = btn.getAttribute('data-preset') || '30';
    });
  });

  // lyrics pane toggle
  let lyricsOpen = false;
  lyricsBtn.addEventListener('click', () => {
    lyricsOpen = !lyricsOpen;
    lyricsWrap.style.display = lyricsOpen ? 'flex' : 'none';
    lyricsBtn.toggleAttribute('selected', lyricsOpen);
  });

  // poll
  setInterval(updateState, 500);
  updateState();

  return container;
}

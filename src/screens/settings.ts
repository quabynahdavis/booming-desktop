import { invoke } from "@tauri-apps/api/core";

export function settingsScreen() {
  const container = document.createElement('div');
  container.className = 'screen-container';
  const header = document.createElement('div');
  header.className = 'screen-header';
  const h1 = document.createElement('h1');
  h1.textContent = 'Settings';
  header.appendChild(h1);
  container.appendChild(header);

  const list = document.createElement('md-list') as any;
  list.className = 'settings-list';
  container.appendChild(list);

  // Theme
  const themeItem = document.createElement('md-list-item') as any;
  const themeHeadline = document.createElement('div');
  themeHeadline.slot = 'headline';
  themeHeadline.textContent = 'Theme';
  themeItem.appendChild(themeHeadline);
  const themeSet = document.createElement('md-outlined-segmented-button-set') as any;
  ['system','light','dark'].forEach((t) => {
    const btn = document.createElement('md-outlined-segmented-button') as any;
    btn.label = t;
    if (t === 'system') btn.selected = true;
    btn.addEventListener('click', () => {
      localStorage.setItem('theme', t);
    });
    themeSet.appendChild(btn);
  });
  themeItem.appendChild(themeSet);
  list.appendChild(themeItem);

  // ReplayGain
  const rgItem = document.createElement('md-list-item') as any;
  const rgHeadline = document.createElement('div');
  rgHeadline.slot = 'headline';
  rgHeadline.textContent = 'ReplayGain mode';
  rgItem.appendChild(rgHeadline);
  const rgSet = document.createElement('md-outlined-segmented-button-set') as any;
  ['off','track','album'].forEach((m) => {
    const btn = document.createElement('md-outlined-segmented-button') as any;
    btn.label = m;
    if (m === 'off') btn.selected = true;
    btn.addEventListener('click', async () => {
      try { await invoke('set_replay_gain_mode', { mode: m }); } catch {}
      localStorage.setItem('replaygain', m);
    });
    rgSet.appendChild(btn);
  });
  rgItem.appendChild(rgSet);
  list.appendChild(rgItem);

  // Gapless
  const gapItem = document.createElement('md-list-item') as any;
  const gapHeadline = document.createElement('div');
  gapHeadline.slot = 'headline';
  gapHeadline.textContent = 'Gapless playback';
  gapItem.appendChild(gapHeadline);
  const gapSwitch = document.createElement('md-switch') as any;
  gapSwitch.checked = false;
  gapItem.appendChild(gapSwitch);
  list.appendChild(gapItem);

  // Library folders
  const folderItem = document.createElement('md-list-item') as any;
  const folderHeadline = document.createElement('div');
  folderHeadline.slot = 'headline';
  folderHeadline.textContent = 'Library folders';
  folderItem.appendChild(folderHeadline);
  const addBtn = document.createElement('md-icon-button') as any;
  addBtn.setAttribute('aria-label', 'Add folder');
  addBtn.textContent = '+';
  addBtn.addEventListener('click', async () => {
    try { await invoke('scan_start', { root: '/tmp/music' }); } catch {}
  });
  folderItem.appendChild(addBtn);
  list.appendChild(folderItem);

  // Rescan button
  const rescanBtn = document.createElement('md-filled-tonal-button');
  rescanBtn.className = 'settings-rescan';
  rescanBtn.textContent = 'Rescan library';
  rescanBtn.addEventListener('click', async () => {
    try { await invoke('scan_start', { root: '/tmp/music' }); } catch {}
  });
  container.appendChild(rescanBtn);

  // Scrobble settings
  const scrobbleItem = document.createElement('md-list-item') as any;
  const scrobbleHeadline = document.createElement('div');
  scrobbleHeadline.slot = 'headline';
  scrobbleHeadline.textContent = 'Scrobble';
  scrobbleItem.appendChild(scrobbleHeadline);

  const scrobbleSet = document.createElement('md-outlined-segmented-button-set') as any;
  ['off','listenbrainz','lastfm'].forEach((s) => {
    const btn = document.createElement('md-outlined-segmented-button') as any;
    btn.label = s;
    if (s === 'off') btn.selected = true;
    btn.addEventListener('click', async () => {
      await invoke('set_scrobble_source', { source: s });
    });
    scrobbleSet.appendChild(btn);
  });
  scrobbleItem.appendChild(scrobbleSet);

  // Scrobble enabled toggle
  const enabledItem = document.createElement('md-list-item') as any;
  const enabledHeadline = document.createElement('div');
  enabledHeadline.slot = 'headline';
  enabledHeadline.textContent = 'Enable scrobble';
  enabledItem.appendChild(enabledHeadline);
  const enabledSwitch = document.createElement('md-switch') as any;
  enabledSwitch.checked = false;
  enabledItem.appendChild(enabledSwitch);

  // ListenBrainz token
  const lbItem = document.createElement('md-list-item') as any;
  const lbHeadline = document.createElement('div');
  lbHeadline.slot = 'headline';
  lbHeadline.textContent = 'ListenBrainz token';
  lbItem.appendChild(lbHeadline);
  const lbInput = document.createElement('input') as any;
  lbInput.type = 'password';
  lbInput.placeholder = 'Optional token';
  lbItem.appendChild(lbInput);

  // Last.fm token
  const lfmItem = document.createElement('md-list-item') as any;
  const lfmHeadline = document.createElement('div');
  lfmHeadline.slot = 'headline';
  lfmHeadline.textContent = 'Last.fm token';
  lfmItem.appendChild(lfmHeadline);
  const lfmInput = document.createElement('input') as any;
  lfmInput.type = 'password';
  lfmInput.placeholder = 'Optional token';
  lfmItem.appendChild(lfmInput);

  // Save button
  const saveBtn = document.createElement('md-filled-tonal-button');
  saveBtn.textContent = 'Save';
  saveBtn.addEventListener('click', async () => {
    const newEnabled = enabledSwitch.checked;
    const newSource = scrobbleSet.selectedOptions?.[0]?.label ?? 'off';
    await invoke('set_scrobble_enabled', { enabled: newEnabled });
    await invoke('set_scrobble_source', { source: newSource });
    await invoke('set_listenbrainz_token', { token: lbInput.value });
    await invoke('set_lastfm_token', { token: lfmInput.value });
  });
  container.appendChild(saveBtn);

  return container;
}

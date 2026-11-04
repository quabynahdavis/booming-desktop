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
  rescanBtn.textContent = 'Rescan library';
  rescanBtn.addEventListener('click', async () => {
    try { await invoke('scan_start', { root: '/tmp/music' }); } catch {}
  });
  container.appendChild(rescanBtn);

  return container;
}

export function songsScreen() {
  const container = document.createElement('div');
  container.className = 'screen-container';
  const header = document.createElement('div');
  header.className = 'screen-header';
  const h1 = document.createElement('h1');
  h1.textContent = 'Songs';
  header.appendChild(h1);
  container.appendChild(header);
  const list = document.createElement('div');
  list.id = 'song-list';
  container.appendChild(list);
  return container;
}

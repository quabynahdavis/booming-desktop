export function screenContainer(title: string, content: HTMLElement): HTMLElement {
  const wrap = document.createElement('div');
  wrap.className = 'screen-container';
  const header = document.createElement('div');
  header.className = 'screen-header';
  const h1 = document.createElement('h1');
  h1.textContent = title;
  header.appendChild(h1);
  wrap.appendChild(header);
  wrap.appendChild(content);
  return wrap;
}

export function grid(): HTMLElement {
  const el = document.createElement('div');
  el.className = 'md-grid';
  return el;
}

export function card(thumbnail: string | HTMLElement, title: string, subtitle?: string): HTMLElement {
  const cardEl = document.createElement('div');
  cardEl.className = 'md-card-stub';
  const thumb = document.createElement('div');
  thumb.className = 'card-thumbnail';
  if (typeof thumbnail === 'string') {
    const img = document.createElement('img');
    img.src = thumbnail;
    thumb.appendChild(img);
  } else {
    thumb.appendChild(thumbnail);
  }
  const content = document.createElement('div');
  content.className = 'card-content';
  const t = document.createElement('div');
  t.className = 'card-title';
  t.textContent = title;
  content.appendChild(t);
  if (subtitle) {
    const s = document.createElement('div');
    s.className = 'card-subtitle';
    s.textContent = subtitle;
    content.appendChild(s);
  }
  cardEl.appendChild(thumb);
  cardEl.appendChild(content);
  return cardEl;
}

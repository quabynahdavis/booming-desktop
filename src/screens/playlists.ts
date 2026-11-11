import { t } from "../i18n/index.js";

export function playlistsScreen() {
  const container = screenContainer(t('screen.playlists'), document.createElement('div'));
  return container;
}

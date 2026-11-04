import { screenContainer } from "./common.js";

export function recentlyPlayedScreen() {
  const container = screenContainer('Recently Played', document.createElement('div'));
  return container;
}

/**
 * Minimal vanilla TypeScript router.
 * No framework, ~50 lines. Swaps DOM fragments for each route.
 */

export type RouteName =
  | "songs"
  | "albums"
  | "artists"
  | "playlists"
  | "folders"
  | "recently-played"
  | "most-played"
  | "settings"
  | "now-playing"
  | "album-detail"
  | "artist-detail"
  | "folder-detail"
  | "genre-detail";

export interface RouteParams {
  id?: string | number;
  name?: string;
}

export interface Route {
  name: RouteName;
  params: RouteParams;
}

type ScreenRenderer = (params: RouteParams) => HTMLElement | DocumentFragment;

const screenRegistry = new Map<RouteName, ScreenRenderer>();
let currentRoute: Route = { name: "songs", params: {} };
let currentElement: HTMLElement | null = null;
const listeners: Array<(route: Route) => void> = [];

export const Router = {
  /**
   * Register a screen renderer for a route name.
   */
  register(name: RouteName, renderer: ScreenRenderer) {
    screenRegistry.set(name, renderer);
  },

  /**
   * Navigate to a route. Updates history and swaps the main content.
   */
  navigate(name: RouteName, params: RouteParams = {}) {
    const route: Route = { name, params };
    currentRoute = route;
    window.history.pushState(route, "", `#${name}${params.id ? `/${params.id}` : ""}`);
    Router.render();
    listeners.forEach((fn) => fn(route));
  },

  /**
   * Replace current route without adding history entry.
   */
  replace(name: RouteName, params: RouteParams = {}) {
    const route: Route = { name, params };
    currentRoute = route;
    window.history.replaceState(route, "", `#${name}${params.id ? `/${params.id}` : ""}`);
    Router.render();
    listeners.forEach((fn) => fn(route));
  },

  /**
   * Get current route.
   */
  getCurrent(): Route {
    return currentRoute;
  },

  /**
   * Subscribe to route changes.
   */
  onRouteChange(fn: (route: Route) => void) {
    listeners.push(fn);
    return () => {
      const idx = listeners.indexOf(fn);
      if (idx >= 0) listeners.splice(idx, 1);
    };
  },

  /**
   * Render the current route's screen into the main content element.
   */
  render() {
    const renderer = screenRegistry.get(currentRoute.name);
    if (!renderer) {
      console.warn(`No screen registered for route: ${currentRoute.name}`);
      return;
    }
    const main = document.querySelector<HTMLElement>("main#app");
    if (!main) return;
    const fragment = renderer(currentRoute.params);
    if (currentElement) {
      main.replaceChild(fragment, currentElement);
    } else {
      main.appendChild(fragment);
    }
    currentElement = fragment instanceof HTMLElement ? fragment : main.lastElementChild as HTMLElement;
  },

  /**
   * Initialize router from current hash and wire navigation drawer.
   */
  init() {
    // Wire nav drawer clicks
    const navItems = document.querySelectorAll<HTMLElement>("#nav-drawer [data-route]");
    navItems.forEach((item) => {
      item.addEventListener("click", () => {
        const route = item.getAttribute("data-route") as RouteName;
        if (route) {
          Router.navigate(route);
        }
      });
    });

    // Update active nav selection on route change
    Router.onRouteChange((route) => {
      navItems.forEach((item) => {
        if (item.getAttribute("data-route") === route.name) {
          item.setAttribute("selected", "");
        } else {
          item.removeAttribute("selected");
        }
      });
    });

    window.addEventListener("popstate", (e) => {
      if (e.state && typeof e.state === "object" && "name" in e.state) {
        currentRoute = e.state as Route;
        Router.render();
        listeners.forEach((fn) => fn(currentRoute));
      } else {
        // Fallback: parse hash
        const hash = window.location.hash.slice(1);
        const [name, ...rest] = hash.split("/");
        const params: RouteParams = {};
        if (rest.length > 0) params.id = rest.join("/");
        currentRoute = { name: (name || "songs") as RouteName, params };
        Router.render();
        listeners.forEach((fn) => fn(currentRoute));
      }
    });

    // Initial render from hash or default
    const hash = window.location.hash.slice(1);
    if (hash) {
      const [name, ...rest] = hash.split("/");
      const params: RouteParams = {};
      if (rest.length > 0) params.id = rest.join("/");
      currentRoute = { name: name as RouteName, params };
    }
    Router.render();
    listeners.forEach((fn) => fn(currentRoute));
  },
};
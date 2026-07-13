import type { Page, WebSocketRoute } from '@playwright/test';
import { CDN_HOST_SUFFIXES, WS_ROUTE_GLOB } from './env';

/**
 * Network fault-injection + hygiene helpers.
 *
 * The flagship reconnect flow needs to independently fail/restore the two backend
 * protocols (REST + WS) on :8090 without touching the :8080 app host — hence route()
 * on `**\/api\/**` and routeWebSocket() on `**\/push\/**`, never context.setOffline
 * (which also kills asset loading from :8080; see PLAYWRIGHT_E2E_PLAN.md).
 */

/** Abort external CDN/analytics requests (Sentry, Fathom, Google Fonts) — inert on local, flaky otherwise. */
export async function blockCdns(page: Page): Promise<void> {
  await page.route(
    (url) => CDN_HOST_SUFFIXES.some((h) => url.hostname === h || url.hostname.endsWith(`.${h}`)),
    (route) => route.abort(),
  );
}

/**
 * Clear localStorage/sessionStorage for the current origin. LocalCache persists per-event
 * like-state + mod UI keyed by event id, so a revisited event can render pre-liked.
 * Must be called with a document already loaded (localStorage needs an origin).
 * NOTE: each Playwright test gets a fresh context already; use this only for intra-test resets.
 */
export async function clearStorage(page: Page): Promise<void> {
  await page.evaluate(() => {
    try {
      window.localStorage.clear();
      window.sessionStorage.clear();
    } catch {
      /* private-mode / not-yet-loaded: ignore */
    }
  });
}

/** Abort every backend REST call → initial GET /api/event/:id fails → Fetched(None) → NotFound. */
export async function abortApi(page: Page): Promise<void> {
  await page.route('**/api/**', (route) => route.abort());
}

/** Release the REST fault so the Connected-triggered refetch reaches the live server. */
export async function restoreApi(page: Page): Promise<void> {
  await page.unroute('**/api/**');
}

/** Mutable gate for the WS route. */
export interface SocketGate {
  /** While true, every NEW socket the app opens is closed immediately (held offline). */
  down: boolean;
  /**
   * Close the currently-established socket to simulate a MID-SESSION drop. The app then re-arms its
   * 4s reconnect (socket.rs) and re-creates a socket, which obeys `down`. Use this for the warm
   * "already-loaded → drop → recover" path: `context.setOffline` does NOT tear down an already-open
   * WebSocket (it only gates new connections), so it can't drop a live socket — this can.
   */
  dropCurrent(): void;
}

/**
 * Intercept the app's WebSocket (`ws://.../push/:id`). While `gate.down`, each new socket
 * (the app re-creates one every 4s — socket.rs) is closed immediately, keeping the app
 * offline. Once `gate.down` is false, attempts proxy to the real backend and recovery fires.
 * `gate.dropCurrent()` closes the live socket for mid-session drop tests.
 *
 * Returns the gate so a test can toggle it mid-run. `down` defaults to false (transparent proxy).
 */
export async function gateWebSocket(page: Page, down = false): Promise<SocketGate> {
  let current: WebSocketRoute | undefined;
  const gate: SocketGate = {
    down,
    dropCurrent() {
      current?.close();
    },
  };
  await page.routeWebSocket(WS_ROUTE_GLOB, (ws) => {
    current = ws;
    if (gate.down) {
      ws.close();
    } else {
      ws.connectToServer();
    }
  });
  return gate;
}

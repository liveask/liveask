/**
 * Shared environment constants for the harness.
 *
 * All values reflect the LA_ENV=local build baked into the WASM bundle
 * (frontend/src/pages/event.rs `la_endpoints()`), so they must stay in sync with it.
 */

/** Frontend app host (Trunk). Same as playwright.config `baseURL`. */
export const FRONTEND_URL = 'http://127.0.0.1:8080';

/** Backend REST + WS host baked into the local WASM (BASE_API_LOCAL / BASE_SOCKET_LOCAL). */
export const BACKEND_URL = 'http://localhost:8090';
export const SOCKET_URL = 'ws://localhost:8090';

/** Readiness probe — `GET /api/ping` returns the literal body `pong` once Redis + DDB are connected. */
export const PING_URL = `${BACKEND_URL}/api/ping`;

/** The app opens `ws://localhost:8090/push/{event_id}` — glob for page.routeWebSocket. */
export const WS_ROUTE_GLOB = '**/push/**';

/**
 * External hosts loaded by index.html (Google Fonts, Sentry SDK + ingest, Fathom).
 * Inert on local; block them for hermetic/offline runs. Matched by hostname suffix.
 */
export const CDN_HOST_SUFFIXES = [
  'fonts.googleapis.com',
  'fonts.gstatic.com',
  'browser.sentry-cdn.com',
  'ingest.sentry.io',
  'cdn.usefathom.com',
] as const;

/**
 * Admin login. The FE hashes the typed password client-side as
 * base64_nopad(sha256(password)) and POSTs that as `pwd_hash`; the backend accepts
 * iff it equals env LA_ADMIN_PWD_HASH. So for BROWSER login the server must run with
 * ADMIN_PWD_HASH = the real hash of ADMIN_PASSWORD (below).
 *
 * NOTE: backend-e2e/justfile `serve` uses the literal "e2e-controlled-admin-hash",
 * which only works for reqwest tests that POST the hash directly — NOT for browser login.
 * fixtures/backend.ts therefore boots with ADMIN_PWD_HASH below by default.
 */
export const ADMIN_USERNAME = 'admin';
export const ADMIN_PASSWORD = 'pwd';
/** base64_nopad(sha256("pwd")). */
export const ADMIN_PWD_HASH = 'oRWenfNnDVSdBFJFMmKfVHfOt97sm0XkfowAlQbsssg';

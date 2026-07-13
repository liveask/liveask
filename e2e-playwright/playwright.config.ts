import { defineConfig, devices } from '@playwright/test';

/**
 * LiveAsk browser E2E.
 *
 * Topology (see PLAYWRIGHT_E2E_PLAN.md):
 *   - Frontend WASM bundle served by Trunk at http://127.0.0.1:8080 (managed by `webServer` below).
 *   - Backend (liveask-server + Redis + DynamoDB-local) at http://localhost:8090, booted SEPARATELY
 *     (locally: `cd backend && just docker-compose` + `cd backend-e2e && just serve`; in CI: a job step).
 *     It is intentionally NOT a Playwright `webServer` — the reconnect suite kills/restarts :8090, so its
 *     lifecycle is owned by fixtures/backend.ts, not this config.
 *
 * The bundle bakes in LA_ENV=local endpoints at build time, so `cargo make serve` (which sets
 * LA_ENV=local + RUSTFLAGS) is the only supported way to produce the bundle under test.
 */
export default defineConfig({
  testDir: './tests',
  // Verifies the backend (:8090 + Redis + DDB) is up before the suite; the FE host is the webServer below.
  globalSetup: './globalSetup.ts',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : [['list'], ['html', { open: 'never' }]],

  timeout: 60_000,
  expect: {
    // WASM first-paint after mount can be slow; reconnect assertions still set their own >=8s timeouts.
    timeout: 10_000,
  },

  use: {
    baseURL: 'http://127.0.0.1:8080',
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    // Copy-link / share assertions need clipboard access.
    permissions: ['clipboard-read', 'clipboard-write'],
    navigationTimeout: 30_000,
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    // Cross-browser (firefox/webkit) is a nightly-only matrix; add projects there, not on the PR gate.
  ],

  webServer: {
    // Local default: `cargo make serve` => `trunk serve --no-autoreload` on :8080 (LA_ENV=local, RUSTFLAGS auto-set).
    // In Docker there's no cargo/trunk at runtime — the image sets E2E_WEBSERVER_CMD to serve the
    // prebuilt dist statically (see Dockerfile / docker-compose.e2e.yml).
    command: process.env.E2E_WEBSERVER_CMD ?? 'cargo make serve',
    cwd: process.env.E2E_WEBSERVER_CWD ?? '../frontend',
    url: 'http://127.0.0.1:8080',
    // First run compiles the WASM bundle — allow a long cold-build window.
    timeout: 300_000,
    reuseExistingServer: !process.env.CI,
    stdout: 'pipe',
    stderr: 'pipe',
  },
});

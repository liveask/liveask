import { expect, test } from '@playwright/test';
import { BackendServer } from '../fixtures/backend';
import { createEvent, routes } from '../fixtures/event';
import { abortApi, blockCdns, gateWebSocket, restoreApi } from '../helpers/net';
import { LOAD_STATE, TID } from '../helpers/selectors';

/**
 * Flagship reconnect/refetch — the bug this whole harness exists for: opening `/event/:id` while the
 * backend was DOWN showed "event not found" *permanently*; the fix always re-arms the 4s socket
 * reconnect and re-fetches the event on reconnect. That transition (fetch fails → NotFound → socket
 * reconnects → Connected re-issues the fetch → Loaded) is invisible to unit tests and backend-e2e.
 * See PLAYWRIGHT_E2E_PLAN.md "Flagship reconnect/refetch spec".
 *
 * Timing: the socket reconnects on a fixed 4s interval (socket.rs `set_reconnect`) and pings every
 * 3s, so recovery can land one or two ticks out — reconnect assertions use timeouts >= 8s and never
 * assert within a single interval. Connection state is read off `topbar[data-connected]` (a
 * deterministic hook) rather than `offline-indicator` visibility: the indicator's "hidden" class is
 * not `display:none`, so toBeHidden() on it is unreliable.
 */

test('flagship: down at initial load → NotFound + offline, recovers on reconnect (route interception)', async ({
  page,
  request,
}) => {
  // test:false → no TTL, so the event survives the whole down window (test:true events get a 60s TTL).
  const event = await createEvent(request);
  await blockCdns(page);

  // DOWN: fail REST (initial GET /api/event/:id → Fetched(None) → NotFound) and hold every reconnect
  // socket closed. The app re-creates a socket every 4s; while `down` each one is closed immediately.
  await abortApi(page);
  const gate = await gateWebSocket(page, /* down */ true);

  await page.goto(routes.event(event.id));

  // (1) the exact bug symptom + offline affordance during the down window.
  await expect(page.getByTestId(TID.eventLoadstate)).toHaveAttribute('data-state', LOAD_STATE.notfound);
  await expect(page.getByText('event not found')).toBeVisible();
  await expect(page.getByTestId(TID.topbar)).toHaveAttribute('data-connected', 'false', { timeout: 8_000 });
  // The offline bar is "shown" by DROPPING its `hidden` class (which only slides it off-screen via
  // `top`, so toBeVisible() on the container is vacuous — it stays a non-zero box either way); assert
  // the class instead, and that its icon (real HTML `hidden` attr) is actually rendered.
  await expect(page.getByTestId(TID.offlineIndicator)).not.toHaveClass(/hidden/);
  await expect(page.getByTestId(TID.offlineIndicator).locator('img')).toBeVisible();

  // UP: release both faults; the next reconnect tick fires Connected → refetch (state is NotFound) → Loaded.
  gate.down = false;
  await restoreApi(page);

  // (2) topbar reconnects, the offline bar re-hides, the event renders, and the noevent banner is gone.
  await expect(page.getByTestId(TID.topbar)).toHaveAttribute('data-connected', 'true', { timeout: 12_000 });
  await expect(page.getByTestId(TID.offlineIndicator)).toHaveClass(/hidden/);
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible({ timeout: 12_000 });
  await expect(page.getByTestId(TID.eventName)).toHaveText(event.name);
  await expect(page.getByTestId(TID.eventLoadstate)).toHaveCount(0);
});

test('warm reconnect: mid-session socket drop → offline bar → recover on next tick', async ({ page, request }) => {
  const event = await createEvent(request);
  await blockCdns(page);

  // Proxy the socket transparently and load fully first (this is the already-loaded secondary path;
  // the cold "down at initial load" case is the flagship above).
  const gate = await gateWebSocket(page, /* down */ false);
  await page.goto(routes.event(event.id));
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  await expect(page.getByTestId(TID.eventName)).toHaveText(event.name);

  // Drop the LIVE socket mid-session, holding reconnect attempts closed so the offline state is
  // stable to assert. (context.setOffline can't do this — it doesn't tear down an already-open WS.)
  gate.down = true;
  gate.dropCurrent();
  await expect(page.getByTestId(TID.topbar)).toHaveAttribute('data-connected', 'false', { timeout: 12_000 });
  // Offline bar shown = its `hidden` class removed (see flagship note on why toBeVisible is vacuous here).
  await expect(page.getByTestId(TID.offlineIndicator)).not.toHaveClass(/hidden/);
  await expect(page.getByTestId(TID.offlineIndicator).locator('img')).toBeVisible();

  // Restore: the next reconnect tick proxies to the live server again. The event stays Loaded
  // throughout (no NotFound flip, unlike the cold case).
  gate.down = false;
  await expect(page.getByTestId(TID.topbar)).toHaveAttribute('data-connected', 'true', { timeout: 12_000 });
  await expect(page.getByTestId(TID.offlineIndicator)).toHaveClass(/hidden/);
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  await expect(page.getByTestId(TID.eventName)).toHaveText(event.name);
});

/**
 * Fallback / nightly canary — the highest-fidelity reconnect: SIGKILL the real liveask-server, load
 * the page (HTTP + WS both refused), then relaunch it and assert the browser recovers on a later
 * reconnect tick. Redis + DynamoDB stay up so the event persists across the restart.
 *
 * It needs to OWN :8090 (you can't SIGKILL a server you didn't spawn), which conflicts with the
 * default topology where the backend is booted externally. So it is opt-in: defined only when
 * E2E_RECONNECT_CANARY=1, run in isolation with only redis+dynamodb up (no external liveask-server),
 * serially, and gated to nightly rather than the PR smoke. When enabled, globalSetup skips its
 * up-front backend check because this suite boots the server itself.
 */
if (process.env.E2E_RECONNECT_CANARY === '1') {
  test.describe('flagship canary: real backend restart (SIGKILL → relaunch)', () => {
    test.describe.configure({ mode: 'serial' });

    const backend = new BackendServer();
    let ownsBackend = false;

    test.beforeAll(async () => {
      if (await BackendServer.isUp()) {
        // A server we didn't spawn is already on :8090 — we can't SIGKILL it. Leave ownsBackend=false
        // so the test self-skips rather than pretending to restart something it doesn't control.
        return;
      }
      backend.start();
      await backend.waitForPing();
      ownsBackend = true;
    });

    test.afterAll(() => {
      if (ownsBackend) backend.stop();
    });

    test('event page recovers after a real backend restart', async ({ page, request }) => {
      test.skip(
        !ownsBackend,
        'canary must own :8090 — boot only redis+dynamodb (no external liveask-server), then E2E_RECONNECT_CANARY=1',
      );

      const event = await createEvent(request);
      await blockCdns(page);

      backend.stop(); // SIGKILL — abrupt, matches the bug (HTTP + WS both refused)
      await page.goto(routes.event(event.id));
      await expect(page.getByText('event not found')).toBeVisible({ timeout: 12_000 });

      backend.start(); // relaunch; Redis+DDB stayed up, so the `liveask` table + event persist
      await backend.waitForPing();

      // Recovery lands on a reconnect tick AFTER the server finishes rebinding, which can exceed one
      // 4s interval — use a wide timeout.
      await expect(page.getByTestId(TID.eventName)).toHaveText(event.name, { timeout: 20_000 });
      await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
    });
  });
}

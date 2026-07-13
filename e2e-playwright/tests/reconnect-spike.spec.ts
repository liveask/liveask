import { expect, test } from '@playwright/test';
import { createEvent, routes } from '../fixtures/event';
import { abortApi, blockCdns, gateWebSocket, restoreApi } from '../helpers/net';
import { TID } from '../helpers/selectors';

/**
 * Validates the reconnect fault-injection technique the realtime/reconnect specs depend on.
 *
 * Two things must hold for `page.routeWebSocket` to be viable against the app's
 * `wasm_sockets::EventClient` socket (`ws://.../push/:id`):
 *   1. The handler actually fires (the WASM socket IS interceptable).
 *   2. `ws.connectToServer()` transparently proxies so the app behaves normally when we allow it,
 *      and `ws.close()` holds it offline when we don't — enabling deterministic down→up control.
 *
 * If either assertion fails, fall back to the real process-kill technique (fixtures/backend.ts)
 * or a toxiproxy toggle.
 *
 * Requires the backend up (globalSetup enforces it).
 */

test('spike: routeWebSocket intercepts the wasm_sockets push socket (transparent proxy)', async ({
  page,
  request,
}) => {
  const event = await createEvent(request);
  await blockCdns(page);

  let wsHandlerHits = 0;
  await page.routeWebSocket('**/push/**', (ws) => {
    wsHandlerHits += 1;
    ws.connectToServer(); // transparent proxy to the real :8090
  });

  await page.goto(routes.event(event.id));

  // App loads through the proxied socket → interception did not break the realtime path.
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  await expect(page.getByTestId(TID.eventName)).toHaveText(event.name);

  // The push socket was intercepted at least once (the whole premise of the flagship test).
  expect(wsHandlerHits, 'routeWebSocket("**/push/**") never fired — wasm_sockets not interceptable').toBeGreaterThan(0);
});

test('spike: down-at-load → NotFound + offline, then recovers on reconnect tick', async ({
  page,
  request,
}) => {
  const event = await createEvent(request);
  await blockCdns(page);

  // DOWN: fail REST (→ Fetched(None) → NotFound) and hold every reconnect socket closed.
  await abortApi(page);
  const gate = await gateWebSocket(page, /* down */ true);

  await page.goto(routes.event(event.id));

  // Exact bug symptom + offline affordance during the down window.
  await expect(page.getByTestId(TID.eventLoadstate)).toHaveAttribute('data-state', 'notfound');
  await expect(page.getByText('event not found')).toBeVisible();
  await expect(page.getByTestId(TID.offlineIndicator)).toBeVisible();
  // Deterministic connection signal (the #ico-offline .hidden class is NOT display:none,
  // so toBeHidden() on it is unreliable — assert the topbar data-connected attribute instead).
  await expect(page.getByTestId(TID.topbar)).toHaveAttribute('data-connected', 'false', { timeout: 8_000 });

  // UP: release both faults; the next 4s reconnect tick fires Connected → refetch → Loaded.
  gate.down = false;
  await restoreApi(page);

  // Reconnect interval (4s) + ping (3s) + fetch → allow up to two ticks.
  await expect(page.getByTestId(TID.topbar)).toHaveAttribute('data-connected', 'true', { timeout: 12_000 });
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible({ timeout: 12_000 });
  await expect(page.getByTestId(TID.eventName)).toHaveText(event.name);
  await expect(page.getByTestId(TID.eventLoadstate)).toHaveCount(0); // noevent banner gone
});

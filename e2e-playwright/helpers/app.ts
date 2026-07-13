import { type Browser, type BrowserContext, type Page, expect } from '@playwright/test';
import { blockCdns } from './net';
import { TID } from './selectors';

/**
 * Shared helpers for driving the real WASM app across contexts.
 */

/**
 * Dismiss the ColorPopup a moderator's first visit to an event auto-opens (event_meta.rs `create()`,
 * gated by a per-event LocalCache flag a fresh context never has). Its full-screen `.popup-bg`
 * intercepts pointer events, so mod controls aren't clickable until it's dismissed — click outside the
 * centered content to trigger the Popup's outside-click close. No-op if no popup is open.
 */
export async function dismissModColorPopup(page: Page): Promise<void> {
  const bg = page.locator('.popup-bg');
  if ((await bg.count()) > 0) {
    await bg.first().click({ position: { x: 5, y: 5 } });
    await expect(bg).toHaveCount(0);
  }
}

/**
 * Open an event `url` in a fresh context (tracked in `track` for teardown), wait until the event is
 * Loaded, dismiss the mod color popup (moderator routes only), and wait until this client's push
 * socket has received a frame — which proves it's subscribed and will receive `q:`/`e` broadcasts (the
 * server sends a viewer-count frame to each socket on its own connect, app.rs `push_subscriber`).
 * Gating on subscription removes the race where an observer misses a fire-and-forget broadcast.
 *
 * For event pages only (a page with no `/push/` socket, e.g. /login, would hang on `subscribed`).
 * The color-popup dismissal is gated to `/eventmod/` on purpose: a viewer's password popup is also a
 * `.popup-bg`, and clicking it away would defeat the password specs.
 */
export async function openLoaded(browser: Browser, url: string, track: BrowserContext[]): Promise<Page> {
  const context = await browser.newContext();
  track.push(context);
  const page = await context.newPage();
  await blockCdns(page);

  const subscribed = new Promise<void>((resolve) => {
    page.on('websocket', (ws) => {
      if (ws.url().includes('/push/')) ws.once('framereceived', () => resolve());
    });
  });

  await page.goto(url);
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  if (url.includes('/eventmod/')) await dismissModColorPopup(page);
  await subscribed;

  return page;
}

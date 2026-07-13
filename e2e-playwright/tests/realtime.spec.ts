import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';
import { adminLogin, upgradeToPremium } from '../fixtures/admin';
import { addQuestion, createEvent, routes } from '../fixtures/event';
import { blockCdns } from '../helpers/net';
import { BUCKET, TID, bucketSelector } from '../helpers/selectors';

/**
 * Cross-client realtime reactivity: two independent BrowserContexts (a moderator on
 * /eventmod/:id/:secret and a viewer on /event/:id) driven by the real WebSocket fan-out. Every
 * mutation round-trips through the server, which broadcasts `q:`/`e`, and BOTH clients refetch and
 * re-render with NO page reload. Contract-level add/like/hide/answer stays owned by backend-e2e;
 * here we assert only the browser DOM reacting across clients.
 *
 * WS propagation + first WASM render can be slow → cross-context assertions use generous timeouts.
 */

// Each clears AddQuestionValidation (>=10 trimmed chars, >=3 words, each word <=30).
const Q_FROM_VIEWER = 'Will the realtime fan-out reach the moderator tab?';
const Q_SEED = 'A seeded question for the moderation bucket tests?';

// Contexts opened via openLoaded, closed after each test even if an assertion throws (manual
// browser.newContext() contexts are NOT auto-disposed by Playwright the way the default one is).
const opened: BrowserContext[] = [];
test.afterEach(async () => {
  await Promise.all(opened.splice(0).map((c) => c.close()));
});

/**
 * Moderators get the ColorPopup auto-opened on their first visit to an event (event_meta.rs
 * `create()`, gated by a per-event LocalCache flag that a fresh context never has). Its full-screen
 * `.popup-bg` intercepts pointer events, so mod controls aren't clickable until it's dismissed —
 * click outside the centered content to trigger the Popup's outside-click close. No-op for viewer
 * tabs (no auto-popup) and for already-dismissed state.
 */
async function dismissAutoPopup(page: Page): Promise<void> {
  const bg = page.locator('.popup-bg');
  if ((await bg.count()) > 0) {
    await bg.first().click({ position: { x: 5, y: 5 } });
    await expect(bg).toHaveCount(0);
  }
}

/**
 * Open `url` in a fresh context and wait until (a) the event is Loaded and (b) this client's push
 * socket has received a frame. The server broadcasts a viewer-count frame to each socket on its own
 * connect (app.rs `push_subscriber`: insert channel → notify), so a received frame proves THIS client
 * is subscribed and will get subsequent `q:`/`e` broadcasts. Gating on it removes the race where an
 * observer misses a fire-and-forget broadcast because its socket hadn't subscribed yet.
 * (topbar[data-connected] can't be used for this — IconBar initialises `connected: true`, so it's
 * already "true" before the socket ever connects.)
 */
async function openLoaded(browser: Browser, url: string): Promise<Page> {
  const context = await browser.newContext();
  opened.push(context);
  const page = await context.newPage();
  await blockCdns(page);

  // Latch the push socket's first received frame. Set up before goto so the connect frame can't slip
  // past — the listener is attached the instant the WS is created.
  const subscribed = new Promise<void>((resolve) => {
    page.on('websocket', (ws) => {
      if (ws.url().includes('/push/')) ws.once('framereceived', () => resolve());
    });
  });

  await page.goto(url);
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  await dismissAutoPopup(page);
  await subscribed;

  return page;
}

test('viewer asks → the question appears in the moderator tab without a reload', async ({ browser, request }) => {
  const event = await createEvent(request);

  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret));
  const viewer = await openLoaded(browser, routes.event(event.id));

  // Marker survives a WS-driven refetch but not a full reload → proves the mod tab didn't reload.
  await mod.evaluate(() => ((globalThis as Record<string, unknown>).__noReload = true));

  await viewer.getByTestId(TID.askButton).click();
  await viewer.getByTestId(TID.questionInput).fill(Q_FROM_VIEWER);
  await expect(viewer.getByTestId(TID.questionSubmit)).toBeEnabled();
  await viewer.getByTestId(TID.questionSubmit).click();

  // Server broadcasts `q:<id>` → the mod tab refetches and renders it under "Hot Questions".
  await expect(mod.getByTestId(TID.questionItem).filter({ hasText: Q_FROM_VIEWER })).toBeVisible({ timeout: 15_000 });
  await expect(mod.locator(bucketSelector(BUCKET.hot))).toBeVisible();
  expect(await mod.evaluate(() => (globalThis as Record<string, unknown>).__noReload === true)).toBe(true);
});

test('moderator answers → the question moves to the Answered bucket in the viewer tab', async ({
  browser,
  request,
}) => {
  const event = await createEvent(request);
  await addQuestion(request, event.id, Q_SEED);

  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret));
  const viewer = await openLoaded(browser, routes.event(event.id));

  // Both start with the sole question under "Hot Questions".
  await expect(viewer.locator(bucketSelector(BUCKET.hot))).toBeVisible();
  await expect(mod.getByTestId(TID.questionItem).filter({ hasText: Q_SEED })).toBeVisible();

  await mod.getByTestId(TID.questionAnswer).click();

  // The viewer's sole question moves buckets live: "Answered" appears, "Hot Questions" empties, and
  // the question is still visible (answered questions ARE sent to viewers, unlike hidden ones).
  await expect(viewer.locator(bucketSelector(BUCKET.answered))).toBeVisible({ timeout: 15_000 });
  await expect(viewer.locator(bucketSelector(BUCKET.hot))).toHaveCount(0);
  await expect(viewer.getByTestId(TID.questionItem).filter({ hasText: Q_SEED })).toBeVisible();
});

test('moderator hides → the question leaves the viewer tab (moderator keeps it in Hidden)', async ({
  browser,
  request,
}) => {
  const event = await createEvent(request);
  await addQuestion(request, event.id, Q_SEED);

  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret));
  const viewer = await openLoaded(browser, routes.event(event.id));

  await expect(viewer.getByTestId(TID.questionItem).filter({ hasText: Q_SEED })).toBeVisible();

  await mod.getByTestId(TID.questionHide).click();

  // The backend filters hidden questions out of the viewer fetch → it disappears for the viewer,
  // while the moderator sees the same question demoted to the "Hidden" bucket.
  await expect(mod.locator(bucketSelector(BUCKET.hidden))).toBeVisible({ timeout: 15_000 });
  await expect(mod.getByTestId(TID.questionItem).filter({ hasText: Q_SEED })).toBeVisible();
  await expect(viewer.getByTestId(TID.questionItem)).toHaveCount(0, { timeout: 15_000 });
  await expect(viewer.locator('.noquestions')).toBeVisible();
});

test('upvote in one tab → like count increments and the bubble wiggles in the other', async ({ browser, request }) => {
  const event = await createEvent(request);
  await addQuestion(request, event.id, Q_SEED); // backend seeds it with likes = 1

  const observer = await openLoaded(browser, routes.event(event.id));
  const voter = await openLoaded(browser, routes.event(event.id));

  // Single-question event → these locators are unambiguous.
  const observerQuestion = observer.getByTestId(TID.questionItem);
  const observerCount = observer.getByTestId(TID.questionLikeCount);
  await expect(observerCount).toHaveText('1');

  // The voter is a fresh context (LocalCache says "not liked") → clicking the anchor sends a like.
  await voter.getByTestId(TID.questionLike).click();

  // Server broadcasts `q:<id>` → observer refetches. `changed()` sees likes go 1→2, which sets the
  // transient (1s) `.wiggle` class and updates the count. Poll for wiggle from right after the click
  // so we don't miss its window, then assert the settled count.
  await expect(observerQuestion.locator('.bubble')).toHaveClass(/wiggle/, { timeout: 15_000 });
  await expect(observerCount).toHaveText('2');
});

test('premium event: viewer count reflects both connected clients across contexts', async ({ browser, request }) => {
  // Viewer count is premium-only in the DOM (view_stats) AND premium/admin-only in the fetch
  // (app.rs:364), so this needs the no-Stripe admin upgrade to surface at all. Skips cleanly if the
  // running backend was booted with a placeholder LA_ADMIN_PWD_HASH (then admin login can't succeed).
  test.skip(!(await adminLogin(request)), 'admin login unavailable (backend booted with a placeholder LA_ADMIN_PWD_HASH)');

  const event = await createEvent(request);
  expect(await upgradeToPremium(request, event.id, event.secret)).toBeTruthy();

  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret));
  await expect(mod.locator('.statistics')).toBeVisible(); // premium-only stats block

  const viewer = await openLoaded(browser, routes.event(event.id));

  // A `v:` push does NOT re-render the Event component (handle_socket returns false), so the DOM count
  // only refreshes on the next fetch. Force one by having the viewer ask a question; the mod's refetch
  // response carries the live viewer count (both sockets are connected → >= 2).
  await viewer.getByTestId(TID.askButton).click();
  await viewer.getByTestId(TID.questionInput).fill(Q_FROM_VIEWER);
  await expect(viewer.getByTestId(TID.questionSubmit)).toBeEnabled();
  await viewer.getByTestId(TID.questionSubmit).click();

  // First `.count` in the statistics block is the viewer count (order: viewers, questions, likes).
  const viewers = mod.locator('.statistics .count').first();
  await expect
    .poll(async () => Number((await viewers.textContent())?.trim() ?? '0'), { timeout: 15_000 })
    .toBeGreaterThanOrEqual(2);
});

import { expect, test } from '@playwright/test';
import { createEvent, routes } from '../fixtures/event';
import { blockCdns } from '../helpers/net';
import { TID } from '../helpers/selectors';

/**
 * Per-route DOM-rendering smoke suite: home navigation, the create-event UI journey, opening an
 * event as a viewer, asking a question (WS-driven, no reload), and the print + privacy pages.
 * Contract-level behaviour (add/get/like/state) stays owned by backend-e2e; this only asserts that
 * the real WASM app renders and its primary happy paths work in a browser.
 *
 * Events that only need to *exist* are provisioned via the API (fixtures/event.ts) so each test is
 * independent under parallel workers; the create journey is the one test that drives the UI form.
 */

// Clears AddQuestionValidation: >=10 trimmed chars, >=3 words, each word <=30.
const SMOKE_QUESTION = 'What is the smoke-test question of the day?';

test('home renders and routes to new-event + example', async ({ page }) => {
  await blockCdns(page);
  await page.goto(routes.home());

  await expect(page.getByTestId(TID.home)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Real-Time questions from your audience' })).toBeVisible();

  // "View Example" pushes /event/eventexample. That event is not seeded in a hermetic backend, so
  // assert the client-side route change only — not that it loads.
  await page.getByRole('button', { name: 'View Example' }).click();
  await expect(page).toHaveURL(/\/event\/eventexample$/);

  // Hero "Create your Event" routes to the new-event form.
  await page.goto(routes.home());
  await page.getByTestId(TID.homeCreateEvent).click();
  await expect(page).toHaveURL(/\/newevent$/);
  await expect(page.getByTestId(TID.neweventFinish)).toBeVisible();
});

test('create-event journey redirects to the moderator route', async ({ page }) => {
  await blockCdns(page);
  await page.goto(routes.newEvent());

  // Name 8-30 trimmed chars; description >=30 (CreateEventValidation).
  const name = `Smoke ${Date.now().toString(36)}`;
  await page.getByTestId(TID.neweventName).fill(name);
  await page
    .getByTestId(TID.neweventDesc)
    .fill('Smoke test event created by the Playwright suite to verify the create journey.');

  const finish = page.getByTestId(TID.neweventFinish);
  await expect(finish).toBeEnabled();
  await finish.click();

  // Lands on /eventmod/:id/:secret with the freshly created event rendered.
  await page.waitForURL(/\/eventmod\/[^/]+\/[^/]+$/);
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  await expect(page.getByTestId(TID.eventName)).toHaveText(name);
});

test('viewer opens an event and sees its name', async ({ page, request }) => {
  await blockCdns(page);
  const event = await createEvent(request);

  await page.goto(routes.event(event.id));
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();
  await expect(page.getByTestId(TID.eventName)).toHaveText(event.name);
});

test('viewer asks a question and it appears under Hot Questions without a reload', async ({ page, request }) => {
  await blockCdns(page);
  const event = await createEvent(request);

  await page.goto(routes.event(event.id));
  await expect(page.getByTestId(TID.eventLoaded)).toBeVisible();

  // Marker survives a WS-driven refetch but not a full page load — proves "no reload".
  await page.evaluate(() => ((globalThis as Record<string, unknown>).__noReload = true));

  await page.getByTestId(TID.askButton).click();
  await page.getByTestId(TID.questionInput).fill(SMOKE_QUESTION);

  const submit = page.getByTestId(TID.questionSubmit);
  await expect(submit).toBeEnabled();
  await submit.click();

  // Server broadcasts `q:<id>` → the app refetches and renders the question under "Hot Questions".
  const question = page.getByTestId(TID.questionItem).filter({ hasText: SMOKE_QUESTION });
  await expect(question).toBeVisible({ timeout: 15_000 });
  await expect(
    page.locator(`[data-testid="${TID.questionsBucket}"][data-bucket="Hot Questions"]`),
  ).toBeVisible();

  expect(await page.evaluate(() => (globalThis as Record<string, unknown>).__noReload === true)).toBe(true);
});

test('print and privacy pages render', async ({ page, request }) => {
  await blockCdns(page);
  const event = await createEvent(request);

  // Print view has no data-testid hooks — assert the printable event name.
  await page.goto(routes.print(event.id));
  await expect(page.locator('.event-name.printable')).toHaveText(event.name);

  await page.goto(routes.privacy());
  await expect(page.getByRole('heading', { name: /Privacy Policy for Live-Ask/i })).toBeVisible();
});

import { type BrowserContext, expect, test } from '@playwright/test';
import { addQuestion, createEvent, routes } from '../fixtures/event';
import { openLoaded } from '../helpers/app';
import { LOAD_STATE, TID } from '../helpers/selectors';

/**
 * Moderator-only surfaces: the event-state `<select>` (Open / VoteOnly / Closed) and its downstream
 * gating (the `.not-open` banners + whether a viewer can still ask/vote), and event deletion. The
 * hide/answer bucket moves are owned by realtime.spec.ts (cross-context, strictly stronger), so they
 * aren't re-tested here. State changes round-trip through the server (broadcast `e` → refetch), so
 * cross-tab assertions use generous timeouts.
 */

const opened: BrowserContext[] = [];
test.afterEach(async () => {
  await Promise.all(opened.splice(0).map((c) => c.close()));
});

const Q_SEED = 'A seeded question for the moderation state tests?';

test('moderator closes the event → closed banner, asking disabled, and reversible', async ({ browser, request }) => {
  const event = await createEvent(request);
  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret), opened);
  const viewer = await openLoaded(browser, routes.event(event.id), opened);

  // Open → the viewer can ask (in-page + topbar), no closed banner.
  await expect(viewer.getByTestId(TID.askButton)).toBeVisible();
  await expect(viewer.getByTestId(TID.askButtonTopbar)).toBeVisible();

  await mod.getByTestId(TID.modStateSelect).selectOption({ value: '2' }); // Closed

  // Both tabs get the closed banner; the viewer can no longer ask (in-page hidden, topbar removed).
  await expect(viewer.getByText('was closed by the moderator')).toBeVisible({ timeout: 15_000 });
  await expect(mod.getByText('was closed by the moderator')).toBeVisible({ timeout: 15_000 });
  await expect(viewer.getByTestId(TID.askButton)).toBeHidden();
  await expect(viewer.getByTestId(TID.askButtonTopbar)).toHaveCount(0);

  // Reversible: reopening clears the banner and restores asking (proves the assertions aren't sticky).
  await mod.getByTestId(TID.modStateSelect).selectOption({ value: '0' }); // Open
  await expect(viewer.getByText('was closed by the moderator')).toBeHidden({ timeout: 15_000 });
  await expect(viewer.getByTestId(TID.askButton)).toBeVisible();
});

test('moderator sets vote-only → asking disabled but voting still works', async ({ browser, request }) => {
  const event = await createEvent(request);
  await addQuestion(request, event.id, Q_SEED); // seeded with likes = 1
  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret), opened);
  const viewer = await openLoaded(browser, routes.event(event.id), opened);

  await mod.getByTestId(TID.modStateSelect).selectOption({ value: '1' }); // VoteOnly

  // Vote-only banner + asking disabled …
  await expect(viewer.getByText('vote-only by the moderator')).toBeVisible({ timeout: 15_000 });
  await expect(viewer.getByTestId(TID.askButton)).toBeHidden();
  await expect(viewer.getByTestId(TID.askButtonTopbar)).toHaveCount(0);

  // … but the question is still there and still votable (can_vote = !closed).
  const count = viewer.getByTestId(TID.questionLikeCount);
  await expect(count).toHaveText('1');
  await viewer.getByTestId(TID.questionLike).click();
  await expect(count).toHaveText('2', { timeout: 15_000 });
});

test('moderator deletes the event → redirected home and the event reads as deleted', async ({ browser, request }) => {
  const event = await createEvent(request);
  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret), opened);

  await mod.getByTestId(TID.modDelete).click();
  // Confirm in the DeletePopup (two .btn-yes divs labelled "yes"/"no"; exact text picks "yes").
  await mod.locator('.delete-popup').getByText('yes', { exact: true }).click();

  // The FE navigates Home after the delete request resolves.
  await expect(mod).toHaveURL(/\/$/);
  await expect(mod.getByTestId(TID.home)).toBeVisible();

  // And the event is really gone: a fresh viewer fetch now reports the deleted load-state.
  await mod.goto(routes.event(event.id));
  await expect(mod.getByTestId(TID.eventLoadstate)).toHaveAttribute('data-state', LOAD_STATE.deleted, {
    timeout: 15_000,
  });
});

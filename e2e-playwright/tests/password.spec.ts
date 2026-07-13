import { type BrowserContext, expect, test } from '@playwright/test';
import { createEvent, routes } from '../fixtures/event';
import { openLoaded } from '../helpers/app';
import { TID } from '../helpers/selectors';

/**
 * Password-protected events: a moderator enables a password (ModPassword), after which a viewer's
 * event is masked (`blurr`) behind a password popup until they enter the right password. Two distinct
 * error messages exist and the plan conflated them — verified against password_popup.rs:
 *   - "invalid password": CLIENT-side format validation (empty/whitespace → trimmed len < 1);
 *   - "try again": SERVER rejection of a valid-format but wrong password.
 * Contract-level password checking/rotation is owned by backend-e2e; here we assert the browser UX.
 */

const opened: BrowserContext[] = [];
test.afterEach(async () => {
  await Promise.all(opened.splice(0).map((c) => c.close()));
});

const PASSWORD = 's3cret';

test('mod enables a password → viewer is gated, wrong is rejected, correct unblurs', async ({ browser, request }) => {
  const event = await createEvent(request);

  // --- Moderator enables a password via the ModPassword control ---
  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret), opened);
  await mod.locator('.password button').click(); // "Password" (disabled → editing)
  await mod.locator('.password input').fill(PASSWORD);
  // Enter commits (set_pwd → mod_edit_event). Wait for the POST so the password is persisted server-side
  // before the viewer loads (avoids a cross-context broadcast race — the viewer then loads masked).
  const saved = mod.waitForResponse(
    (r) => r.url().includes(`/api/mod/event/${event.id}/${event.secret}`) && r.request().method() === 'POST',
  );
  await mod.locator('.password input').press('Enter');
  await saved;
  await expect(mod.locator('.password .confirmed')).toHaveText('*****');

  // --- Viewer loads AFTER the password is set → masked + gated behind the popup ---
  const viewer = await openLoaded(browser, routes.event(event.id), opened);
  await expect(viewer.getByTestId(TID.passwordPopup)).toBeVisible();
  await expect(viewer.getByTestId(TID.eventDesc)).toHaveClass(/blurr/);

  const input = viewer.getByTestId(TID.passwordInput);
  const ok = viewer.locator('.pwd-popup button.dlg-button');

  // (1) invalid format (whitespace → trimmed len < 1): client validation, submit disabled.
  await input.fill(' ');
  await expect(viewer.getByText('invalid password')).toBeVisible();
  await expect(ok).toBeDisabled();

  // (2) valid format but WRONG → server rejects → "try again", still masked.
  await input.fill('nottherightone');
  await expect(ok).toBeEnabled();
  await ok.click();
  await expect(viewer.getByText('try again')).toBeVisible({ timeout: 15_000 });
  await expect(viewer.getByTestId(TID.passwordPopup)).toBeVisible();
  await expect(viewer.getByTestId(TID.eventDesc)).toHaveClass(/blurr/);

  // (3) correct password → grant cookie → refetch → popup closes and content unblurs.
  await input.fill(PASSWORD);
  await expect(ok).toBeEnabled();
  await ok.click();
  await expect(viewer.getByTestId(TID.passwordPopup)).toHaveCount(0, { timeout: 15_000 });
  await expect(viewer.getByTestId(TID.eventDesc)).not.toHaveClass(/blurr/);
});

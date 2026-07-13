import { expect, test } from '@playwright/test';
import { adminLogin } from '../fixtures/admin';
import { routes } from '../fixtures/event';
import { ADMIN_PASSWORD, ADMIN_USERNAME } from '../helpers/env';
import { blockCdns } from '../helpers/net';

/**
 * Admin login page (/login). This is the one flow that proves cross-origin credentialed auth works in
 * the browser: the FE (127.0.0.1:8080) hashes the password client-side and POSTs it to :8090, whose
 * Set-Cookie (SameSite=None; Secure — accepted because localhost is a trustworthy origin) must be
 * stored and re-sent on the follow-up /user fetch for the page to reach the logged-in state.
 *
 * Needs the backend booted with the real sha256("pwd") admin hash; skips cleanly otherwise (the login
 * would silently fail and the page would stay on the form).
 */

test('admin can log in and out via /login', async ({ page, request }) => {
  // Probe first (via the API cookie jar) so we can skip cleanly when the backend has a placeholder hash.
  test.skip(!(await adminLogin(request)), 'admin login unavailable (backend booted with a placeholder LA_ADMIN_PWD_HASH)');

  await blockCdns(page);
  await page.goto(routes.login());

  // Resolves from RequestingInfo ("Waiting...") to the not-logged-in form.
  await expect(page.getByText('Admin Login')).toBeVisible();
  const loginBtn = page.getByRole('button', { name: 'login' });
  await expect(loginBtn).toBeVisible();

  await page.locator('input[placeholder="user name"]').fill(ADMIN_USERNAME);
  await page.locator('input[placeholder="password"]').fill(ADMIN_PASSWORD);
  await loginBtn.click();

  // Logged-in state: the cross-origin cookie was stored + re-sent on /user.
  await expect(page.getByText(`Logged in as: '${ADMIN_USERNAME}'`)).toBeVisible({ timeout: 15_000 });

  // Logout returns to the form.
  await page.getByRole('button', { name: 'logout' }).click();
  await expect(page.getByRole('button', { name: 'login' })).toBeVisible({ timeout: 15_000 });
});

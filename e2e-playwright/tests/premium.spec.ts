import { type BrowserContext, expect, test } from '@playwright/test';
import { adminLogin, upgradeToPremium } from '../fixtures/admin';
import { createEvent, routes } from '../fixtures/event';
import { openLoaded } from '../helpers/app';

/**
 * Premium rendering. Real payment (Stripe/PayPal) checkout is out of scope; we only assert (a) a free
 * event's moderator sees the Upgrade component, and (b) after an out-of-band no-Stripe admin upgrade,
 * the premium affordances render (stats block, screening checkbox, export) and the Upgrade banner is
 * gone. Contract-level upgrade behaviour is owned by backend-e2e.
 */

const opened: BrowserContext[] = [];
test.afterEach(async () => {
  await Promise.all(opened.splice(0).map((c) => c.close()));
});

test('free event: moderator sees the Upgrade banner and no premium affordances', async ({ browser, request }) => {
  const event = await createEvent(request);
  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret), opened);

  await expect(mod.locator('.premium-banner')).toBeVisible();
  await expect(mod.getByText('Upgrade now to')).toBeVisible();
  // Premium-only affordances are absent while free.
  await expect(mod.locator('.statistics')).toHaveCount(0);
  await expect(mod.locator('.premium')).toHaveCount(0);
});

test('admin-upgraded event: moderator sees premium affordances, no Upgrade banner', async ({ browser, request }) => {
  // Needs the no-Stripe admin path (real admin hash); skip cleanly otherwise.
  test.skip(!(await adminLogin(request)), 'admin login unavailable (backend booted with a placeholder LA_ADMIN_PWD_HASH)');

  const event = await createEvent(request);
  expect(await upgradeToPremium(request, event.id, event.secret)).toBeTruthy();

  // Loads already-premium (upgraded before navigation) → no reload needed.
  const mod = await openLoaded(browser, routes.eventMod(event.id, event.secret), opened);

  await expect(mod.locator('.statistics')).toBeVisible(); // realtime stats block
  await expect(mod.getByText('This is a premium event', { exact: true })).toBeVisible(); // .premium title (not the deadline line)
  await expect(mod.locator('#vehicle1')).toBeVisible(); // screening checkbox
  await expect(mod.getByRole('button', { name: 'Export' })).toBeVisible();
  await expect(mod.locator('.premium-banner')).toHaveCount(0); // Upgrade component gone once premium
});

import type { APIRequestContext } from '@playwright/test';
import { ADMIN_PWD_HASH, ADMIN_USERNAME, BACKEND_URL } from '../helpers/env';

/**
 * Admin + premium provisioning via the backend API.
 *
 * The browser normally hashes the typed password client-side and POSTs the hash; here we POST the
 * pre-computed hash directly. The AUTH_COOKIE (a stateless JWT) that login returns is stored in the
 * passed `request`'s cookie jar and automatically re-sent on subsequent same-origin calls, so
 * `adminLogin(request)` then `upgradeToPremium(request, …)` share the admin session.
 */

/**
 * Log in as admin (`POST /api/admin/login`). Returns false when the backend rejects the credentials
 * — which happens if it was booted with a placeholder `LA_ADMIN_PWD_HASH` (e.g. `backend-e2e just
 * serve`, which uses a literal that only works for reqwest tests) rather than the real sha256("pwd")
 * hash that fixtures/backend.ts boots with. Callers gate premium/admin specs on this.
 */
export async function adminLogin(request: APIRequestContext): Promise<boolean> {
  const res = await request.post(`${BACKEND_URL}/api/admin/login`, {
    data: { name: ADMIN_USERNAME, pwd_hash: ADMIN_PWD_HASH },
  });
  return res.ok();
}

/**
 * Upgrade an event to premium via the no-Stripe admin path
 * (`POST /api/mod/event/upgrade/:id/:secret`). Requires a prior `adminLogin(request)` — the admin
 * cookie is what selects the `EventUpgradeResponse::AdminUpgrade` branch server-side; without it the
 * backend tries to build a payment redirect instead. Returns whether the upgrade call succeeded.
 */
export async function upgradeToPremium(
  request: APIRequestContext,
  id: string,
  secret: string,
): Promise<boolean> {
  const res = await request.post(`${BACKEND_URL}/api/mod/event/upgrade/${id}/${secret}`, {
    data: { context: 'Regular' },
  });
  return res.ok();
}

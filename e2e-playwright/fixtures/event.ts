import type { APIRequestContext } from '@playwright/test';
import { BACKEND_URL } from '../helpers/env';

/** An event provisioned via the backend API, with the tokens needed to build its routes. */
export interface CreatedEvent {
  /** publicToken — used in /event/:id and /eventmod/:id/:secret. */
  id: string;
  /** moderatorToken — the :secret in /eventmod/:id/:secret. */
  secret: string;
  name: string;
  description: string;
}

export interface CreateEventOptions {
  name?: string;
  description?: string;
  /** moderatorEmail; null (default) omits it. */
  email?: string | null;
}

// Description must clear CreateEventValidation's MinLength; keep the default comfortably long.
const DEFAULT_DESC =
  'Provisioned by the Playwright E2E harness. This description is intentionally long enough to satisfy validation.';

/**
 * Create a real (test:false → normal lifetime, survives the reconnect down-window) event
 * via `POST /api/event/add`. Body shape is `shared::AddEvent` with its serde renames;
 * response is `shared::EventInfo`.
 *
 * Pass any APIRequestContext (Playwright's `request` fixture, or `page.request`).
 */
export async function createEvent(
  request: APIRequestContext,
  opts: CreateEventOptions = {},
): Promise<CreatedEvent> {
  const name = opts.name ?? `E2E ${Date.now().toString(36)}`;
  const description = opts.description ?? DEFAULT_DESC;

  const res = await request.post(`${BACKEND_URL}/api/event/add`, {
    data: {
      eventData: { name, description, shortUrl: '', longUrl: null, color: null },
      moderatorEmail: opts.email ?? null,
      test: false,
      customer: null,
    },
  });

  if (!res.ok()) {
    throw new Error(`create_event failed: ${res.status()} ${res.statusText()} — ${await res.text()}`);
  }

  const info = (await res.json()) as {
    tokens?: { publicToken?: string; moderatorToken?: string | null };
  };
  const id = info.tokens?.publicToken;
  const secret = info.tokens?.moderatorToken ?? undefined;
  if (!id || !secret) {
    throw new Error(`create_event: missing tokens in response: ${JSON.stringify(info)}`);
  }

  return { id, secret, name, description };
}

/** Route builders (mirror frontend/src/routes.rs). */
export const routes = {
  home: () => '/',
  newEvent: () => '/newevent',
  event: (id: string) => `/event/${id}`,
  eventMod: (id: string, secret: string) => `/eventmod/${id}/${secret}`,
  print: (id: string) => `/event/print/${id}`,
  login: () => '/login',
};

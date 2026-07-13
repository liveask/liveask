import type { FullConfig } from '@playwright/test';
import { BackendServer } from './fixtures/backend';
import { BACKEND_URL } from './helpers/env';

/**
 * Runs once before the suite. The Trunk FE host is started by the config `webServer`, but the
 * backend (+ Redis + DynamoDB-local) must already be running — fail fast with a helpful message
 * rather than letting every test time out.
 *
 * Boot locally with:
 *   cd backend    && just docker-compose   # redis :6379 + dynamodb-local :8000
 *   cd backend-e2e && just serve           # liveask-server :8090 (RELAX_CORS=1)
 */
async function globalSetup(_config: FullConfig): Promise<void> {
  // The reconnect canary owns the :8090 lifecycle itself (it boots liveask-server via
  // fixtures/backend.ts in a beforeAll, so it can SIGKILL/relaunch it). Don't pre-require the
  // backend here in that opt-in mode — run it with only redis + dynamodb-local up.
  if (process.env.E2E_RECONNECT_CANARY === '1') return;

  if (!(await BackendServer.isUp())) {
    throw new Error(
      `Backend not reachable at ${BACKEND_URL}/api/ping.\n` +
        'Start it before running E2E:\n' +
        '  cd backend     && just docker-compose   (redis + dynamodb-local)\n' +
        '  cd backend-e2e && just serve            (liveask-server on :8090)\n',
    );
  }
}

export default globalSetup;

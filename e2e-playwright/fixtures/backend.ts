import { type ChildProcess, spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { ADMIN_PWD_HASH, PING_URL } from '../helpers/env';

const REPO_ROOT = path.resolve(__dirname, '../..');
const BACKEND_DIR = path.join(REPO_ROOT, 'backend');
const PREBUILT_BINARY = path.join(REPO_ROOT, 'target', 'debug', 'liveask-server');

/**
 * Env block that mirrors `backend-e2e/justfile` `serve`, except LA_ADMIN_PWD_HASH, which we
 * set to the real hash of the admin password so BROWSER-based admin login works (the recipe's
 * literal placeholder only works for reqwest tests that POST the hash directly).
 */
function serverEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    DDB_LOCAL: '1',
    DDB_URL: 'http://localhost:8000',
    REDIS_URL: 'redis://localhost:6379',
    LA_PORT: '8090',
    LIVEASK_ENV: 'local',
    BASE_URL: 'http://localhost:8090',
    RELAX_CORS: '1',
    LA_ADMIN_PWD_HASH: ADMIN_PWD_HASH,
    RUST_LOG: 'warn,liveask_server=info',
  };
}

/**
 * Controls a locally-spawned liveask-server (port 8090) for the flagship reconnect fallback,
 * which needs to abruptly kill (SIGKILL — matches the bug) and relaunch the real backend.
 *
 * Requires Redis + DynamoDB-local already up (`cd backend && just docker-compose`); those
 * persist across a server restart so a created event survives the down-window.
 *
 * Uses SIGKILL, never SIGTERM: the graceful shutdown loop has no drain timeout and, with a live
 * browser socket, may never exit (see PLAYWRIGHT_E2E_PLAN.md).
 */
export class BackendServer {
  private proc: ChildProcess | undefined;

  /** Spawn the server. Prefers the prebuilt debug binary for fast restarts; falls back to `cargo run`. */
  start(): void {
    if (this.proc) throw new Error('BackendServer already started');

    const [cmd, args] = existsSync(PREBUILT_BINARY)
      ? [PREBUILT_BINARY, [] as string[]]
      : ['cargo', ['run', '-p', 'liveask-server']];

    this.proc = spawn(cmd, args, {
      cwd: BACKEND_DIR,
      env: serverEnv(),
      stdio: 'inherit',
      detached: false,
    });
  }

  /** SIGKILL the server process (abrupt drop). No-op if not running. */
  stop(): void {
    if (!this.proc) return;
    this.proc.kill('SIGKILL');
    this.proc = undefined;
  }

  /** Poll GET /api/ping until it returns 200 `pong` (deps are connected once it binds). */
  async waitForPing(timeoutMs = 30_000): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    let lastErr: unknown;
    while (Date.now() < deadline) {
      try {
        const res = await fetch(PING_URL);
        if (res.ok && (await res.text()).trim() === 'pong') return;
      } catch (err) {
        lastErr = err;
      }
      await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error(`backend did not become ready within ${timeoutMs}ms: ${String(lastErr)}`);
  }

  /** True if GET /api/ping currently returns `pong`. */
  static async isUp(): Promise<boolean> {
    try {
      const res = await fetch(PING_URL);
      return res.ok && (await res.text()).trim() === 'pong';
    } catch {
      return false;
    }
  }
}

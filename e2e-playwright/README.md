# LiveAsk browser E2E (Playwright)

Drives the **real Trunk-built WASM app** in Chromium against a **real `liveask-server`**, covering
what unit tests and `backend-e2e/` can't: per-route DOM rendering, WASM behaviour, WebSocket
lifecycle, cross-client reactivity, and the flagship reconnect/refetch flow. See
[`../PLAYWRIGHT_E2E_PLAN.md`](../PLAYWRIGHT_E2E_PLAN.md) for the full design and ownership boundary.

## Topology

| Host | Port | What |
|---|---|---|
| Trunk serve (FE, WASM) | `127.0.0.1:8080` | managed by Playwright `webServer` (`cargo make serve`) |
| liveask-server (REST + WS) | `localhost:8090` | booted separately (see below) |
| Redis | `6379` | hard dep |
| DynamoDB-local | `8000` | hard dep |

The bundle bakes in `LA_ENV=local` endpoints (`http://localhost:8090`, `ws://localhost:8090`) at
build time, so it must be built via `cargo make serve` (sets `LA_ENV=local` + `RUSTFLAGS`).
The backend must run with `RELAX_CORS=1` (cross-origin :8080→:8090 with credentials).

## Prerequisites

- Node (see [`.nvmrc`](.nvmrc)), `cargo-make`, `trunk` (repo toolchain).
- One-time: `npm ci && npx playwright install --with-deps chromium`.

## Running

```bash
# 1. deps
cd backend && just docker-compose          # redis :6379 + dynamodb-local :8000

# 2. backend (separate terminal) — NOT managed by Playwright, so reconnect tests can kill it
cd backend-e2e && just serve               # liveask-server :8090, RELAX_CORS=1

# 3. tests (Playwright starts Trunk on :8080 itself)
cd e2e-playwright && npm test
```

`globalSetup` fails fast with instructions if the backend isn't reachable.

## Running in Docker (no local toolchains)

If you'd rather not install Node/cargo/trunk/browsers, run the whole thing in containers. The only
requirement is Docker — everything else (Node, cargo, trunk, Chromium) is built and run inside the
image. From the repo root:

```bash
docker compose -f e2e-playwright/docker-compose.e2e.yml up --build \
    --abort-on-container-exit --exit-code-from e2e
```

The exit code is Playwright's, and the HTML report + traces are written to
`e2e-playwright/{playwright-report,test-results}` on the host. Tear down with:

```bash
docker compose -f e2e-playwright/docker-compose.e2e.yml down -v
```

(If you *do* have Node, `npm run docker:e2e` / `npm run docker:clean` from `e2e-playwright/` are
thin aliases for the two commands above — but they're just for convenience, not required.)

How it works: a multi-stage build compiles the **WASM frontend and the `liveask-server` binary from
source** (so it reflects your local code), then a Playwright runtime image runs the backend + serves
the prebuilt dist + runs Chromium — all in one container, so the WASM bundle's hard-coded
`localhost:8090` resolves. Redis + DynamoDB-local are ephemeral sidecars (fresh state per run). First
build is slow (Rust compile + ~850 MB Playwright image); rebuilds are layer-cached.

Notes:
- The container runs the reconnect fault-injection at the browser layer (`routeWebSocket`/route-abort) —
  there's no separately killable backend here, which is fine for the current specs. The real
  process-kill fallback (`fixtures/backend.ts`) needs a split topology and isn't run in this container.
- First build pulls `rust:1-bookworm` + `mcr.microsoft.com/playwright:v1.61.1-jammy`; keep the
  `PLAYWRIGHT_VERSION` build arg in `docker-compose.e2e.yml` in sync with `@playwright/test` in `package.json`.

## Layout

```
playwright.config.ts   baseURL :8080, chromium, webServer=cargo make serve, clipboard perms
globalSetup.ts         asserts backend /api/ping is up
helpers/
  selectors.ts         central data-testid map (TID.*) + load-state values
  env.ts               URLs, CDN host list, admin creds/hash
  net.ts               blockCdns / clearStorage / abortApi / gateWebSocket (routeWebSocket)
fixtures/
  event.ts             createEvent() + addQuestion() via the API (test:false) + route builders
  admin.ts             adminLogin() + upgradeToPremium() (no-Stripe admin path) for premium specs
  backend.ts           BackendServer: start/stop(SIGKILL)/waitForPing for the reconnect canary
tests/
  smoke.spec.ts             per-route DOM rendering + create/ask happy paths
  reconnect.spec.ts         flagship: cold down-at-load recovery, warm mid-session drop, restart canary
  realtime.spec.ts          two-context cross-client reactivity (ask/answer/hide/upvote/viewer-count)
  reconnect-spike.spec.ts   proves routeWebSocket intercepts the wasm_sockets WS + the down→up recovery
```

## Reconnect canary (nightly, opt-in)

`reconnect.spec.ts` includes a highest-fidelity case that SIGKILLs and relaunches the **real**
`liveask-server`. It must own :8090, so it's defined only when `E2E_RECONNECT_CANARY=1` and runs in
isolation — boot only the deps (no external server), then:

```bash
cd backend && just docker-compose            # redis + dynamodb-local ONLY
cd e2e-playwright && E2E_RECONNECT_CANARY=1 npx playwright test reconnect.spec.ts -g canary
```

In that mode `globalSetup` skips its up-front backend check (the canary boots the server itself via
`fixtures/backend.ts`). It belongs in nightly, not the PR smoke.

## Premium / admin specs

The premium viewer-count spec upgrades an event via the no-Stripe admin path, which needs the backend
booted with the **real** `sha256("pwd")` admin hash (`ADMIN_PWD_HASH` in `helpers/env.ts`) — what
`fixtures/backend.ts` uses. `backend-e2e just serve` boots a placeholder hash instead, so that spec
**skips cleanly** there rather than failing.

## Test hooks

The Yew views carry `data-testid` attributes (mapped in `helpers/selectors.ts`).
Prefer `getByTestId(TID.xxx)` over class/text selectors — the share and question popups both use
`.share-popup`, so class selectors are ambiguous. The shared `TextArea` component gained an optional
`testid` prop (rendered only when set) so `#input-desc` and `#questiontext` are addressable without
stamping unrelated call-sites.

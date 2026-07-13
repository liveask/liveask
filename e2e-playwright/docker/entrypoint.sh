#!/usr/bin/env bash
# Boots the backend inside the container, waits for readiness, then runs Playwright.
# Backend env (DDB_URL, REDIS_URL, RELAX_CORS, LA_PORT, LIVEASK_ENV, LA_ADMIN_PWD_HASH, ...) is
# supplied by docker-compose.e2e.yml. Any args are forwarded to `playwright test`.
set -uo pipefail

start_server() {
  /app/liveask-server &
  SERVER_PID=$!
}

echo ">> starting liveask-server"
start_server

echo ">> waiting for backend /api/ping (deps may still be spinning up) ..."
ready=0
for _ in $(seq 1 90); do
  if curl -sf http://localhost:8090/api/ping >/dev/null 2>&1; then
    ready=1; break
  fi
  # The server exits if Redis/DynamoDB aren't up yet at boot — relaunch until deps are ready.
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo ">> server not up yet (deps warming?), relaunching"
    start_server
  fi
  sleep 1
done

if [ "$ready" -ne 1 ]; then
  echo "!! backend never became ready" >&2
  exit 1
fi
echo ">> backend ready"

cd /app/e2e
exec npx playwright test "$@"

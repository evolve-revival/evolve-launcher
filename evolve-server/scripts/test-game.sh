#!/usr/bin/env bash
set -euo pipefail

GAME_DIR="${GAME_DIR:-/run/media/navitank/Untitled/Projects/EvolveFilesLegacy}"
PROTON="${PROTON:-$HOME/.steam/steam/steamapps/common/Proton 8.0/proton}"
TIMEOUT="${TIMEOUT:-90}"
SERVER_LOG="/tmp/evolve-server-test.log"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

SERVER_PID=""
GAME_PID=""

cleanup() {
    [[ -n "$GAME_PID" ]] && kill "$GAME_PID" 2>/dev/null || true
    [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Building server..."
cd "$REPO_DIR"
make build

echo "==> Setting port-443 capability (via /tmp to work around exFAT)..."
cp ./bin/evolve-server /tmp/evolve-server-bin
sudo setcap cap_net_bind_service=+ep /tmp/evolve-server-bin

echo "==> Clearing game logs..."
: > "$GAME_DIR/kandoC.log"
: > "$GAME_DIR/EVOLVE_LOG.txt"

echo "==> Starting evolve-server on :443..."
PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key \
    /tmp/evolve-server-bin > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!

echo "==> Waiting for server health (up to 10s)..."
for i in $(seq 1 10); do
    if curl -sk --cacert certs/ca.crt https://127.0.0.1/status 2>/dev/null | grep -q '"ok"'; then
        echo "    server ready (${i}s)"
        break
    fi
    if [[ $i -eq 10 ]]; then
        echo "FAIL: server did not become healthy in 10s"
        echo "--- server log ---"
        cat "$SERVER_LOG"
        exit 1
    fi
    sleep 1
done

echo "==> Launching Evolve via Proton..."
STEAM_COMPAT_DATA_PATH="$GAME_DIR/proton_prefix" \
STEAM_COMPAT_CLIENT_INSTALL_PATH="$HOME/.steam/steam" \
"$PROTON" run "$GAME_DIR/bin64_SteamRetail/Evolve.exe" &
GAME_PID=$!

echo "==> Watching for main menu (up to ${TIMEOUT}s)..."
echo "    Pass signal: /queue/waittime with non-empty regionId in server log"
START=$(date +%s)
while true; do
    NOW=$(date +%s)
    ELAPSED=$((NOW - START))

    if [[ $ELAPSED -ge $TIMEOUT ]]; then
        echo "FAIL: main menu not reached within ${TIMEOUT}s"
        echo "--- server log (last 30 lines) ---"
        tail -30 "$SERVER_LOG"
        echo "--- kandoC.log (last 20 lines) ---"
        tail -20 "$GAME_DIR/kandoC.log"
        exit 1
    fi

    # Pass: queue/waittime hit with a real regionId (e.g. IAD, EU, etc.)
    # The game sends regionId= (empty) on first boot; non-empty means main menu loaded.
    if grep -qE 'queue/waittime.*regionId=[A-Z]{2,}' "$SERVER_LOG" 2>/dev/null; then
        echo "PASS: main menu reached (${ELAPSED}s)"
        exit 0
    fi

    sleep 2
done

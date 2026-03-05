#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${1:-6970}"
SERVER_PID=""

cleanup() {
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" >/dev/null 2>&1 || true
        wait "$SERVER_PID" >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "ERROR: required command not found: $1"
        exit 1
    fi
}

pick_browser() {
    for candidate in "${CHROME_BIN:-}" google-chrome chromium chromium-browser; do
        if [ -n "$candidate" ] && command -v "$candidate" >/dev/null 2>&1; then
            command -v "$candidate"
            return 0
        fi
    done

    echo "ERROR: no supported Chrome/Chromium binary found"
    exit 1
}

wait_for_url() {
    python3 - "$1" <<'PY'
import sys
import time
import urllib.request

url = sys.argv[1]
deadline = time.time() + 10.0

while time.time() < deadline:
    try:
        with urllib.request.urlopen(url) as response:
            if response.status < 500:
                sys.exit(0)
    except Exception:
        time.sleep(0.1)

sys.exit(1)
PY
}

need_cmd python3
need_cmd node
need_cmd npm
BROWSER_BIN="$(pick_browser)"

cd "$ROOT"

DEMO_URL="http://127.0.0.1:${PORT}/?wasm=baseline"

echo "==> Serving demo at ${DEMO_URL}"
npm run demo:dev -- --host 127.0.0.1 --port "$PORT" >/dev/null 2>&1 &
SERVER_PID=$!
wait_for_url "http://127.0.0.1:${PORT}/"

echo "==> Loading demo page in headless browser..."
KIMG_DEMO_URL="$DEMO_URL" CHROME_BIN="$BROWSER_BIN" node "$ROOT/scripts/demo-smoke.mjs"

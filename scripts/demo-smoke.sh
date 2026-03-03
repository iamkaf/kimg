#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${1:-6970}"
DOM_FILE=""
SERVER_PID=""

cleanup() {
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" >/dev/null 2>&1 || true
        wait "$SERVER_PID" >/dev/null 2>&1 || true
    fi

    if [ -n "${DOM_FILE:-}" ] && [ -f "$DOM_FILE" ]; then
        rm -f "$DOM_FILE"
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
            printf '%s\n' "$candidate"
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
BROWSER_BIN="$(pick_browser)"

cd "$ROOT"

DEMO_URL="http://127.0.0.1:${PORT}/demo/?wasm=baseline"

echo "==> Serving demo at ${DEMO_URL}"
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$ROOT" >/dev/null 2>&1 &
SERVER_PID=$!
wait_for_url "http://127.0.0.1:${PORT}/demo/"

CHROME_FLAGS=(
    --headless=new
    --disable-gpu
    --disable-dev-shm-usage
    --run-all-compositor-stages-before-draw
    --virtual-time-budget=30000
    --dump-dom
)
if [ "$(id -u)" -eq 0 ] || [ "${CI:-}" = "true" ] || [ "${GITHUB_ACTIONS:-}" = "true" ]; then
    CHROME_FLAGS+=(--no-sandbox)
fi

echo "==> Loading demo page in headless browser..."
DOM="$("$BROWSER_BIN" "${CHROME_FLAGS[@]}" "$DEMO_URL")"
DOM_FILE="$(mktemp)"
printf '%s' "$DOM" >"$DOM_FILE"

python3 - "$DOM_FILE" <<'PY'
import html
import re
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    dom = handle.read()
match = re.search(r"<body\b([^>]*)>", dom, re.S)
if not match:
    raise SystemExit("ERROR: body tag not found in demo DOM")

attrs = dict(re.findall(r'data-([a-z0-9-]+)="([^"]*)"', html.unescape(match.group(1))))

status = attrs.get("suite-status")
cards = int(attrs.get("suite-count", 0))
passed = int(attrs.get("suite-pass", 0))
failed = int(attrs.get("suite-fail", 0))
experimental = int(attrs.get("suite-experimental", 0))
diagnostics = int(attrs.get("suite-diagnostics", 0))

print(
    "demo-status:"
    f" status={status}"
    f" cards={cards}"
    f" pass={passed}"
    f" fail={failed}"
    f" experimental={experimental}"
    f" diagnostics={diagnostics}"
)

if status != "completed":
    raise SystemExit(f"ERROR: demo did not complete cleanly (status={status})")
if cards < 20:
    raise SystemExit(f"ERROR: demo rendered too few cards ({cards})")
if failed != 0:
    raise SystemExit(f"ERROR: demo reported failing cards ({failed})")
if diagnostics != 0:
    raise SystemExit(f"ERROR: demo captured diagnostics ({diagnostics})")
if passed <= 0:
    raise SystemExit("ERROR: demo reported zero passing cards")
if passed + failed + experimental != cards:
    raise SystemExit(
        "ERROR: demo counters do not add up "
        f"(pass={passed}, fail={failed}, experimental={experimental}, cards={cards})"
    )
PY

echo "==> Demo smoke test passed."

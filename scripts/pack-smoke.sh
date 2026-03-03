#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
PACK_FILE=""
SERVER_PID=""

cleanup() {
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" >/dev/null 2>&1 || true
        wait "$SERVER_PID" >/dev/null 2>&1 || true
    fi

    if [ -n "${PACK_FILE:-}" ] && [ -f "$ROOT/$PACK_FILE" ]; then
        rm -f "$ROOT/$PACK_FILE"
    fi

    rm -rf "$TMP_DIR"
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
deadline = time.time() + 5.0

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

need_cmd npm
need_cmd node
need_cmd python3
BROWSER_BIN="$(pick_browser)"

echo "==> Packing @iamkaf/kimg..."
PACK_JSON="$(
    cd "$ROOT" \
        && npm pack --json 2>&1 \
        | node -e '
            const fs = require("fs");
            const input = fs.readFileSync(0, "utf8");
            const match = input.match(/\[\s*\{[\s\S]*\}\s*\]\s*$/);
            if (!match) {
              throw new Error("npm pack output did not end with JSON");
            }
            process.stdout.write(match[0]);
        '
)"
PACK_FILE="$(
    printf '%s' "$PACK_JSON" \
        | node -e 'const fs = require("fs"); const data = JSON.parse(fs.readFileSync(0, "utf8")); process.stdout.write(data[0].filename);'
)"
PACK_PATH="$ROOT/$PACK_FILE"

echo "==> Node install smoke test..."
NODE_DIR="$TMP_DIR/node"
mkdir -p "$NODE_DIR"
pushd "$NODE_DIR" >/dev/null
npm init -y >/dev/null
npm install "$PACK_PATH" >/dev/null
cat > smoke.mjs <<'EOF'
import { Composition, detectFormat, rgbToHex } from "@iamkaf/kimg";

const rgba = new Uint8Array([
  255, 0, 0, 255,
  0, 255, 0, 255,
  0, 0, 255, 255,
  255, 255, 255, 255,
]);

const composition = await Composition.create({ width: 2, height: 2 });
composition.addImageLayer({
  name: "smoke",
  rgba,
  width: 2,
  height: 2,
});

const png = composition.exportPng();
if ((await detectFormat(png)) !== "png") {
  throw new Error("Node smoke test failed: expected PNG export");
}

if ((await rgbToHex(255, 128, 0)) !== "#ff8000") {
  throw new Error("Node smoke test failed: rgbToHex mismatch");
}

console.log("node-pack-ok");
EOF
node smoke.mjs
popd >/dev/null

echo "==> Browser install smoke test..."
BROWSER_DIR="$TMP_DIR/browser"
mkdir -p "$BROWSER_DIR"
pushd "$BROWSER_DIR" >/dev/null
npm init -y >/dev/null
npm install "$PACK_PATH" >/dev/null
cat > index.html <<'EOF'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>kimg pack smoke</title>
    <script type="importmap">
      {
        "imports": {
          "@iamkaf/kimg": "./node_modules/@iamkaf/kimg/dist/index.js",
          "@iamkaf/kimg/color-utils": "./node_modules/@iamkaf/kimg/dist/color-utils.js"
        }
      }
    </script>
  </head>
  <body data-status="pending">loading</body>
  <script type="module">
    import { Composition, detectFormat } from "@iamkaf/kimg";
    import { readableTextColor } from "@iamkaf/kimg/color-utils";

    const rgba = new Uint8Array([
      255, 0, 0, 255,
      0, 255, 0, 255,
      0, 0, 255, 255,
      255, 255, 255, 255,
    ]);

    try {
      const composition = await Composition.create({ width: 2, height: 2 });
      composition.addImageLayer({
        name: "smoke",
        rgba,
        width: 2,
        height: 2,
      });

      const png = composition.exportPng();
      const format = await detectFormat(png);
      const textColor = readableTextColor("#111111");
      if (format !== "png") {
        throw new Error(`expected png, got ${format}`);
      }
      if (textColor !== "#ffffff") {
        throw new Error(`expected #ffffff, got ${textColor}`);
      }

      document.body.dataset.status = "ok";
      document.body.textContent = `browser-pack-ok:${format}:${textColor}`;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      document.body.dataset.status = "error";
      document.body.textContent = `browser-pack-failed:${message}`;
      console.error(error);
    }
  </script>
</html>
EOF

PORT="$(
    python3 - <<'PY'
import socket

sock = socket.socket()
sock.bind(("127.0.0.1", 0))
print(sock.getsockname()[1])
sock.close()
PY
)"

python3 -m http.server "$PORT" --bind 127.0.0.1 >/dev/null 2>&1 &
SERVER_PID=$!
wait_for_url "http://127.0.0.1:$PORT/index.html"

CHROME_FLAGS=(
    --headless=new
    --disable-gpu
    --virtual-time-budget=10000
    --dump-dom
)
if [ "$(id -u)" -eq 0 ]; then
    CHROME_FLAGS+=(--no-sandbox)
fi

DOM="$("$BROWSER_BIN" "${CHROME_FLAGS[@]}" "http://127.0.0.1:$PORT/index.html")"
printf '%s\n' "$DOM"
if ! grep -q "browser-pack-ok:png:#ffffff" <<<"$DOM"; then
    echo "ERROR: browser smoke test did not reach the expected success state"
    exit 1
fi
popd >/dev/null
kill "$SERVER_PID" >/dev/null 2>&1 || true
wait "$SERVER_PID" >/dev/null 2>&1 || true
SERVER_PID=""

echo "==> Pack smoke tests passed."

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${1:-6969}"

echo "==> Serving kimg demo at http://localhost:${PORT}/demo/"
echo "    Press Ctrl+C to stop."
echo ""

cd "$ROOT"
python3 -m http.server "$PORT"

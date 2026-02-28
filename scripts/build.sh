#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/pkg"

echo "==> Building kimg-wasm (release, wasm32-unknown-unknown)..."
cargo build --manifest-path "$ROOT/Cargo.toml" \
    --target wasm32-unknown-unknown \
    --release \
    -p kimg-wasm

WASM_FILE="$ROOT/target/wasm32-unknown-unknown/release/kimg_wasm.wasm"

if [ ! -f "$WASM_FILE" ]; then
    echo "ERROR: WASM file not found at $WASM_FILE"
    exit 1
fi

echo "==> Generating JS bindings with wasm-bindgen..."
mkdir -p "$PKG"
wasm-bindgen "$WASM_FILE" \
    --out-dir "$PKG" \
    --target web \
    --typescript

# Optional: optimize with wasm-opt if available
if command -v wasm-opt &> /dev/null; then
    echo "==> Optimizing with wasm-opt..."
    wasm-opt -Os "$PKG/kimg_wasm_bg.wasm" -o "$PKG/kimg_wasm_bg.wasm"
else
    echo "    (wasm-opt not found, skipping optimization)"
fi

echo ""
echo "==> Build complete. Output in $PKG/"
ls -lh "$PKG/"

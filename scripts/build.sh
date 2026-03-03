#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JS_SRC="$ROOT/js"
DIST="$ROOT/dist"
PACKAGE_JSON="$ROOT/package.json"
BASELINE_OUT="$(mktemp -d)"
SIMD_OUT="$(mktemp -d)"
SVG_BASELINE_OUT="$(mktemp -d)"
SVG_SIMD_OUT="$(mktemp -d)"
TEXT_BASELINE_OUT="$(mktemp -d)"
TEXT_SIMD_OUT="$(mktemp -d)"
TEXT_SVG_BASELINE_OUT="$(mktemp -d)"
TEXT_SVG_SIMD_OUT="$(mktemp -d)"
GENERATED_TYPES=(
    "$JS_SRC/kimg_wasm_bg.d.ts"
    "$JS_SRC/kimg_wasm_simd.d.ts"
    "$JS_SRC/kimg_wasm_svg_bg.d.ts"
    "$JS_SRC/kimg_wasm_svg_simd.d.ts"
    "$JS_SRC/kimg_wasm_bg.js"
    "$JS_SRC/kimg_wasm_simd.js"
    "$JS_SRC/kimg_wasm_svg_bg.js"
    "$JS_SRC/kimg_wasm_svg_simd.js"
    "$JS_SRC/kimg_wasm_text_bg.d.ts"
    "$JS_SRC/kimg_wasm_text_simd.d.ts"
    "$JS_SRC/kimg_wasm_text_svg_bg.d.ts"
    "$JS_SRC/kimg_wasm_text_svg_simd.d.ts"
    "$JS_SRC/kimg_wasm_text_bg.js"
    "$JS_SRC/kimg_wasm_text_simd.js"
    "$JS_SRC/kimg_wasm_text_svg_bg.js"
    "$JS_SRC/kimg_wasm_text_svg_simd.js"
)

cleanup() {
    rm -rf "$BASELINE_OUT" "$SIMD_OUT" "$SVG_BASELINE_OUT" "$SVG_SIMD_OUT" \
        "$TEXT_BASELINE_OUT" "$TEXT_SIMD_OUT" "$TEXT_SVG_BASELINE_OUT" "$TEXT_SVG_SIMD_OUT"
    rm -f "${GENERATED_TYPES[@]}"
}

trap cleanup EXIT

build_variant() {
    local target_dir="$1"
    local out_dir="$2"
    local out_name="$3"
    shift 3

    cargo build --manifest-path "$ROOT/Cargo.toml" \
        --target wasm32-unknown-unknown \
        --target-dir "$target_dir" \
        --release \
        -p kimg-wasm \
        "$@"

    local wasm_file="$target_dir/wasm32-unknown-unknown/release/kimg_wasm.wasm"
    if [ ! -f "$wasm_file" ]; then
        echo "ERROR: WASM file not found at $wasm_file"
        exit 1
    fi

    wasm-bindgen "$wasm_file" \
        --out-dir "$out_dir" \
        --out-name "$out_name" \
        --target web \
        --typescript
}

echo "==> Building baseline kimg-wasm (release, wasm32-unknown-unknown)..."
build_variant "$ROOT/target/wasm32-baseline" "$BASELINE_OUT" "kimg_wasm"

echo "==> Building SIMD kimg-wasm (release, wasm32-unknown-unknown + simd128)..."
RUSTFLAGS="-Ctarget-feature=+simd128" \
    build_variant "$ROOT/target/wasm32-simd" "$SIMD_OUT" "kimg_wasm_simd"

echo "==> Building baseline SVG-enabled kimg-wasm (release, wasm32-unknown-unknown)..."
build_variant "$ROOT/target/wasm32-svg-baseline" "$SVG_BASELINE_OUT" "kimg_wasm_svg" \
    --features svg-backend

echo "==> Building SIMD SVG-enabled kimg-wasm (release, wasm32-unknown-unknown + simd128)..."
RUSTFLAGS="-Ctarget-feature=+simd128" \
    build_variant "$ROOT/target/wasm32-svg-simd" "$SVG_SIMD_OUT" "kimg_wasm_svg_simd" \
    --features svg-backend

echo "==> Building baseline text-enabled kimg-wasm (release, wasm32-unknown-unknown)..."
build_variant "$ROOT/target/wasm32-text-baseline" "$TEXT_BASELINE_OUT" "kimg_wasm_text" \
    --features cosmic-text-backend

echo "==> Building SIMD text-enabled kimg-wasm (release, wasm32-unknown-unknown + simd128)..."
RUSTFLAGS="-Ctarget-feature=+simd128" \
    build_variant "$ROOT/target/wasm32-text-simd" "$TEXT_SIMD_OUT" "kimg_wasm_text_simd" \
    --features cosmic-text-backend

echo "==> Building baseline text+SVG-enabled kimg-wasm (release, wasm32-unknown-unknown)..."
build_variant "$ROOT/target/wasm32-text-svg-baseline" "$TEXT_SVG_BASELINE_OUT" "kimg_wasm_text_svg" \
    --features cosmic-text-backend,svg-backend

echo "==> Building SIMD text+SVG-enabled kimg-wasm (release, wasm32-unknown-unknown + simd128)..."
RUSTFLAGS="-Ctarget-feature=+simd128" \
    build_variant "$ROOT/target/wasm32-text-svg-simd" "$TEXT_SVG_SIMD_OUT" "kimg_wasm_text_svg_simd" \
    --features cosmic-text-backend,svg-backend

echo "==> Preparing dist/..."
rm -rf "$DIST"
mkdir -p "$DIST"

if [ ! -f "$PACKAGE_JSON" ]; then
    echo "ERROR: package.json not found at $PACKAGE_JSON"
    exit 1
fi

if ! command -v npx &> /dev/null; then
    echo "ERROR: npx not found; install Node.js/npm to build the TypeScript wrapper."
    exit 1
fi

echo "==> Staging generated wasm-bindgen types for wrapper compilation..."
rm -f "${GENERATED_TYPES[@]}"
cp "$BASELINE_OUT/kimg_wasm.d.ts" "$JS_SRC/kimg_wasm_bg.d.ts"
cp "$SIMD_OUT/kimg_wasm_simd.d.ts" "$JS_SRC/kimg_wasm_simd.d.ts"
cp "$SVG_BASELINE_OUT/kimg_wasm_svg.d.ts" "$JS_SRC/kimg_wasm_svg_bg.d.ts"
cp "$SVG_SIMD_OUT/kimg_wasm_svg_simd.d.ts" "$JS_SRC/kimg_wasm_svg_simd.d.ts"
cp "$BASELINE_OUT/kimg_wasm.js" "$JS_SRC/kimg_wasm_bg.js"
cp "$SIMD_OUT/kimg_wasm_simd.js" "$JS_SRC/kimg_wasm_simd.js"
cp "$SVG_BASELINE_OUT/kimg_wasm_svg.js" "$JS_SRC/kimg_wasm_svg_bg.js"
cp "$SVG_SIMD_OUT/kimg_wasm_svg_simd.js" "$JS_SRC/kimg_wasm_svg_simd.js"
cp "$TEXT_BASELINE_OUT/kimg_wasm_text.d.ts" "$JS_SRC/kimg_wasm_text_bg.d.ts"
cp "$TEXT_SIMD_OUT/kimg_wasm_text_simd.d.ts" "$JS_SRC/kimg_wasm_text_simd.d.ts"
cp "$TEXT_SVG_BASELINE_OUT/kimg_wasm_text_svg.d.ts" "$JS_SRC/kimg_wasm_text_svg_bg.d.ts"
cp "$TEXT_SVG_SIMD_OUT/kimg_wasm_text_svg_simd.d.ts" "$JS_SRC/kimg_wasm_text_svg_simd.d.ts"
cp "$TEXT_BASELINE_OUT/kimg_wasm_text.js" "$JS_SRC/kimg_wasm_text_bg.js"
cp "$TEXT_SIMD_OUT/kimg_wasm_text_simd.js" "$JS_SRC/kimg_wasm_text_simd.js"
cp "$TEXT_SVG_BASELINE_OUT/kimg_wasm_text_svg.js" "$JS_SRC/kimg_wasm_text_svg_bg.js"
cp "$TEXT_SVG_SIMD_OUT/kimg_wasm_text_svg_simd.js" "$JS_SRC/kimg_wasm_text_svg_simd.js"

echo "==> Compiling TypeScript wrapper into dist/ with tsgo..."
npx --no-install tsgo -p "$ROOT/tsconfig.json"

echo "==> Copying generated bindings into dist/..."
cp "$BASELINE_OUT/kimg_wasm_bg.wasm" "$DIST/kimg_wasm_bg.wasm"
cp "$BASELINE_OUT/kimg_wasm_bg.wasm.d.ts" "$DIST/kimg_wasm_bg.wasm.d.ts"
cp "$BASELINE_OUT/kimg_wasm.d.ts" "$DIST/kimg_wasm_bg.d.ts"
sed 's|@ts-self-types="./kimg_wasm.d.ts"|@ts-self-types="./kimg_wasm_bg.d.ts"|' \
    "$BASELINE_OUT/kimg_wasm.js" > "$DIST/kimg_wasm_bg.js"

cp "$SIMD_OUT/kimg_wasm_simd.js" "$DIST/kimg_wasm_simd.js"
cp "$SIMD_OUT/kimg_wasm_simd.d.ts" "$DIST/kimg_wasm_simd.d.ts"
cp "$SIMD_OUT/kimg_wasm_simd_bg.wasm" "$DIST/kimg_wasm_simd_bg.wasm"
cp "$SIMD_OUT/kimg_wasm_simd_bg.wasm.d.ts" "$DIST/kimg_wasm_simd_bg.wasm.d.ts"

cp "$SVG_BASELINE_OUT/kimg_wasm_svg_bg.wasm" "$DIST/kimg_wasm_svg_bg.wasm"
cp "$SVG_BASELINE_OUT/kimg_wasm_svg_bg.wasm.d.ts" "$DIST/kimg_wasm_svg_bg.wasm.d.ts"
cp "$SVG_BASELINE_OUT/kimg_wasm_svg.d.ts" "$DIST/kimg_wasm_svg_bg.d.ts"
sed 's|@ts-self-types="./kimg_wasm_svg.d.ts"|@ts-self-types="./kimg_wasm_svg_bg.d.ts"|' \
    "$SVG_BASELINE_OUT/kimg_wasm_svg.js" > "$DIST/kimg_wasm_svg_bg.js"

cp "$SVG_SIMD_OUT/kimg_wasm_svg_simd.js" "$DIST/kimg_wasm_svg_simd.js"
cp "$SVG_SIMD_OUT/kimg_wasm_svg_simd.d.ts" "$DIST/kimg_wasm_svg_simd.d.ts"
cp "$SVG_SIMD_OUT/kimg_wasm_svg_simd_bg.wasm" "$DIST/kimg_wasm_svg_simd_bg.wasm"
cp "$SVG_SIMD_OUT/kimg_wasm_svg_simd_bg.wasm.d.ts" "$DIST/kimg_wasm_svg_simd_bg.wasm.d.ts"

cp "$TEXT_BASELINE_OUT/kimg_wasm_text_bg.wasm" "$DIST/kimg_wasm_text_bg.wasm"
cp "$TEXT_BASELINE_OUT/kimg_wasm_text_bg.wasm.d.ts" "$DIST/kimg_wasm_text_bg.wasm.d.ts"
cp "$TEXT_BASELINE_OUT/kimg_wasm_text.d.ts" "$DIST/kimg_wasm_text_bg.d.ts"
sed 's|@ts-self-types="./kimg_wasm_text.d.ts"|@ts-self-types="./kimg_wasm_text_bg.d.ts"|' \
    "$TEXT_BASELINE_OUT/kimg_wasm_text.js" > "$DIST/kimg_wasm_text_bg.js"

cp "$TEXT_SIMD_OUT/kimg_wasm_text_simd.js" "$DIST/kimg_wasm_text_simd.js"
cp "$TEXT_SIMD_OUT/kimg_wasm_text_simd.d.ts" "$DIST/kimg_wasm_text_simd.d.ts"
cp "$TEXT_SIMD_OUT/kimg_wasm_text_simd_bg.wasm" "$DIST/kimg_wasm_text_simd_bg.wasm"
cp "$TEXT_SIMD_OUT/kimg_wasm_text_simd_bg.wasm.d.ts" "$DIST/kimg_wasm_text_simd_bg.wasm.d.ts"

cp "$TEXT_SVG_BASELINE_OUT/kimg_wasm_text_svg_bg.wasm" "$DIST/kimg_wasm_text_svg_bg.wasm"
cp "$TEXT_SVG_BASELINE_OUT/kimg_wasm_text_svg_bg.wasm.d.ts" "$DIST/kimg_wasm_text_svg_bg.wasm.d.ts"
cp "$TEXT_SVG_BASELINE_OUT/kimg_wasm_text_svg.d.ts" "$DIST/kimg_wasm_text_svg_bg.d.ts"
sed 's|@ts-self-types="./kimg_wasm_text_svg.d.ts"|@ts-self-types="./kimg_wasm_text_svg_bg.d.ts"|' \
    "$TEXT_SVG_BASELINE_OUT/kimg_wasm_text_svg.js" > "$DIST/kimg_wasm_text_svg_bg.js"

cp "$TEXT_SVG_SIMD_OUT/kimg_wasm_text_svg_simd.js" "$DIST/kimg_wasm_text_svg_simd.js"
cp "$TEXT_SVG_SIMD_OUT/kimg_wasm_text_svg_simd.d.ts" "$DIST/kimg_wasm_text_svg_simd.d.ts"
cp "$TEXT_SVG_SIMD_OUT/kimg_wasm_text_svg_simd_bg.wasm" "$DIST/kimg_wasm_text_svg_simd_bg.wasm"
cp "$TEXT_SVG_SIMD_OUT/kimg_wasm_text_svg_simd_bg.wasm.d.ts" "$DIST/kimg_wasm_text_svg_simd_bg.wasm.d.ts"

# Optional: optimize with wasm-opt if available
if command -v wasm-opt &> /dev/null; then
    echo "==> Optimizing with wasm-opt..."
    wasm-opt -Os "$DIST/kimg_wasm_bg.wasm" -o "$DIST/kimg_wasm_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_simd_bg.wasm" -o "$DIST/kimg_wasm_simd_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_svg_bg.wasm" -o "$DIST/kimg_wasm_svg_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_svg_simd_bg.wasm" -o "$DIST/kimg_wasm_svg_simd_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_text_bg.wasm" -o "$DIST/kimg_wasm_text_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_text_simd_bg.wasm" -o "$DIST/kimg_wasm_text_simd_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_text_svg_bg.wasm" -o "$DIST/kimg_wasm_text_svg_bg.wasm"
    wasm-opt -Os "$DIST/kimg_wasm_text_svg_simd_bg.wasm" -o "$DIST/kimg_wasm_text_svg_simd_bg.wasm"
else
    echo "    (wasm-opt not found, skipping optimization)"
fi

echo ""
echo "==> Build complete. Output in $DIST/"
ls -lh "$DIST/"

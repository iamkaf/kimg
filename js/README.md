# kimg

Rust+WASM pixel engine for layer-based image compositing.

## Browser

```js
import init, { Composition } from 'kimg';

await init(); // auto-selects the SIMD wasm build when supported

const doc = new Composition(128, 128);
doc.add_image_layer('bg', rgbaData, 128, 128, 0, 0);
const png = doc.export_png();
```

## Node.js

```js
import { readFileSync } from 'fs';
import { initSync, Composition, simdSupported } from 'kimg';

const wasmName = simdSupported() ? 'kimg_wasm_simd_bg.wasm' : 'kimg_wasm_bg.wasm';
const wasm = readFileSync(new URL(wasmName, import.meta.resolve('kimg')));
initSync({ module: wasm });

const doc = new Composition(128, 128);
// ...
```

## Subpath Exports

```js
// Base64 RGBA helpers (pure JS, no WASM needed)
import { rgbaToBase64, base64ToRgba } from 'kimg/base64';

// Color utilities (requires WASM init first)
import { readableTextColor } from 'kimg/color-utils';

// Low-level wasm-bound surface
import initRaw, { Composition as RawComposition } from 'kimg/raw';
```

# kimg

Rust+WASM pixel engine for layer-based image compositing.

## Browser

```js
import { Composition, loadGoogleFont, registerFont } from "@iamkaf/kimg";

const doc = await Composition.create({ width: 128, height: 128 });
const layerId = doc.addImageLayer({
  name: "bg",
  rgba: rgbaData,
  width: 128,
  height: 128,
  x: 0,
  y: 0,
});
doc.updateLayer(layerId, {
  anchor: "center",
  rotation: 22.5,
  scaleX: 1.25,
  scaleY: 0.75,
});
doc.bucketFillLayer(layerId, {
  x: 8,
  y: 8,
  color: [0, 255, 0, 255],
  contiguous: true,
  tolerance: 0,
});
doc.paintStrokeLayer(layerId, {
  color: [201, 73, 45, 255],
  size: 12,
  hardness: 0.8,
  spacing: 0.4,
  points: [
    { x: 12, y: 18, pressure: 0.3 },
    { x: 42, y: 26, pressure: 0.8 },
    { x: 88, y: 44, pressure: 1.0 },
  ],
});
const png = doc.exportPng();

doc.addShapeLayer({
  name: "badge",
  type: "rectangle",
  x: 16,
  y: 16,
  width: 48,
  height: 24,
  radius: 8,
  fill: [255, 0, 0, 255],
  stroke: { color: [255, 255, 255, 255], width: 2 },
});

await registerFont({
  family: "Inter",
  bytes: interFontBytes,
  weight: 400,
  style: "normal",
});

doc.addTextLayer({
  name: "title",
  text: "HELLO\nKIMG",
  fontFamily: "Inter",
  color: [24, 77, 163, 255],
  fontSize: 24,
  lineHeight: 28,
  letterSpacing: 2,
  x: 20,
  y: 20,
});

await loadGoogleFont({
  family: "Inter",
  weights: [400, 700],
  text: "HELLOKIMG",
});

doc.addSvgLayer({
  name: "logo",
  svg: svgMarkup,
  width: 96,
  height: 96,
  x: 16,
  y: 16,
});
```

`Composition.create()` and `Composition.deserialize()` use the text-enabled wasm renderer from the main package, so text works without a separate init step. In browsers, SVG support is lazy-loaded the first time you add an SVG layer or deserialize a document that already contains one; on Node the main package uses the full text+SVG renderer eagerly. `loadGoogleFont()` is browser-only; on Node use `registerFont()` with raw bytes. SVG layers keep the original markup around for scalable rendering until you call `rasterizeSvgLayer(id)`.

## Node.js

```js
import { Composition } from "@iamkaf/kimg";

const doc = await Composition.create({ width: 128, height: 128 });
// ...
```

## Subpath Exports

```js
// Base64 RGBA helpers (pure JS, no WASM needed)
import { rgbaToBase64, base64ToRgba } from "@iamkaf/kimg/base64";

// Color utilities
import { readableTextColor } from "@iamkaf/kimg/color-utils";

// Low-level wasm-bound surface (browser)
import initRaw, { Composition as RawComposition } from "@iamkaf/kimg/raw";

await initRaw();
const raw = new RawComposition(128, 128);

// Low-level wasm-bound surface (Node.js)
import { readFileSync } from "node:fs";
import { initSync } from "@iamkaf/kimg/raw";

const wasm = readFileSync(new URL("./kimg_wasm_bg.wasm", import.meta.url));
initSync({ module: wasm });
```

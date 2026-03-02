# kimg

Rust+WASM pixel engine for layer-based image compositing.

## Browser

```js
import { Composition } from '@iamkaf/kimg';

const doc = await Composition.create({ width: 128, height: 128 });
doc.add_image_layer('bg', rgbaData, 128, 128, 0, 0);
const png = doc.export_png();
```

## Node.js

```js
import { Composition } from '@iamkaf/kimg';

const doc = await Composition.create({ width: 128, height: 128 });
// ...
```

## Subpath Exports

```js
// Base64 RGBA helpers (pure JS, no WASM needed)
import { rgbaToBase64, base64ToRgba } from '@iamkaf/kimg/base64';

// Color utilities
import { readableTextColor } from '@iamkaf/kimg/color-utils';

// Low-level wasm-bound surface
import initRaw, { Composition as RawComposition } from '@iamkaf/kimg/raw';

await initRaw();
const raw = new RawComposition(128, 128);
```

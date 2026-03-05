import { Composition, detectFormat, extractPaletteFromRgba } from "#kimg/index.js";
import { base64ToRgba, rgbaToBase64 } from "#kimg/base64.js";
import { readableTextColor } from "#kimg/color-utils.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView, swatchView, codeView, messageView } from "../helpers/views.js";
import { rgbaEquals, decodeBase64 } from "../helpers/context.js";

const SIMPLE_SVG = `
  <svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96">
    <defs>
      <linearGradient id="chip" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#d9482b" />
        <stop offset="100%" stop-color="#2d55d7" />
      </linearGradient>
    </defs>
    <rect x="10" y="10" width="76" height="76" rx="18" fill="url(#chip)" />
    <circle cx="34" cy="34" r="10" fill="#f2c94c" />
    <path d="M28 62 L48 42 L67 61" fill="none" stroke="#fffaf1" stroke-width="9" stroke-linecap="round" />
    <rect x="54" y="24" width="12" height="24" rx="6" fill="#fffaf1" />
  </svg>
`;

async function buildExportScene(context) {
  const composition = await Composition.create({ width: 264, height: 184 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  composition.addGradientLayer({
    direction: "diagonalUp",
    name: "shade",
    stops: [
      { color: [255, 255, 255, 0], position: 0 },
      { color: [35, 79, 221, 42], position: 1 },
    ],
  });
  composition.addImageLayer({
    height: context.fixture.height,
    name: "teapot",
    rgba: context.fixture.rgba,
    width: context.fixture.width,
    x: 38,
    y: 8,
  });
  composition.addShapeLayer({
    fill: [201, 73, 45, 32],
    height: 110,
    name: "halo",
    stroke: { color: [201, 73, 45, 110], width: 3 },
    type: "ellipse",
    width: 110,
    x: 144,
    y: 26,
  });
  return composition;
}

export const ioTests = [
  {
    expectation:
      "An SVG layer should retain its kind before rasterization. After rasterizeSvgLayer(), the kind should change to 'raster'. Both compositions should render non-empty pixels.",
    section: "io",
    title: "SVG Layer Import and Rasterize",
    async run() {
      const verify = createVerifier();
      const retained = await Composition.create({ width: 208, height: 132 });
      const rasterized = await Composition.create({ width: 208, height: 132 });

      try {
        retained.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        rasterized.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });

        const retainedId = retained.addSvgLayer({ name: "logo", svg: SIMPLE_SVG, width: 96, height: 96, x: 56, y: 18 });
        const rasterizedId = rasterized.addSvgLayer({ name: "logo", svg: SIMPLE_SVG, width: 96, height: 96, x: 56, y: 18 });

        retained.updateLayer(retainedId, { anchor: "center", rotation: -14, scaleX: 1.45, scaleY: 1.45 });
        rasterized.updateLayer(rasterizedId, { anchor: "center", rotation: -14, scaleX: 1.45, scaleY: 1.45 });

        const retainedInfo = retained.getLayer(retainedId);
        const retainedLocal = retained.getLayerRgba(retainedId);
        verify.equal(retainedInfo?.kind, "svg", "retained layer should stay svg-backed before rasterize");
        verify.equal(retainedLocal.length, 96 * 96 * 4, "getLayerRgba should rasterize the local SVG bounds");

        verify.equal(rasterized.rasterizeSvgLayer(rasterizedId), true, "rasterizeSvgLayer should succeed");
        const rasterizedInfo = rasterized.getLayer(rasterizedId);
        verify.equal(rasterizedInfo?.kind, "raster", "rasterized layer should become a raster layer");
        verify.ok(retained.renderRgba().some((v) => v !== 0), "retained svg composition should render");
        verify.ok(rasterized.renderRgba().some((v) => v !== 0), "rasterized svg composition should render");

        return {
          assertions: verify.count,
          metrics: [
            ["Retained kind", retainedInfo?.kind ?? "n/a"],
            ["Rasterized kind", rasterizedInfo?.kind ?? "n/a"],
            ["Local bounds", `${retainedInfo?.width ?? "?"}x${retainedInfo?.height ?? "?"}`],
            ["Transform", `${retainedInfo?.scaleX?.toFixed(2) ?? "n/a"}x @ ${retainedInfo?.rotation?.toFixed(1) ?? "n/a"}°`],
          ],
          views: [
            rgbaView("Retained SVG layer", retained.renderRgba(), retained.width, retained.height),
            rgbaView("After rasterizeSvgLayer()", rasterized.renderRgba(), rasterized.width, rasterized.height),
          ],
        };
      } finally {
        retained.free();
        rasterized.free();
      }
    },
  },
  {
    expectation:
      "Pixel scaling should keep edges crisp, the atlas should pack each sprite once, and the contact sheet should lay them out in a regular grid.",
    featured: true,
    fullSpan: true,
    previewMin: 250,
    section: "io",
    title: "Sprite Helpers and Contact Sheet",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 196, height: 96 });

      try {
        const sprites = context.spriteFixtures.map((sprite, index) =>
          composition.addImageLayer({
            height: sprite.height,
            name: `sprite-${index + 1}`,
            rgba: sprite.rgba,
            width: sprite.width,
            x: 8 + index * 44,
            y: 10,
          }),
        );

        composition.pixelScaleLayer(sprites[0], { factor: 4 });
        const scaled = composition.getLayerRgba(sprites[0]);
        const atlasJson = JSON.parse(composition.packSpritesJson({ layerIds: sprites, maxWidth: 128, padding: 6 }));
        const atlas = composition.packSprites({ layerIds: sprites, maxWidth: 128, padding: 6 });
        const sheet = composition.contactSheet({ background: [0, 0, 0, 0], columns: 2, layerIds: sprites, padding: 8 });

        verify.equal(atlasJson.sprites.length, sprites.length, "atlas json should describe every sprite");
        verify.equal(scaled.length, 32 * 32 * 4, "pixelScaleLayer should upscale the first sprite to 32x32");
        verify.ok(atlasJson.width >= 32, "atlas width should reflect the packed sprite sizes");
        verify.ok(atlasJson.height >= 32, "atlas height should reflect the packed sprite sizes");

        return {
          assertions: verify.count,
          metrics: [
            ["Scaled sprite", "32x32"],
            ["Atlas", `${atlasJson.width}x${atlasJson.height}`],
            ["Packed entries", atlasJson.sprites.length],
            ["Contact sheet", "72x72"],
          ],
          views: [
            rgbaView("pixelScaleLayer()", scaled, 32, 32, { maxDisplay: 220 }),
            rgbaView("packSprites()", atlas, atlasJson.width, atlasJson.height, { maxDisplay: 260 }),
            rgbaView("contactSheet()", sheet, 72, 72, { maxDisplay: 220 }),
            codeView("packSpritesJson()", JSON.stringify(atlasJson, null, 2)),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "The deserialized composition should match the original exactly, and both PNG import paths should reproduce the same visual layer.",
    section: "io",
    title: "Serialization, addPngLayer, importImage",
    async run(context) {
      const verify = createVerifier();
      const source = await buildExportScene(context);

      try {
        const serialized = source.serialize();
        const roundTripped = await Composition.deserialize(serialized);

        try {
          const before = source.renderRgba();
          const after = roundTripped.renderRgba();
          const exportedPng = source.exportPng();
          const addPngComp = await Composition.create({ width: source.width, height: source.height });
          const importPngComp = await Composition.create({ width: source.width, height: source.height });

          try {
            addPngComp.addPngLayer({ name: "from-export", png: exportedPng });
            importPngComp.importImage({ bytes: context.fixture.pngBytes, name: "from-fixture" });

            verify.ok(rgbaEquals(before, after), "serialize/deserialize should preserve the render");
            verify.equal(await detectFormat(exportedPng), "png", "exportPng should detect as png");

            return {
              assertions: verify.count,
              metrics: [
                ["Serialized bytes", serialized.length.toLocaleString()],
                ["exportPng()", `${exportedPng.length.toLocaleString()} bytes`],
              ],
              views: [
                rgbaView("Original", before, source.width, source.height),
                rgbaView("deserialize()", after, roundTripped.width, roundTripped.height),
                rgbaView("addPngLayer()", addPngComp.renderRgba(), addPngComp.width, addPngComp.height),
                rgbaView("importImage()", importPngComp.renderRgba(), importPngComp.width, importPngComp.height),
              ],
            };
          } finally {
            addPngComp.free();
            importPngComp.free();
          }
        } finally {
          roundTripped.free();
        }
      } finally {
        source.free();
      }
    },
  },
  {
    expectation:
      "JPEG and WebP round trips should stay recognizably close to the source, with format detection reflecting the exported bytes.",
    section: "io",
    title: "JPEG and WebP Export / Import",
    async run(context) {
      const verify = createVerifier();
      const source = await buildExportScene(context);

      try {
        const original = source.renderRgba();
        const jpeg = source.exportJpeg({ quality: 82 });
        const webp = source.exportWebp();

        const jpegComp = await Composition.create({ width: source.width, height: source.height });
        const webpComp = await Composition.create({ width: source.width, height: source.height });

        try {
          jpegComp.importJpeg({ bytes: jpeg, name: "jpeg-roundtrip" });
          webpComp.importWebp({ bytes: webp, name: "webp-roundtrip" });

          verify.equal(await detectFormat(jpeg), "jpeg", "exportJpeg should detect as jpeg");
          verify.equal(await detectFormat(webp), "webp", "exportWebp should detect as webp");

          return {
            assertions: verify.count,
            metrics: [
              ["JPEG bytes", jpeg.length.toLocaleString()],
              ["WebP bytes", webp.length.toLocaleString()],
            ],
            views: [
              rgbaView("Original", original, source.width, source.height),
              rgbaView("importJpeg()", jpegComp.renderRgba(), jpegComp.width, jpegComp.height),
              rgbaView("importWebp()", webpComp.renderRgba(), webpComp.width, webpComp.height),
            ],
          };
        } finally {
          jpegComp.free();
          webpComp.free();
        }
      } finally {
        source.free();
      }
    },
  },
  {
    expectation:
      "A valid GIF should import as one or more layers. The card uses a tiny embedded sample and scales it up so failures are obvious.",
    section: "io",
    title: "GIF Frame Import",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 128, height: 92 });

      try {
        const gifBytes = decodeBase64("R0lGODlhAQABAIABAP8AAP///yH5BAEAAAEALAAAAAABAAEAAAICRAEAOw==");
        const format = await detectFormat(gifBytes);
        const ids = composition.importGifFrames({ bytes: gifBytes });
        composition.updateLayer(ids[0], { scaleX: 56, scaleY: 56, x: 22, y: 18 });

        verify.equal(format, "gif", "embedded sample should detect as gif");
        verify.ok(ids.length >= 1, "importGifFrames should add at least one layer");

        return {
          assertions: verify.count,
          metrics: [["detectFormat()", format], ["imported layers", ids.length]],
          views: [rgbaView("Scaled GIF frame", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "Utility outputs should agree on basic color conversions, luminance, contrast, and base64 round trips.",
    featured: true,
    fullSpan: true,
    previewMin: 250,
    section: "io",
    title: "Package Utilities and Subpaths",
    async run(context) {
      const { hexToRgb, rgbToHex, relativeLuminance, contrastRatio } = await import("#kimg/index.js");
      const verify = createVerifier();
      const rgb = await hexToRgb("#234fdd");
      const hex = await rgbToHex({ r: 35, g: 79, b: 221 });
      const luminance = await relativeLuminance("#ffffff");
      const ratio = await contrastRatio("#ffffff", "#1d1c1a");
      const base64 = rgbaToBase64(context.utilityTile.rgba);
      const roundTrip = base64ToRgba(base64);
      const readable = readableTextColor("#234fdd");

      verify.equal(hex, "#234fdd", "rgbToHex should round-trip the accent blue");
      verify.equal(rgb.length, 3, "hexToRgb should return three bytes");
      verify.ok(luminance > 0.9, "white luminance should stay high");
      verify.ok(ratio > 10, "contrast ratio should remain strong");
      verify.ok(rgbaEquals(roundTrip, context.utilityTile.rgba), "base64 round trip should preserve bytes");
      verify.equal(readable, "#ffffff", "readableTextColor should choose white on blue");

      return {
        assertions: verify.count,
        metrics: [
          ["hexToRgb()", `${rgb[0]}, ${rgb[1]}, ${rgb[2]}`],
          ["rgbToHex()", hex],
          ["relativeLuminance()", luminance.toFixed(3)],
          ["contrastRatio()", ratio.toFixed(2)],
          ["readableTextColor()", readable],
        ],
        views: [
          swatchView("Utility tile", context.utilityPalette),
          rgbaView("base64 round trip", roundTrip, context.utilityTile.width, context.utilityTile.height, { maxDisplay: 240 }),
          codeView("Base64 sample", base64.slice(0, 88) + "..."),
        ],
      };
    },
  },

  // ── New IO tests ────────────────────────────────────────────────────────────

  {
    expectation:
      "packSprites on 4 distinct shape layers should produce a JSON atlas with exactly 4 entries and a rendered RGBA atlas image. Metadata coords should be within the atlas bounds.",
    section: "io",
    title: "Pack Sprites Atlas",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 200 });

      try {
        const colors = [
          [201, 73, 45, 255],
          [35, 79, 221, 255],
          [61, 214, 140, 255],
          [240, 191, 61, 255],
        ];
        const sizes = [
          { w: 32, h: 32 },
          { w: 48, h: 24 },
          { w: 24, h: 40 },
          { w: 36, h: 36 },
        ];

        const ids = colors.map((fill, i) =>
          composition.addShapeLayer({
            fill,
            height: sizes[i].h,
            name: `icon-${i}`,
            type: "rectangle",
            width: sizes[i].w,
            x: 0,
            y: 0,
          }),
        );

        const atlasRgba = composition.packSprites({ layerIds: ids, maxWidth: 128, padding: 4 });
        const atlasJson = JSON.parse(composition.packSpritesJson({ layerIds: ids, maxWidth: 128, padding: 4 }));

        verify.equal(atlasJson.sprites.length, ids.length, "atlas should describe all 4 sprites");
        verify.ok(atlasJson.width > 0 && atlasJson.height > 0, "atlas should have non-zero dimensions");
        verify.ok(
          atlasJson.sprites.every((s) => s.x >= 0 && s.y >= 0 && s.x + s.w <= atlasJson.width && s.y + s.h <= atlasJson.height),
          "all sprite rects should fit within atlas bounds",
        );
        verify.ok(atlasRgba.length === atlasJson.width * atlasJson.height * 4, "atlas RGBA byte count should match dimensions");

        return {
          assertions: verify.count,
          metrics: [
            ["Sprites packed", atlasJson.sprites.length],
            ["Atlas size", `${atlasJson.width}x${atlasJson.height}`],
          ],
          views: [
            rgbaView("Atlas render", atlasRgba, atlasJson.width, atlasJson.height, { maxDisplay: 260 }),
            codeView("Atlas JSON", JSON.stringify(atlasJson, null, 2)),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "contactSheet on 6 sprite layers with columns=3 and padding=8 should return a flat RGBA array sized to fit all sprites in a 3-column grid with correct spacing.",
    section: "io",
    title: "Contact Sheet Grid",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 400, height: 400 });

      try {
        const sprites = context.spriteFixtures;
        const ids = sprites.map((sprite, i) =>
          composition.addImageLayer({
            height: sprite.height,
            name: `frame-${i}`,
            rgba: sprite.rgba,
            width: sprite.width,
            x: 0,
            y: 0,
          }),
        );

        // Create 6 layers by duplicating if needed
        while (ids.length < 6) {
          const src = sprites[ids.length % sprites.length];
          ids.push(composition.addImageLayer({ height: src.height, name: `frame-extra-${ids.length}`, rgba: src.rgba, width: src.width, x: 0, y: 0 }));
        }

        const cols = 3;
        const padding = 8;
        const sw = sprites[0].width;
        const sh = sprites[0].height;
        const rows = Math.ceil(6 / cols);
        const sheetW = cols * sw + (cols - 1) * padding;
        const sheetH = rows * sh + (rows - 1) * padding;

        const sheet = composition.contactSheet({ background: [14, 14, 20, 255], columns: cols, layerIds: ids.slice(0, 6), padding });

        verify.ok(sheet.length > 0, "contactSheet should return RGBA bytes");
        verify.equal(sheet.length % 4, 0, "contactSheet output should be RGBA-aligned");
        verify.equal(sheet.length, sheetW * sheetH * 4, "contactSheet byte count should match computed grid dimensions");

        return {
          assertions: verify.count,
          metrics: [
            ["Layers", 6],
            ["Columns", cols],
            ["Padding", padding],
            ["Sheet size", `${sheetW}x${sheetH}`],
          ],
          views: [rgbaView("Contact sheet (3-col)", sheet, sheetW, sheetH, { maxDisplay: 320 })],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "pixelScaleLayer with factor=4 should quadruple each pixel dimension and produce visibly hard edges. A bilinear-scaled comparison at the same output size should show smooth edges instead.",
    section: "io",
    title: "Pixel Scale vs Bilinear",
    async run(context) {
      const verify = createVerifier();

      const pixelComp = await Composition.create({ width: context.spriteFixtures[0].width, height: context.spriteFixtures[0].height });
      const bilinearComp = await Composition.create({ width: context.spriteFixtures[0].width, height: context.spriteFixtures[0].height });

      try {
        const src = context.spriteFixtures[0];
        const pixelId = pixelComp.addImageLayer({ height: src.height, name: "pixel", rgba: src.rgba, width: src.width });
        const bilinearId = bilinearComp.addImageLayer({ height: src.height, name: "bilinear", rgba: src.rgba, width: src.width });

        pixelComp.pixelScaleLayer(pixelId, { factor: 4 });
        const pixelInfo = pixelComp.getLayer(pixelId);
        const pixelRgba = pixelComp.getLayerRgba(pixelId);

        bilinearComp.updateLayer(bilinearId, { scaleX: 4, scaleY: 4 });
        const bilinearRgba = bilinearComp.getLayerRgba(bilinearId);

        verify.equal(pixelInfo?.width, src.width * 4, "pixel-scaled width should be 4× original");
        verify.equal(pixelInfo?.height, src.height * 4, "pixel-scaled height should be 4× original");
        verify.equal(pixelRgba.length, src.width * 4 * src.height * 4 * 4, "pixel-scaled RGBA should match 4× dimensions");
        verify.ok(!rgbaEquals(pixelRgba, bilinearRgba), "pixel-scale and bilinear should produce different pixel data");

        return {
          assertions: verify.count,
          metrics: [
            ["Source size", `${src.width}x${src.height}`],
            ["Pixel-scaled size", `${pixelInfo?.width}x${pixelInfo?.height}`],
            ["Factor", 4],
          ],
          views: [
            rgbaView("Pixel scale ×4 (crisp)", pixelRgba, pixelInfo?.width ?? src.width * 4, pixelInfo?.height ?? src.height * 4, { maxDisplay: 260 }),
            rgbaView("Bilinear ×4 (smooth)", bilinearRgba, src.width * 4, src.height * 4, { maxDisplay: 260 }),
          ],
        };
      } finally {
        pixelComp.free();
        bilinearComp.free();
      }
    },
  },
  {
    expectation:
      "quantizeLayer should reduce the teapot to at most 8 distinct colors. The quantized result should have fewer unique RGB values than the original and still be visually recognizable.",
    section: "io",
    title: "Quantize Layer",
    async run(context) {
      const verify = createVerifier();
      const comp = await Composition.create({ width: context.fixture.width, height: context.fixture.height });

      try {
        const id = comp.addImageLayer({ height: context.fixture.height, name: "teapot", rgba: context.fixture.rgba, width: context.fixture.width });
        const original = comp.getLayerRgba(id);
        const palette = comp.extractPalette(id, { maxColors: 8 });
        comp.quantizeLayer(id, { palette });
        const quantized = comp.getLayerRgba(id);

        const uniqueOriginal = new Set();
        const uniqueQuantized = new Set();
        for (let i = 0; i < original.length; i += 4) {
          if (original[i + 3] > 0) uniqueOriginal.add(`${original[i]},${original[i + 1]},${original[i + 2]}`);
          if (quantized[i + 3] > 0) uniqueQuantized.add(`${quantized[i]},${quantized[i + 1]},${quantized[i + 2]}`);
        }

        verify.ok(!rgbaEquals(original, quantized), "quantized layer should differ from original");
        verify.equal(palette.length, 32, "palette should contain 8 RGBA swatches");
        verify.ok(uniqueQuantized.size <= 8, "quantized image should have at most 8 unique opaque colors");
        verify.ok(uniqueQuantized.size < uniqueOriginal.size, "quantized image should have fewer unique colors than original");

        return {
          assertions: verify.count,
          metrics: [
            ["Original unique colors", uniqueOriginal.size.toLocaleString()],
            ["Quantized colors", uniqueQuantized.size],
            ["Palette colors", palette.length / 4],
            ["Max colors", 8],
          ],
          views: [
            rgbaView("Original", original, context.fixture.width, context.fixture.height, { maxDisplay: 280 }),
            rgbaView("Quantized (8 colors)", quantized, context.fixture.width, context.fixture.height, { maxDisplay: 280 }),
          ],
        };
      } finally {
        comp.free();
      }
    },
  },
  {
    expectation:
      "The layer method extractPalette and the free function extractPaletteFromRgba should return identical RGBA bytes when given the same source and maxColors.",
    section: "io",
    title: "Extract Palette: Layer vs Free Function",
    async run(context) {
      const verify = createVerifier();
      const comp = await Composition.create({ width: context.fixture.width, height: context.fixture.height });

      try {
        const id = comp.addImageLayer({ height: context.fixture.height, name: "teapot", rgba: context.fixture.rgba, width: context.fixture.width });

        const layerPalette = comp.extractPalette(id, { maxColors: 6 });
        const freePalette = await extractPaletteFromRgba(context.fixture.rgba, { height: context.fixture.height, maxColors: 6, width: context.fixture.width });

        verify.equal(layerPalette.length % 4, 0, "layer palette should be RGBA aligned");
        verify.equal(freePalette.length % 4, 0, "free palette should be RGBA aligned");
        verify.ok(layerPalette.length >= 4, "layer palette should contain at least 1 color");
        verify.ok(freePalette.length >= 4, "free palette should contain at least 1 color");
        verify.ok(rgbaEquals(layerPalette, freePalette), "layer method and free function should return identical palette bytes");

        return {
          assertions: verify.count,
          metrics: [
            ["Layer palette colors", layerPalette.length / 4],
            ["Free fn colors", freePalette.length / 4],
            ["Match", rgbaEquals(layerPalette, freePalette) ? "yes" : "no"],
          ],
          views: [
            rgbaView("Teapot source", context.fixture.rgba, context.fixture.width, context.fixture.height, { maxDisplay: 280 }),
            swatchView("Layer extractPalette()", layerPalette),
            swatchView("Free extractPaletteFromRgba()", freePalette),
          ],
        };
      } finally {
        comp.free();
      }
    },
  },
];

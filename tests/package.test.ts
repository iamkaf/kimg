import { readFileSync } from "node:fs";

import { describe, expect, test, vi } from "vitest";

import {
  clearRegisteredFonts,
  Composition,
  decodeImage,
  detectFormat,
  hexToRgb,
  loadGoogleFont,
  preload,
  registerFont,
  registeredFontCount,
  rgbToHex,
} from "../dist/index.js";
import {
  Composition as RawComposition,
  decode_image as rawDecodeImage,
  initSync,
  initTextSync,
} from "../dist/raw.js";

const wasm = readFileSync(new URL("../dist/kimg_wasm_bg.wasm", import.meta.url));
const textWasm = readFileSync(new URL("../dist/kimg_wasm_text_bg.wasm", import.meta.url));
const INTER_KIMG_WOFF2 = Uint8Array.from(
  readFileSync(new URL("./fixtures/inter-kimg.woff2", import.meta.url)),
);
const SIMPLE_SVG = `
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
    <rect x="2" y="2" width="20" height="20" rx="4" fill="#d9482b"/>
    <circle cx="12" cy="12" r="5" fill="#f2c94c"/>
  </svg>
`;

async function initBindings() {
  initSync({ module: wasm });
  await preload({ module_or_path: wasm });
}

function spritePixels(fill) {
  return new Uint8Array([
    fill[0],
    fill[1],
    fill[2],
    fill[3],
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    fill[0],
    fill[1],
    fill[2],
    fill[3],
  ]);
}

describe("main package facade", () => {
  test("preload is idempotent and create returns the JS facade", async () => {
    const [first, second] = await Promise.all([preload(), preload()]);
    expect(first).toBe(second);

    const composition = await Composition.create({ width: 2, height: 2 });

    expect(composition.width).toBe(2);
    expect(composition.height).toBe(2);
    expect("add_image_layer" in composition).toBe(false);

    composition.free();
  });

  test("composition methods normalize arguments and round-trip through serialization", async () => {
    const composition = await Composition.create({ width: 4, height: 4 });

    const backgroundId = composition.addSolidColorLayer({
      name: "background",
      color: [255, 255, 255, 255],
    });
    const spriteId = composition.addImageLayer({
      name: "sprite",
      rgba: new Uint8ClampedArray([
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
      ]),
      width: 2,
      height: 2,
    });
    const groupId = composition.addGroupLayer({ name: "group" });

    expect(backgroundId).not.toBe(spriteId);
    expect(composition.moveLayer(spriteId, { parentId: groupId, index: 0 })).toBe(true);

    composition.setLayerPosition(spriteId, { x: 1, y: 2 });
    expect(
      composition.updateLayer(spriteId, {
        opacity: 0.5,
        anchor: "center",
        flipX: true,
        rotation: 22.5,
        scaleX: 1.5,
        scaleY: 0.75,
      }),
    ).toBe(true);

    const sprite = composition.getLayer(spriteId);
    expect(sprite).toMatchObject({
      anchor: "center",
      flipX: true,
      opacity: 0.5,
      parentId: groupId,
      rotation: 22.5,
      scaleX: 1.5,
      scaleY: 0.75,
      x: 1,
      y: 2,
    });

    expect(
      composition
        .listLayers({
          parentId: groupId,
          recursive: false,
        })
        .map((layer) => layer.id),
    ).toEqual([spriteId]);

    const serialized = composition.serialize();
    const rendered = composition.renderRgba();
    const roundTrip = await Composition.deserialize(serialized);

    expect(roundTrip.width).toBe(4);
    expect(roundTrip.height).toBe(4);
    expect(roundTrip.listLayers().map((layer) => layer.name)).toEqual(
      composition.listLayers().map((layer) => layer.name),
    );
    expect(Array.from(roundTrip.renderRgba())).toEqual(Array.from(rendered));

    composition.free();
    roundTrip.free();
  });

  test("main package utility functions work against the built bindings", async () => {
    const composition = await Composition.create({ width: 2, height: 2 });
    composition.addSolidColorLayer({
      name: "background",
      color: [255, 128, 0, 255],
    });

    const png = composition.exportPng();

    expect(await detectFormat(png)).toBe("png");
    expect(Array.from(await decodeImage(png))).toEqual(Array.from(composition.renderRgba()));
    expect(Array.from(await hexToRgb("#ff8000"))).toEqual([255, 128, 0]);
    expect(await rgbToHex({ r: 255, g: 128, b: 0 })).toBe("#ff8000");

    composition.free();
  });

  test("registerFont accepts WOFF2 bytes and changes text rendering", async () => {
    await clearRegisteredFonts();

    const composition = await Composition.create({ width: 192, height: 64 });
    composition.addTextLayer({
      name: "headline",
      text: "KIMG",
      color: [17, 22, 32, 255],
      fontFamily: "Inter",
      fontSize: 28,
      lineHeight: 32,
      x: 0,
      y: 12,
    });

    const before = Array.from(composition.renderRgba());
    const loadedFaces = await registerFont({
      bytes: INTER_KIMG_WOFF2,
      family: "Inter",
      style: "normal",
      weight: 400,
    });
    const after = Array.from(composition.renderRgba());

    expect(loadedFaces).toBeGreaterThan(0);
    expect(await registeredFontCount()).toBeGreaterThan(0);
    expect(after).not.toEqual(before);

    composition.free();
    await clearRegisteredFonts();
    expect(await registeredFontCount()).toBe(0);
  });

  test("loadGoogleFont parses CSS2 and caches fetched font binaries", async () => {
    initTextSync({ module: textWasm });

    const originalFetch = globalThis.fetch;
    const processDescriptor = Object.getOwnPropertyDescriptor(globalThis, "process");
    const fontUrl =
      "https://fonts.gstatic.com/l/font?kit=UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuLyfMZ1zifLJ0KI&skey=c491285d6722e4fa&v=v20";
    const stylesheet = `@font-face {\n  font-family: 'Inter';\n  font-style: normal;\n  font-weight: 400;\n  font-display: swap;\n  src: url(${fontUrl}) format('woff2');\n}`;
    const fetchMock = vi.fn(async (input, init) => {
      const url =
        typeof input === "string" ? input : input instanceof URL ? input.href : String(input);
      if (url.startsWith("https://fonts.googleapis.com/css2?")) {
        return new Response(stylesheet, {
          headers: { "content-type": "text/css; charset=utf-8" },
          status: 200,
        });
      }

      if (url === fontUrl) {
        return new Response(INTER_KIMG_WOFF2, {
          headers: { "content-type": "font/woff2" },
          status: 200,
        });
      }

      return originalFetch(input as never, init);
    });

    await clearRegisteredFonts();
    globalThis.fetch = fetchMock as typeof globalThis.fetch;
    Object.defineProperty(globalThis, "process", {
      configurable: true,
      value: undefined,
    });
    let composition: Awaited<ReturnType<typeof Composition.create>> | null = null;

    try {
      composition = await Composition.create({ width: 192, height: 64 });
      composition.addTextLayer({
        color: [17, 22, 32, 255],
        fontFamily: "Inter",
        fontSize: 20,
        lineHeight: 24,
        name: "headline",
        text: "KIMG",
        x: 4,
        y: 12,
      });
      const before = Array.from(composition.renderRgba());

      const first = await loadGoogleFont({
        family: "Inter",
        text: "KIMG",
        weights: [400],
      });
      const second = await loadGoogleFont({
        family: "Inter",
        text: "KIMG",
        weights: [400],
      });

      expect(first.registeredFaces).toBeGreaterThan(0);
      expect(first.faces).toEqual([
        {
          family: "Inter",
          format: "woff2",
          style: "normal",
          url: fontUrl,
          weight: 400,
        },
      ]);
      expect(first.stylesheetUrl).toContain("fonts.googleapis.com/css2?");
      expect(second).toEqual(first);
      expect(await registeredFontCount()).toBeGreaterThan(0);
      expect(Array.from(composition.renderRgba())).not.toEqual(before);
      expect(fetchMock).toHaveBeenCalledTimes(3);
    } finally {
      composition?.free();
      globalThis.fetch = originalFetch;
      if (processDescriptor === undefined) {
        delete (globalThis as typeof globalThis & { process?: unknown }).process;
      } else {
        Object.defineProperty(globalThis, "process", processDescriptor);
      }
      await clearRegisteredFonts();
    }
  });

  test("decodeImage strips raw wasm width and height prefix bytes", async () => {
    await initBindings();

    const composition = await Composition.create({ width: 2, height: 2 });
    composition.addImageLayer({
      name: "sprite",
      rgba: new Uint8Array([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]),
      width: 2,
      height: 2,
    });

    const png = composition.exportPng();
    const publicDecoded = await decodeImage(png);
    const rawDecoded = rawDecodeImage(png);

    expect(rawDecoded.length).toBe(publicDecoded.length + 8);
    expect(Array.from(publicDecoded)).toEqual(Array.from(composition.renderRgba()));
    expect(Array.from(rawDecoded.slice(8))).toEqual(Array.from(publicDecoded));

    composition.free();
  });

  test("levelsLayer maps public options to the raw levels API correctly", async () => {
    await initBindings();

    const publicComposition = await Composition.create({ width: 1, height: 1 });
    const rawComposition = new RawComposition(1, 1);

    const rgba = new Uint8Array([255, 128, 0, 255]);
    const publicLayerId = publicComposition.addImageLayer({
      name: "public",
      rgba,
      width: 1,
      height: 1,
    });
    const rawLayerId = rawComposition.add_image_layer("raw", rgba, 1, 1, 0, 0);

    publicComposition.levelsLayer(publicLayerId, {
      shadows: 0.18,
      midtones: 0.64,
      highlights: 0.88,
    });
    rawComposition.levels_layer(rawLayerId, 46, 224, 0.64, 0, 255);

    expect(Array.from(publicComposition.getLayerRgba(publicLayerId))).toEqual(
      Array.from(rawComposition.get_layer_rgba(rawLayerId)),
    );

    publicComposition.free();
    rawComposition.free();
  });

  test("gradientMapLayer matches the raw gradient_map_layer call", async () => {
    await initBindings();

    const publicComposition = await Composition.create({ width: 2, height: 1 });
    const rawComposition = new RawComposition(2, 1);
    const rgba = new Uint8Array([255, 0, 0, 255, 0, 0, 255, 255]);

    const publicLayerId = publicComposition.addImageLayer({
      name: "public",
      rgba,
      width: 2,
      height: 1,
    });
    const rawLayerId = rawComposition.add_image_layer("raw", rgba, 2, 1, 0, 0);

    const stops = [
      { color: [14, 25, 73, 255], position: 0 },
      { color: [255, 224, 132, 255], position: 1 },
    ];

    publicComposition.gradientMapLayer(publicLayerId, { stops });
    rawComposition.gradient_map_layer(
      rawLayerId,
      new Uint8Array([14, 25, 73, 255, 255, 224, 132, 255]),
      new Float64Array([0, 1]),
    );

    expect(Array.from(publicComposition.getLayerRgba(publicLayerId))).toEqual(
      Array.from(rawComposition.get_layer_rgba(rawLayerId)),
    );

    publicComposition.free();
    rawComposition.free();
  });

  test("setLayerMask and inversion match raw mask operations", async () => {
    await initBindings();

    const publicComposition = await Composition.create({ width: 2, height: 1 });
    const rawComposition = new RawComposition(2, 1);
    const rgba = new Uint8Array([255, 0, 0, 255, 0, 0, 255, 255]);
    const mask = new Uint8Array([255, 255, 255, 255, 0, 0, 0, 255]);

    const publicLayerId = publicComposition.addImageLayer({
      name: "public",
      rgba,
      width: 2,
      height: 1,
    });
    const rawLayerId = rawComposition.add_image_layer("raw", rgba, 2, 1, 0, 0);

    publicComposition.setLayerMask(publicLayerId, {
      rgba: mask,
      width: 2,
      height: 1,
      inverted: true,
    });
    rawComposition.set_layer_mask(rawLayerId, mask, 2, 1);
    rawComposition.set_mask_inverted(rawLayerId, true);

    expect(publicComposition.getLayer(publicLayerId)).toMatchObject({
      hasMask: true,
      maskInverted: true,
    });
    expect(rawComposition.get_layer(rawLayerId)).toMatchObject({
      hasMask: true,
      maskInverted: true,
    });
    expect(Array.from(publicComposition.renderRgba())).toEqual(Array.from(rawComposition.render()));

    publicComposition.free();
    rawComposition.free();
  });

  test("updateLayer matches raw update_layer for transform fields", async () => {
    await initBindings();

    const publicComposition = await Composition.create({ width: 6, height: 6 });
    const rawComposition = new RawComposition(6, 6);
    const rgba = new Uint8Array([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255]);

    const publicLayerId = publicComposition.addImageLayer({
      name: "public",
      rgba,
      width: 2,
      height: 2,
    });
    const rawLayerId = rawComposition.add_image_layer("raw", rgba, 2, 2, 0, 0);

    const patch = {
      alphaLocked: true,
      anchor: "center",
      flipX: true,
      flipY: true,
      opacity: 0.75,
      rotation: 22.5,
      scaleX: 1.25,
      scaleY: 0.8,
      x: 3,
      y: 4,
    };

    expect(publicComposition.updateLayer(publicLayerId, patch)).toBe(true);
    expect(
      rawComposition.update_layer(rawLayerId, {
        anchor: "center",
        flipX: true,
        flipY: true,
        opacity: 0.75,
        rotation: 22.5,
        scaleX: 1.25,
        scaleY: 0.8,
        alphaLocked: true,
        x: 3,
        y: 4,
      }),
    ).toBe(true);

    expect(publicComposition.getLayer(publicLayerId)).toMatchObject({
      alphaLocked: true,
      anchor: "center",
      flipX: true,
      flipY: true,
      opacity: 0.75,
      rotation: 22.5,
      scaleX: 1.25,
      scaleY: 0.8,
      x: 3,
      y: 4,
    });
    expect(rawComposition.get_layer(rawLayerId)).toMatchObject({
      alphaLocked: true,
      anchor: "center",
      flipX: true,
      flipY: true,
      opacity: 0.75,
      rotation: 22.5,
      scaleX: 1.25,
      scaleY: 0.8,
      x: 3,
      y: 4,
    });
    expect(Array.from(publicComposition.renderRgba())).toEqual(Array.from(rawComposition.render()));

    publicComposition.free();
    rawComposition.free();
  });

  test("packSprites, packSpritesJson, and contactSheet match raw sprite helpers", async () => {
    await initBindings();

    const publicComposition = await Composition.create({ width: 16, height: 16 });
    const rawComposition = new RawComposition(16, 16);

    const publicLayerIds = [
      publicComposition.addImageLayer({
        name: "red",
        rgba: spritePixels([255, 0, 0, 255]),
        width: 2,
        height: 2,
        x: 1,
        y: 1,
      }),
      publicComposition.addImageLayer({
        name: "blue",
        rgba: spritePixels([0, 0, 255, 255]),
        width: 2,
        height: 2,
        x: 6,
        y: 2,
      }),
    ];
    const rawLayerIds = new Uint32Array([
      rawComposition.add_image_layer("red", spritePixels([255, 0, 0, 255]), 2, 2, 1, 1),
      rawComposition.add_image_layer("blue", spritePixels([0, 0, 255, 255]), 2, 2, 6, 2),
    ]);

    const publicAtlas = publicComposition.packSprites({
      layerIds: publicLayerIds,
      maxWidth: 16,
      padding: 1,
    });
    const rawAtlas = rawComposition.pack_sprites(rawLayerIds, 16, 1);

    expect(Array.from(publicAtlas)).toEqual(Array.from(rawAtlas));
    expect(
      JSON.parse(
        publicComposition.packSpritesJson({
          layerIds: publicLayerIds,
          maxWidth: 16,
          padding: 1,
        }),
      ),
    ).toEqual(JSON.parse(rawComposition.pack_sprites_json(rawLayerIds, 16, 1)));

    const publicSheet = publicComposition.contactSheet({
      layerIds: publicLayerIds,
      columns: 2,
      padding: 2,
      background: [255, 255, 255, 255],
    });
    const rawSheet = rawComposition.contact_sheet(rawLayerIds, 2, 2, 255, 255, 255, 255);

    expect(Array.from(publicSheet)).toEqual(Array.from(rawSheet));

    publicComposition.free();
    rawComposition.free();
  });

  test("bucket fill edits image and paint layers with tolerance controls", async () => {
    const composition = await Composition.create({ width: 4, height: 2 });
    const imageId = composition.addImageLayer({
      name: "image",
      rgba: [100, 100, 100, 128, 0, 0, 0, 255, 100, 100, 100, 140],
      width: 3,
      height: 1,
    });
    const alphaImageId = composition.addImageLayer({
      name: "alpha-aware",
      rgba: [50, 50, 50, 128, 50, 50, 50, 145],
      width: 2,
      height: 1,
      y: 1,
    });
    const paintId = composition.addPaintLayer({
      name: "paint",
      width: 2,
      height: 1,
    });
    const groupId = composition.addGroupLayer({ name: "group" });

    expect(
      composition.bucketFillLayer(imageId, {
        x: 0,
        y: 0,
        color: [0, 255, 0, 255],
        contiguous: false,
        tolerance: 12,
      }),
    ).toBe(true);
    expect(
      composition.bucketFillLayer(alphaImageId, {
        x: 0,
        y: 0,
        color: [255, 0, 0, 255],
        contiguous: false,
        tolerance: 12,
      }),
    ).toBe(true);
    expect(
      composition.bucketFillLayer(paintId, {
        x: 0,
        y: 0,
        color: [0, 0, 255, 255],
      }),
    ).toBe(true);
    expect(
      composition.bucketFillLayer(groupId, {
        x: 0,
        y: 0,
        color: [255, 255, 255, 255],
      }),
    ).toBe(false);

    const image = composition.getLayerRgba(imageId);
    expect(Array.from(image.slice(0, 4))).toEqual([0, 255, 0, 255]);
    expect(Array.from(image.slice(4, 8))).toEqual([0, 0, 0, 255]);
    expect(Array.from(image.slice(8, 12))).toEqual([0, 255, 0, 255]);

    const alphaImage = composition.getLayerRgba(alphaImageId);
    expect(Array.from(alphaImage.slice(0, 4))).toEqual([255, 0, 0, 255]);
    expect(Array.from(alphaImage.slice(4, 8))).toEqual([50, 50, 50, 145]);

    const paint = composition.getLayerRgba(paintId);
    expect(Array.from(paint.slice(0, 4))).toEqual([0, 0, 255, 255]);
    expect(Array.from(paint.slice(4, 8))).toEqual([0, 0, 255, 255]);

    composition.free();
  });

  test("paintStrokeLayer paints and erases raster layers", async () => {
    const composition = await Composition.create({ width: 12, height: 12 });
    const paintId = composition.addPaintLayer({
      name: "paint",
      width: 12,
      height: 12,
    });

    expect(
      composition.paintStrokeLayer(paintId, {
        color: [255, 0, 0, 255],
        flow: 0.9,
        hardness: 0.8,
        opacity: 1,
        points: [
          { x: 2, y: 2, pressure: 0.4 },
          { x: 9, y: 9, pressure: 1 },
        ],
        pressureOpacity: 0.3,
        pressureSize: 1,
        size: 4,
        smoothing: 0.15,
        spacing: 0.5,
      }),
    ).toBe(true);

    let painted = composition.getLayerRgba(paintId);
    expect(painted.some((value) => value !== 0)).toBe(true);

    expect(
      composition.paintStrokeLayer(paintId, {
        points: [{ x: 9, y: 9, pressure: 1 }],
        size: 3,
        tool: "erase",
      }),
    ).toBe(true);

    painted = composition.getLayerRgba(paintId);
    const alphaAtTail = painted[(9 * 12 + 9) * 4 + 3];
    expect(alphaAtTail).toBeLessThan(255);

    composition.free();
  });

  test("alpha lock constrains raster fill and paint to existing alpha", async () => {
    const composition = await Composition.create({ width: 3, height: 1 });
    const imageId = composition.addImageLayer({
      name: "alpha-image",
      rgba: new Uint8Array([10, 10, 10, 255, 10, 10, 10, 0, 10, 10, 10, 255]),
      width: 3,
      height: 1,
    });

    composition.setLayerAlphaLocked(imageId, true);
    expect(composition.getLayer(imageId)).toMatchObject({ alphaLocked: true });

    expect(
      composition.bucketFillLayer(imageId, {
        x: 0,
        y: 0,
        color: [0, 255, 0, 255],
        contiguous: false,
        tolerance: 0,
      }),
    ).toBe(true);

    expect(
      composition.paintStrokeLayer(imageId, {
        color: [255, 0, 0, 255],
        size: 3,
        points: [
          { x: 0, y: 0, pressure: 1 },
          { x: 2, y: 0, pressure: 1 },
        ],
      }),
    ).toBe(true);

    const rgba = composition.getLayerRgba(imageId);
    expect(Array.from(rgba.slice(0, 4))).toEqual([255, 0, 0, 255]);
    expect(Array.from(rgba.slice(4, 8))).toEqual([10, 10, 10, 0]);
    expect(Array.from(rgba.slice(8, 12))).toEqual([255, 0, 0, 255]);

    composition.free();
  });

  test("shape layers render and expose shape metadata through the facade", async () => {
    const composition = await Composition.create({ width: 8, height: 8 });
    const groupId = composition.addGroupLayer({ name: "group" });
    const paintId = composition.addPaintLayer({
      name: "paint",
      width: 2,
      height: 2,
    });
    const shapeId = composition.addShapeLayer({
      name: "badge",
      type: "rectangle",
      x: 1,
      y: 1,
      width: 4,
      height: 3,
      radius: 1,
      fill: [255, 0, 0, 255],
      stroke: {
        color: [255, 255, 255, 255],
        width: 1,
      },
      parentId: groupId,
    });
    expect(
      composition.updateLayer(paintId, {
        anchor: "center",
        flipY: true,
        rotation: 15,
        scaleX: 2,
        scaleY: 0.5,
      }),
    ).toBe(true);
    expect(
      composition.updateLayer(shapeId, {
        anchor: "center",
        flipX: true,
        rotation: 30,
        scaleX: 1.25,
        scaleY: 0.75,
      }),
    ).toBe(true);

    const shape = composition.getLayer(shapeId);
    expect(shape).toMatchObject({
      anchor: "center",
      flipX: true,
      kind: "shape",
      parentId: groupId,
      rotation: 30,
      scaleX: 1.25,
      scaleY: 0.75,
      shapeType: "rectangle",
      width: 4,
      height: 3,
      radius: 1,
      strokeWidth: 1,
    });
    expect(composition.getLayer(paintId)).toMatchObject({
      anchor: "center",
      flipY: true,
      kind: "raster",
      rotation: 15,
      scaleX: 2,
      scaleY: 0.5,
    });

    const rgba = composition.renderRgba();
    const pixelIndex = (1 * composition.width + 1) * 4;
    const pixel = Array.from(rgba.slice(pixelIndex, pixelIndex + 4));
    expect(pixel[0]).toBeGreaterThan(0);
    expect(pixel[3]).toBeGreaterThan(0);

    composition.free();
  });

  test("paintStrokeLayer matches the raw brush binding", async () => {
    const publicComposition = await Composition.create({ width: 12, height: 12 });
    initSync({ module: wasm });
    const rawComposition = new RawComposition(12, 12);

    try {
      const publicLayerId = publicComposition.addPaintLayer({
        name: "paint",
        width: 12,
        height: 12,
      });
      const rawLayerId = rawComposition.add_paint_layer("paint", 12, 12);
      const points = [
        { x: 1, y: 1, pressure: 0.3 },
        { x: 10, y: 5, pressure: 1 },
        { x: 6, y: 10, pressure: 0.7 },
      ];

      expect(
        publicComposition.paintStrokeLayer(publicLayerId, {
          color: [35, 79, 221, 255],
          flow: 0.85,
          hardness: 0.55,
          opacity: 0.9,
          points,
          pressureOpacity: 0.4,
          pressureSize: 1,
          size: 5,
          smoothing: 0.25,
          spacing: 0.4,
        }),
      ).toBe(true);

      expect(
        rawComposition.paint_stroke_layer(
          rawLayerId,
          new Float32Array(points.flatMap((point) => [point.x, point.y, point.pressure, 0, 0, 0])),
          6,
          35,
          79,
          221,
          255,
          5,
          0.9,
          0.85,
          0.55,
          0.4,
          0.25,
          1,
          0.4,
          0,
          0,
          false,
        ),
      ).toBe(true);

      expect(Array.from(publicComposition.getLayerRgba(publicLayerId))).toEqual(
        Array.from(rawComposition.get_layer_rgba(rawLayerId)),
      );
    } finally {
      publicComposition.free();
      rawComposition.free();
    }
  });

  test("streaming brush sessions match the raw bindings and cancel restores the raster", async () => {
    const publicComposition = await Composition.create({ width: 12, height: 12 });
    const rawComposition = new RawComposition(12, 12);

    try {
      const publicLayerId = publicComposition.addPaintLayer({
        height: 12,
        name: "paint",
        width: 12,
      });
      const rawLayerId = rawComposition.add_paint_layer("paint", 12, 12);
      const before = Array.from(publicComposition.getLayerRgba(publicLayerId));
      const chunks = [
        [
          { x: 1, y: 1, pressure: 0.3 },
          { x: 4, y: 3, pressure: 0.6 },
        ],
        [
          { x: 7, y: 6, pressure: 1 },
          { x: 10, y: 8, pressure: 0.8 },
        ],
      ];

      const publicStrokeId = publicComposition.beginBrushStroke(publicLayerId, {
        color: [201, 73, 45, 255],
        flow: 0.85,
        hardness: 0.5,
        opacity: 0.9,
        pressureOpacity: 0.2,
        pressureSize: 1,
        size: 5,
        smoothing: 0.2,
        spacing: 0.35,
      });
      const rawStrokeId = rawComposition.begin_brush_stroke(
        rawLayerId,
        201,
        73,
        45,
        255,
        5,
        0.9,
        0.85,
        0.5,
        0.35,
        0.2,
        1,
        0.2,
        0,
        0,
        false,
      );

      expect(publicStrokeId).toBeGreaterThan(0);
      expect(rawStrokeId).toBeGreaterThan(0);

      for (const chunk of chunks) {
        expect(publicComposition.pushBrushPoints(publicStrokeId, chunk)).toBe(true);
        expect(
          rawComposition.push_brush_points(
            rawStrokeId,
            new Float32Array(chunk.flatMap((point) => [point.x, point.y, point.pressure, 0, 0, 0])),
            6,
          ),
        ).toBe(true);
      }

      expect(publicComposition.endBrushStroke(publicStrokeId)).toBe(true);
      expect(rawComposition.end_brush_stroke(rawStrokeId)).toBe(true);
      expect(Array.from(publicComposition.getLayerRgba(publicLayerId))).toEqual(
        Array.from(rawComposition.get_layer_rgba(rawLayerId)),
      );

      const cancelStrokeId = publicComposition.beginBrushStroke(publicLayerId, {
        color: [35, 79, 221, 255],
        size: 4,
      });
      expect(
        publicComposition.pushBrushPoints(cancelStrokeId, [{ x: 2, y: 10, pressure: 1 }]),
      ).toBe(true);
      expect(publicComposition.cancelBrushStroke(cancelStrokeId)).toBe(true);
      expect(Array.from(publicComposition.getLayerRgba(publicLayerId))).toEqual(
        Array.from(rawComposition.get_layer_rgba(rawLayerId)),
      );
      expect(before).not.toEqual(Array.from(publicComposition.getLayerRgba(publicLayerId)));
    } finally {
      publicComposition.free();
      rawComposition.free();
    }
  });

  test("brush options support grain tips, modeler smoothing, and tilt", async () => {
    const publicComposition = await Composition.create({ width: 24, height: 24 });
    initSync({ module: wasm });
    const rawComposition = new RawComposition(24, 24);

    try {
      const publicLayerId = publicComposition.addPaintLayer({
        name: "paint",
        width: 24,
        height: 24,
      });
      const rawLayerId = rawComposition.add_paint_layer("paint", 24, 24);
      const points = [
        { x: 4, y: 6, pressure: 0.3, tiltX: 0.2, tiltY: 0.7, timeMs: 0 },
        { x: 12, y: 10, pressure: 0.8, tiltX: 0.7, tiltY: 0.2, timeMs: 16 },
        { x: 20, y: 16, pressure: 1, tiltX: 1, tiltY: 0, timeMs: 32 },
      ];

      expect(
        publicComposition.paintStrokeLayer(publicLayerId, {
          color: [35, 79, 221, 255],
          hardness: 0.55,
          points,
          pressureOpacity: 0.4,
          pressureSize: 1,
          size: 7,
          smoothing: 0.4,
          smoothingMode: "modeler",
          spacing: 0.35,
          tip: "grain",
        }),
      ).toBe(true);

      expect(
        rawComposition.paint_stroke_layer(
          rawLayerId,
          new Float32Array(
            points.flatMap((point) => [
              point.x,
              point.y,
              point.pressure,
              point.tiltX,
              point.tiltY,
              point.timeMs,
            ]),
          ),
          6,
          35,
          79,
          221,
          255,
          7,
          1,
          1,
          0.55,
          0.35,
          0.4,
          1,
          0.4,
          1,
          1,
          false,
        ),
      ).toBe(true);

      expect(Array.from(publicComposition.getLayerRgba(publicLayerId))).toEqual(
        Array.from(rawComposition.get_layer_rgba(rawLayerId)),
      );
    } finally {
      publicComposition.free();
      rawComposition.free();
    }
  });

  test("text layers render, serialize, and expose text metadata", async () => {
    const composition = await Composition.create({ width: 96, height: 32 });
    const textId = composition.addTextLayer({
      align: "center",
      boxWidth: 48,
      name: "headline",
      text: "Hi",
      color: [255, 0, 0, 255],
      fontFamily: "serif",
      fontStyle: "italic",
      fontWeight: 700,
      fontSize: 16,
      lineHeight: 18,
      letterSpacing: 1,
      wrap: "word",
      x: 4,
      y: 6,
    });

    expect(
      composition.updateLayer(textId, {
        rotation: 12,
        textConfig: {
          align: "right",
          boxWidth: 52,
          text: "Hello",
          color: [0, 0, 255, 255],
          fontFamily: "monospace",
          fontStyle: "oblique",
          fontWeight: 500,
          fontSize: 24,
          lineHeight: 28,
          letterSpacing: 2,
          wrap: "word",
        },
      }),
    ).toBe(true);

    const layer = composition.getLayer(textId);
    expect(layer).toMatchObject({
      kind: "text",
      text: "Hello",
      align: "right",
      boxWidth: 52,
      fontFamily: "monospace",
      fontStyle: "oblique",
      fontWeight: 500,
      fontSize: 24,
      lineHeight: 28,
      letterSpacing: 2,
      rotation: 12,
      wrap: "word",
    });

    const rendered = composition.renderRgba();
    expect(rendered.some((value) => value !== 0)).toBe(true);

    const roundTrip = await Composition.deserialize(composition.serialize());
    expect(roundTrip.getLayer(textId)).toMatchObject({
      kind: "text",
      text: "Hello",
      align: "right",
      boxWidth: 52,
      fontFamily: "monospace",
      fontStyle: "oblique",
      fontWeight: 500,
      fontSize: 24,
      lineHeight: 28,
      letterSpacing: 2,
      wrap: "word",
    });

    composition.free();
    roundTrip.free();
  });

  test("svg layers render, serialize, and rasterize through the facade", async () => {
    const composition = await Composition.create({ width: 48, height: 48 });
    const groupId = composition.addGroupLayer({ name: "assets" });
    const svgId = composition.addSvgLayer({
      name: "logo",
      svg: SIMPLE_SVG,
      width: 24,
      height: 24,
      x: 6,
      y: 8,
      parentId: groupId,
    });

    expect(
      composition.updateLayer(svgId, {
        anchor: "center",
        rotation: 18,
        scaleX: 1.5,
        scaleY: 0.75,
      }),
    ).toBe(true);

    const beforeRaster = composition.getLayer(svgId);
    expect(beforeRaster).toMatchObject({
      kind: "svg",
      parentId: groupId,
      width: 24,
      height: 24,
      rotation: 18,
      scaleX: 1.5,
      scaleY: 0.75,
    });

    const svgPixels = composition.getLayerRgba(svgId);
    expect(svgPixels.length).toBe(24 * 24 * 4);
    expect(svgPixels.some((value) => value !== 0)).toBe(true);

    const roundTrip = await Composition.deserialize(composition.serialize());
    expect(roundTrip.getLayer(svgId)).toMatchObject({
      kind: "svg",
      width: 24,
      height: 24,
      rotation: 18,
      scaleX: 1.5,
      scaleY: 0.75,
    });

    expect(roundTrip.rasterizeSvgLayer(svgId)).toBe(true);
    expect(roundTrip.getLayer(svgId)).toMatchObject({
      kind: "raster",
      width: 24,
      height: 24,
    });

    composition.free();
    roundTrip.free();
  });
});

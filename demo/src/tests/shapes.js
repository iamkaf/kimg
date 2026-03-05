import { Composition } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView, swatchView } from "../helpers/views.js";
import { rgbaEquals, createFillFixture } from "../helpers/context.js";
import { dominantRgbFromRgba, histogramRgba, extractPaletteFromRgba, quantizeRgba } from "#kimg/index.js";

async function buildFillVariant(context, mode) {
  const composition = await Composition.create({ width: 156, height: 116 });
  composition.addSolidColorLayer({ color: [248, 243, 232, 255], name: "paper" });
  const pattern = createFillFixture(mode);
  const layerId = composition.addImageLayer({ height: pattern.height, name: `${mode}-fill`, rgba: pattern.rgba, width: pattern.width, x: 18, y: 16 });

  if (mode === "contiguous") {
    composition.bucketFillLayer(layerId, { color: [35, 79, 221, 255], contiguous: true, tolerance: 0, x: 20, y: 20 });
  } else if (mode === "non-contiguous") {
    composition.bucketFillLayer(layerId, { color: [201, 73, 45, 255], contiguous: false, tolerance: 0, x: 20, y: 20 });
  } else {
    composition.bucketFillLayer(layerId, { color: [24, 140, 93, 255], contiguous: true, tolerance: 20, x: 20, y: 20 });
  }

  const result = { height: composition.height, rgba: composition.renderRgba(), width: composition.width };
  composition.free();
  return result;
}

export const shapeTests = [
  {
    expectation:
      "Rectangle, rounded rect, ellipse, line, and polygon layers should all appear in one composition with both fill and stroke visible.",
    section: "shapes",
    title: "Shape Layer Gallery",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 272, height: 180 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        composition.addShapeLayer({ fill: [201, 73, 45, 224], height: 52, name: "rectangle", stroke: { color: [73, 32, 24, 255], width: 3 }, type: "rectangle", width: 90, x: 18, y: 24 });
        composition.addShapeLayer({ fill: [35, 79, 221, 160], height: 52, name: "rounded", radius: 14, stroke: { color: [35, 79, 221, 255], width: 4 }, type: "rectangle", width: 90, x: 130, y: 24 });
        composition.addShapeLayer({ fill: [42, 155, 106, 140], height: 70, name: "ellipse", stroke: { color: [18, 84, 56, 255], width: 3 }, type: "ellipse", width: 70, x: 192, y: 84 });
        composition.addShapeLayer({ name: "line", stroke: { color: [14, 14, 14, 255], width: 5 }, type: "line", width: 96, height: 54, x: 20, y: 102 });
        composition.addShapeLayer({
          fill: [242, 190, 62, 210], name: "polygon",
          points: [{ x: 0, y: 46 }, { x: 36, y: 0 }, { x: 76, y: 20 }, { x: 56, y: 70 }, { x: 12, y: 78 }],
          stroke: { color: [94, 55, 7, 255], width: 3 }, type: "polygon", x: 116, y: 92,
        });

        const layers = composition.listLayers();
        verify.equal(layers.filter((l) => l.kind === "shape").length, 5, "expected five shape layers");

        return {
          assertions: verify.count,
          layers,
          metrics: [["Shape layers", 5]],
          views: [rgbaView("Shape composition", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "Contiguous fill should stay inside the island, non-contiguous fill should hit every matching island, and tolerance fill should smooth out noisy neighbors.",
    section: "shapes",
    title: "Bucket Fill Modes",
    async run(context) {
      const verify = createVerifier();
      const contiguous = await buildFillVariant(context, "contiguous");
      const nonContiguous = await buildFillVariant(context, "non-contiguous");
      const tolerance = await buildFillVariant(context, "tolerance");

      verify.ok(!rgbaEquals(contiguous.rgba, nonContiguous.rgba), "fill modes should not collapse to the same output");
      verify.ok(!rgbaEquals(nonContiguous.rgba, tolerance.rgba), "tolerance fill should differ from exact non-contiguous fill");

      return {
        assertions: verify.count,
        metrics: [["Contiguous target", "single enclosed island"], ["Non-contiguous target", "all shared color islands"], ["Tolerance", "alpha-aware match threshold"]],
        views: [
          rgbaView("Contiguous", contiguous.rgba, contiguous.width, contiguous.height),
          rgbaView("Non-contiguous", nonContiguous.rgba, nonContiguous.width, nonContiguous.height),
          rgbaView("Tolerance", tolerance.rgba, tolerance.width, tolerance.height),
        ],
      };
    },
  },
  {
    expectation:
      "The brush engine should show a hard red stroke, a blue modeler-smoothed stroke, a grainy ochre textured stroke, an eraser cut, and a side-by-side unlocked-versus-alpha-locked green brush stroke where only the locked version is clipped to the stencil islands.",
    section: "shapes",
    title: "Brush Stroke Engine",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 272, height: 156 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        composition.addShapeLayer({ fill: [236, 222, 207, 255], height: 104, name: "panel", stroke: { color: [214, 193, 171, 255], width: 2 }, type: "rectangle", width: 232, x: 20, y: 24 });
        composition.addShapeLayer({ fill: [0, 0, 0, 0], height: 84, name: "guide-a", stroke: { color: [120, 112, 101, 72], width: 2 }, type: "line", width: 80, x: 40, y: 36 });
        composition.addShapeLayer({ fill: [0, 0, 0, 0], height: 84, name: "guide-b", stroke: { color: [120, 112, 101, 72], width: 2 }, type: "line", width: 80, x: 152, y: 36 });

        const paintId = composition.addPaintLayer({ name: "brush-layer", width: 232, height: 104 });
        composition.setLayerPosition(paintId, { x: 20, y: 24 });

        const alphaLockStencil = new Uint8Array(72 * 28 * 4);
        for (let y = 0; y < 28; y++) {
          for (let x = 0; x < 72; x++) {
            const idx = (y * 72 + x) * 4;
            const inside = (x - 15) ** 2 + (y - 14) ** 2 <= 8 ** 2 || (x >= 28 && x <= 44 && y >= 9 && y <= 19) || Math.abs(x - 57) + Math.abs(y - 14) <= 8;
            alphaLockStencil[idx] = 88; alphaLockStencil[idx + 1] = 82; alphaLockStencil[idx + 2] = 92;
            alphaLockStencil[idx + 3] = inside ? 210 : 0;
          }
        }

        const unlockedId = composition.addImageLayer({ name: "alpha-unlocked", rgba: alphaLockStencil, width: 72, height: 28, x: -120, y: 92 });
        const lockedId = composition.addImageLayer({ name: "alpha-locked", rgba: alphaLockStencil, width: 72, height: 28, x: 156, y: 92 });
        composition.setLayerAlphaLocked(lockedId, true);

        const alphaLockStroke = [
          { pressure: 0.35, tiltX: -0.45, tiltY: 0.2, timeMs: 0, x: 6, y: 22 },
          { pressure: 0.55, tiltX: -0.25, tiltY: 0.4, timeMs: 18, x: 16, y: 18 },
          { pressure: 0.9, tiltX: 0.1, tiltY: 0.7, timeMs: 36, x: 30, y: 12 },
          { pressure: 0.75, tiltX: 0.3, tiltY: 0.75, timeMs: 54, x: 42, y: 10 },
          { pressure: 0.6, tiltX: 0.55, tiltY: 0.5, timeMs: 72, x: 56, y: 14 },
          { pressure: 0.42, tiltX: 0.72, tiltY: 0.25, timeMs: 90, x: 66, y: 7 },
        ];
        const hardStroke = Array.from({ length: 12 }, (_, i) => ({ pressure: 1, x: 18 + i * 16, y: 26 + Math.sin(i / 2) * 12 }));
        const softStroke = Array.from({ length: 14 }, (_, i) => ({ pressure: 0.2 + (i / 13) * 0.8, tiltX: 0.15 + (i / 13) * 0.85, tiltY: 0.8 - (i / 13) * 0.65, timeMs: i * 18, x: 32 + i * 12, y: 72 + Math.cos(i / 2.2) * 10 }));
        const grainStroke = Array.from({ length: 11 }, (_, i) => ({ pressure: 0.55 + Math.sin(i / 2.5) * 0.2, tiltX: -0.55 + i * 0.1, tiltY: 0.3, timeMs: i * 14, x: 42 + i * 15, y: 48 + Math.sin(i / 1.8) * 9 }));
        const eraseStroke = Array.from({ length: 10 }, (_, i) => ({ pressure: 1, x: 36 + i * 18, y: 14 + i * 8 }));

        verify.ok((() => {
          const sid = composition.beginBrushStroke(paintId, { color: [201, 73, 45, 255], hardness: 0.95, size: 10, spacing: 0.4 });
          const mid = Math.ceil(hardStroke.length / 2);
          return sid > 0 && composition.pushBrushPoints(sid, hardStroke.slice(0, mid)) && composition.pushBrushPoints(sid, hardStroke.slice(mid)) && composition.endBrushStroke(sid);
        })(), "streamed hard brush stroke should paint into the raster layer");

        verify.ok((() => {
          const sid = composition.beginBrushStroke(paintId, { color: [35, 79, 221, 255], flow: 0.65, hardness: 0.25, pressureOpacity: 0.35, pressureSize: 1, size: 16, smoothing: 0.2, smoothingMode: "modeler", spacing: 0.35 });
          const third = Math.ceil(softStroke.length / 3);
          return sid > 0 && composition.pushBrushPoints(sid, softStroke.slice(0, third)) && composition.pushBrushPoints(sid, softStroke.slice(third, third * 2)) && composition.pushBrushPoints(sid, softStroke.slice(third * 2)) && composition.endBrushStroke(sid);
        })(), "streamed soft pressure stroke should paint into the raster layer");

        verify.ok(composition.paintStrokeLayer(paintId, { color: [183, 132, 52, 255], hardness: 0.45, points: grainStroke, size: 13, smoothing: 0.3, smoothingMode: "modeler", spacing: 0.32, tip: "grain" }), "grain tip stroke should leave a textured path");

        verify.ok((() => {
          const sid = composition.beginBrushStroke(paintId, { size: 12, spacing: 0.3, tool: "erase" });
          return sid > 0 && composition.pushBrushPoints(sid, eraseStroke.slice(0, 4)) && composition.pushBrushPoints(sid, eraseStroke.slice(4)) && composition.endBrushStroke(sid);
        })(), "streamed eraser stroke should clear alpha from the raster layer");

        const lockedBefore = composition.getLayerRgba(lockedId);
        const beforeCancel = Array.from(composition.getLayerRgba(paintId));
        const cancelSid = composition.beginBrushStroke(paintId, { color: [201, 73, 45, 255], size: 8 });
        verify.ok(cancelSid > 0 && composition.pushBrushPoints(cancelSid, [{ pressure: 1, x: 28, y: 88 }, { pressure: 1, x: 88, y: 94 }]) && composition.cancelBrushStroke(cancelSid), "canceled streamed stroke should restore the original raster");

        verify.ok(composition.paintStrokeLayer(unlockedId, { color: [24, 176, 120, 255], flow: 0.72, hardness: 0.28, points: alphaLockStroke, size: 16, smoothing: 0.18, smoothingMode: "modeler", spacing: 0.28, tip: "grain" }), "reference raster should show the unconstrained brush stroke");
        verify.ok(composition.paintStrokeLayer(lockedId, { color: [24, 176, 120, 255], flow: 0.72, hardness: 0.28, points: alphaLockStroke, size: 16, smoothing: 0.18, smoothingMode: "modeler", spacing: 0.28, tip: "grain" }), "alpha-locked raster should accept paint inside existing opaque pixels");

        const paintLayer = composition.getLayer(paintId);
        const lockedLayer = composition.getLayer(lockedId);
        const paintRgba = composition.getLayerRgba(paintId);
        const unlockedRgba = composition.getLayerRgba(unlockedId);
        const lockedRgba = composition.getLayerRgba(lockedId);
        const alphaSamples = [paintRgba[(30 * 232 + 34) * 4 + 3], paintRgba[(70 * 232 + 52) * 4 + 3], paintRgba[(40 * 232 + 96) * 4 + 3]];

        verify.equal(paintLayer?.kind, "raster", "brush strokes should target a raster layer");
        verify.ok(alphaSamples.some((v) => v > 0), "brush layer should contain painted alpha");
        verify.equal(lockedLayer?.alphaLocked, true, "locked raster should report alpha lock");
        verify.ok(unlockedRgba[(14 * 72 + 24) * 4 + 3] > 0, "unlocked reference stroke should spill through transparent gaps");
        verify.equal(lockedRgba[(14 * 72 + 24) * 4 + 3], 0, "alpha lock should preserve transparent gaps between stencil islands");
        verify.equal(Array.from(paintRgba).join(","), beforeCancel.join(","), "canceled stroke should not leave any pixels behind");

        return {
          assertions: verify.count,
          metrics: [["Streamed / direct strokes", "3 + 2"], ["Canceled sessions", 1], ["Hard / modeler / grain / erase / alpha lock", "10px / 16px / 13px / 12px / 18px"]],
          views: [
            rgbaView("Brush composition", composition.renderRgba(), composition.width, composition.height),
            rgbaView("Paint layer only", paintRgba, 232, 104),
            rgbaView("Stencil mask", lockedBefore, 72, 28, { maxDisplay: 260 }),
            rgbaView("Same stroke unlocked", unlockedRgba, 72, 28, { maxDisplay: 260 }),
            rgbaView("Same stroke alpha-locked", lockedRgba, 72, 28, { maxDisplay: 260 }),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "Quantization should visibly flatten the teapot palette, and the extracted swatches plus dominant color facts should line up with the rendered result.",
    featured: true,
    fullSpan: true,
    previewMin: 250,
    section: "shapes",
    title: "Palette Extraction and Quantization",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: context.fixture.width, height: context.fixture.height });

      try {
        const layerId = composition.addImageLayer({ height: context.fixture.height, name: "teapot", rgba: context.fixture.rgba, width: context.fixture.width });
        const palette = composition.extractPalette(layerId, { maxColors: 8 });
        const dominant = await dominantRgbFromRgba(context.fixture.rgba, { height: context.fixture.height, width: context.fixture.width });
        const histogram = await histogramRgba(context.fixture.rgba, { height: context.fixture.height, width: context.fixture.width });
        const topPalette = await extractPaletteFromRgba(context.fixture.rgba, { height: context.fixture.height, maxColors: 8, width: context.fixture.width });

        composition.quantizeLayer(layerId, { palette });
        const quantizedLayer = composition.getLayerRgba(layerId);
        const topQuantized = await quantizeRgba(context.fixture.rgba, { height: context.fixture.height, palette: topPalette, width: context.fixture.width });

        verify.equal(palette.length, 32, "layer palette should expose 8 RGBA swatches");
        verify.equal(topPalette.length, 32, "top-level palette extraction should expose 8 RGBA swatches");
        verify.equal(dominant.length, 3, "dominantRgbFromRgba should return RGB");
        verify.equal(histogram.length, 1024, "histogramRgba should expose 4 * 256 bins");

        return {
          assertions: verify.count,
          metrics: [["Dominant RGB", `${dominant[0]}, ${dominant[1]}, ${dominant[2]}`], ["Histogram bins", histogram.length], ["Layer palette", `${palette.length / 4} colors`]],
          views: [
            rgbaView("Original", context.fixture.rgba, context.fixture.width, context.fixture.height, { maxDisplay: 300 }),
            rgbaView("quantizeLayer()", quantizedLayer, context.fixture.width, context.fixture.height, { maxDisplay: 300 }),
            rgbaView("quantizeRgba()", topQuantized, context.fixture.width, context.fixture.height, { maxDisplay: 300 }),
            swatchView("Layer palette", palette),
            swatchView("Top-level palette", topPalette),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },

  // ── New shape tests ─────────────────────────────────────────────────────────

  {
    expectation:
      "A polygon layer defined by explicit points should render as a closed filled shape with vertices at the specified coordinates.",
    section: "shapes",
    title: "Polygon with Explicit Points",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 180 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        composition.addShapeLayer({
          fill: [97, 123, 255, 200], name: "star",
          points: [{ x: 60, y: 10 }, { x: 80, y: 70 }, { x: 140, y: 70 }, { x: 90, y: 110 }, { x: 110, y: 170 }, { x: 60, y: 130 }, { x: 10, y: 170 }, { x: 30, y: 110 }, { x: 0, y: 70 }, { x: 40, y: 70 }],
          stroke: { color: [35, 79, 221, 255], width: 3 }, type: "polygon", x: 30, y: 5,
        });

        const layers = composition.listLayers();
        const polygonLayer = layers.find((l) => l.name === "star");

        verify.equal(polygonLayer?.kind, "shape", "polygon should be a shape layer");
        verify.ok(composition.renderRgba().some((v) => v !== 0), "polygon should render visible pixels");

        return {
          assertions: verify.count,
          metrics: [["Points", 10], ["Type", "polygon"]],
          views: [rgbaView("Star polygon", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "A group of four overlapping shape layers with group opacity 0.8 should render as a semi-transparent cluster where internal overlaps are composited before the opacity cut.",
    section: "shapes",
    title: "Shape Group Composition",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 240, height: 200 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const groupId = composition.addGroupLayer({ name: "shape-cluster" });

        composition.addShapeLayer({ fill: [201, 73, 45, 255], height: 80, name: "rect", type: "rectangle", width: 80, x: 20, y: 20, parentId: groupId });
        composition.addShapeLayer({ fill: [35, 79, 221, 255], height: 80, name: "ellipse", type: "ellipse", width: 80, x: 80, y: 50, parentId: groupId });
        composition.addShapeLayer({ fill: [61, 214, 140, 255], height: 80, name: "rect2", type: "rectangle", width: 80, x: 120, y: 30, parentId: groupId });
        composition.addShapeLayer({ fill: [233, 177, 14, 255], height: 80, name: "ellipse2", type: "ellipse", width: 80, x: 60, y: 100, parentId: groupId });

        composition.setLayerOpacity(groupId, 0.8);

        const withGroup = composition.renderRgba();

        verify.equal(composition.getLayer(groupId)?.opacity, 0.8, "group opacity should be 0.8");
        verify.ok(withGroup.some((v) => v !== 0), "group should render visible content");

        return {
          assertions: verify.count,
          metrics: [["Group opacity", 0.8], ["Shapes in group", 4]],
          views: [rgbaView("Group @ 0.8 opacity", withGroup, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "A line shape layer should render as a stroke between two points with the specified color and width. No fill should be visible.",
    section: "shapes",
    title: "Line Shape Layer",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 160 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const id = composition.addShapeLayer({
          name: "diagonal", stroke: { color: [201, 73, 45, 255], width: 8 },
          type: "line", width: 160, height: 120, x: 20, y: 20,
        });

        const layer = composition.getLayer(id);
        verify.equal(layer?.kind, "shape", "line should be a shape layer");
        verify.ok(composition.renderRgba().some((v) => v > 0), "line should render visible pixels");

        return {
          assertions: verify.count,
          metrics: [["Type", "line"], ["Stroke width", 8], ["Color", "accent red"]],
          views: [rgbaView("Line shape", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
];

import { Composition } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView } from "../helpers/views.js";
import { rgbaEquals } from "../helpers/context.js";

function straightLine(startX, startY, endX, endY, steps = 10) {
  return Array.from({ length: steps }, (_, i) => ({
    pressure: 1,
    x: startX + ((endX - startX) * i) / (steps - 1),
    y: startY + ((endY - startY) * i) / (steps - 1),
  }));
}

export const brushStrokeTests = [
  {
    expectation:
      "A round tip static stroke should deposit opaque pixels along the path. Alpha values at sampled stroke positions should be greater than zero.",
    section: "brushStrokes",
    title: "Round Tip Static Stroke",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 100 });

      try {
        const paintId = composition.addPaintLayer({ name: "canvas", width: 200, height: 100 });
        const points = straightLine(20, 50, 180, 50, 12);

        verify.ok(
          composition.paintStrokeLayer(paintId, { color: [201, 73, 45, 255], hardness: 0.9, points, size: 14, spacing: 0.4, tip: "round" }),
          "round tip stroke should return true",
        );

        const rgba = composition.getLayerRgba(paintId);
        const midAlpha = rgba[(50 * 200 + 100) * 4 + 3];
        verify.ok(midAlpha > 0, "midpoint of stroke should have alpha > 0");

        return {
          assertions: verify.count,
          metrics: [["Tip", "round"], ["Size", 14], ["Points", points.length], ["Mid alpha", midAlpha]],
          views: [rgbaView("Round tip stroke", rgba, 200, 100, { maxDisplay: 280 })],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "A grain tip stroke should show visible alpha variation between adjacent pixels along the path — the texture creates local irregularity that a round tip would not.",
    section: "brushStrokes",
    title: "Grain Tip Textured Stroke",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 100 });

      try {
        const paintId = composition.addPaintLayer({ name: "canvas", width: 200, height: 100 });
        const points = Array.from({ length: 12 }, (_, i) => ({ pressure: 0.7, tiltX: -0.3 + i * 0.05, tiltY: 0.4, timeMs: i * 14, x: 20 + i * 14, y: 50 + Math.sin(i) * 8 }));

        verify.ok(
          composition.paintStrokeLayer(paintId, { color: [183, 132, 52, 255], hardness: 0.4, points, size: 16, spacing: 0.28, tip: "grain" }),
          "grain stroke should return true",
        );

        const rgba = composition.getLayerRgba(paintId);
        const samples = Array.from({ length: 6 }, (_, i) => rgba[(50 * 200 + 30 + i * 24) * 4 + 3]);
        verify.ok(samples.some((a) => a > 0), "grain stroke should leave painted pixels");
        verify.ok(
          samples.some((a, _, arr) => a !== arr[0]),
          "grain tip should create local alpha variation across adjacent samples",
        );

        return {
          assertions: verify.count,
          metrics: [["Tip", "grain"], ["Size", 16], ["Alpha samples", samples.join(", ")]],
          views: [rgbaView("Grain tip stroke", rgba, 200, 100, { maxDisplay: 280 })],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "A streaming stroke committed via begin → push x3 → end should deposit pixels. The layer should have non-zero alpha at stroke positions after endBrushStroke.",
    section: "brushStrokes",
    title: "Streaming Stroke Commit",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 100 });

      try {
        const paintId = composition.addPaintLayer({ name: "canvas", width: 200, height: 100 });
        const allPoints = straightLine(20, 50, 180, 50, 15);
        const chunk = Math.ceil(allPoints.length / 3);

        const strokeId = composition.beginBrushStroke(paintId, { color: [97, 123, 255, 255], hardness: 0.85, size: 12, spacing: 0.35 });
        verify.ok(strokeId > 0, "beginBrushStroke should return a positive stroke id");

        verify.ok(composition.pushBrushPoints(strokeId, allPoints.slice(0, chunk)), "first push should succeed");
        verify.ok(composition.pushBrushPoints(strokeId, allPoints.slice(chunk, chunk * 2)), "second push should succeed");
        verify.ok(composition.pushBrushPoints(strokeId, allPoints.slice(chunk * 2)), "third push should succeed");
        verify.ok(composition.endBrushStroke(strokeId), "endBrushStroke should commit the stroke");

        const rgba = composition.getLayerRgba(paintId);
        verify.ok(rgba[(50 * 200 + 100) * 4 + 3] > 0, "midpoint should have alpha after commit");

        return {
          assertions: verify.count,
          metrics: [["Stroke id", strokeId], ["Chunks", 3], ["Total points", allPoints.length]],
          views: [rgbaView("Committed streaming stroke", rgba, 200, 100, { maxDisplay: 280 })],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "A streaming stroke canceled via cancelBrushStroke should leave the layer pixel-for-pixel identical to the state before beginBrushStroke was called.",
    section: "brushStrokes",
    title: "Streaming Stroke Cancel",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 100 });

      try {
        const paintId = composition.addPaintLayer({ name: "canvas", width: 200, height: 100 });
        composition.paintStrokeLayer(paintId, { color: [35, 79, 221, 255], hardness: 0.9, points: straightLine(10, 30, 190, 30, 10), size: 10, spacing: 0.4 });

        const beforeCancel = Array.from(composition.getLayerRgba(paintId));

        const strokeId = composition.beginBrushStroke(paintId, { color: [201, 73, 45, 255], size: 14 });
        verify.ok(strokeId > 0, "beginBrushStroke should return a positive id");
        verify.ok(composition.pushBrushPoints(strokeId, straightLine(20, 70, 180, 70, 8)), "push before cancel should succeed");
        verify.ok(composition.cancelBrushStroke(strokeId), "cancelBrushStroke should succeed");

        const afterCancel = Array.from(composition.getLayerRgba(paintId));
        verify.ok(rgbaEquals(new Uint8Array(beforeCancel), new Uint8Array(afterCancel)), "canceled stroke should not change any pixels");

        return {
          assertions: verify.count,
          metrics: [["Stroke id", strokeId], ["Pre-stroke pixels", beforeCancel.filter((_, i) => (i + 1) % 4 === 0 && beforeCancel[i] > 0).length]],
          views: [rgbaView("Layer after cancel (unchanged)", new Uint8Array(afterCancel), 200, 100, { maxDisplay: 280 })],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "Erase mode should cut transparency through previously painted opaque pixels. Sampled pixels along the erase path should have alpha of zero.",
    section: "brushStrokes",
    title: "Erase Mode",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 100 });

      try {
        const paintId = composition.addPaintLayer({ name: "canvas", width: 200, height: 100 });
        composition.bucketFillLayer(paintId, { color: [35, 79, 221, 255], x: 0, y: 0 });
        const beforeErase = composition.getLayerRgba(paintId);
        verify.ok(beforeErase[(50 * 200 + 100) * 4 + 3] > 0, "layer should be opaque before erase");

        const erasePoints = straightLine(30, 50, 170, 50, 12);
        const strokeId = composition.beginBrushStroke(paintId, { size: 16, spacing: 0.3, tool: "erase" });
        verify.ok(strokeId > 0, "erase stroke should begin");
        verify.ok(composition.pushBrushPoints(strokeId, erasePoints.slice(0, 6)), "first erase push should succeed");
        verify.ok(composition.pushBrushPoints(strokeId, erasePoints.slice(6)), "second erase push should succeed");
        verify.ok(composition.endBrushStroke(strokeId), "erase stroke should commit");

        const afterErase = composition.getLayerRgba(paintId);
        verify.equal(afterErase[(50 * 200 + 100) * 4 + 3], 0, "midpoint of erase path should be transparent");

        return {
          assertions: verify.count,
          metrics: [["Tool", "erase"], ["Erase size", 16], ["Target alpha before", beforeErase[(50 * 200 + 100) * 4 + 3]], ["Target alpha after", afterErase[(50 * 200 + 100) * 4 + 3]]],
          views: [
            rgbaView("Before erase", beforeErase, 200, 100, { maxDisplay: 280 }),
            rgbaView("After erase stroke", afterErase, 200, 100, { maxDisplay: 280 }),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "A pressure-sensitive stroke varying from 0.1 to 1.0 should produce visibly tapering stroke width. High-pressure end pixels should have more alpha coverage than the low-pressure start.",
    section: "brushStrokes",
    title: "Pressure-Sensitive Taper",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 220, height: 80 });

      try {
        const paintId = composition.addPaintLayer({ name: "canvas", width: 220, height: 80 });
        const points = Array.from({ length: 16 }, (_, i) => ({
          pressure: 0.1 + (i / 15) * 0.9,
          tiltX: -0.5 + (i / 15),
          tiltY: 0.3,
          timeMs: i * 16,
          x: 14 + i * 13,
          y: 40,
        }));

        verify.ok(
          composition.paintStrokeLayer(paintId, { color: [61, 214, 140, 255], flow: 0.8, hardness: 0.6, points, pressureOpacity: 0.7, pressureSize: 0.8, size: 18, smoothing: 0.15, smoothingMode: "modeler", spacing: 0.3 }),
          "pressure-sensitive stroke should succeed",
        );

        const rgba = composition.getLayerRgba(paintId);
        const startAlpha = rgba[(40 * 220 + 18) * 4 + 3];
        const endAlpha = rgba[(40 * 220 + 200) * 4 + 3];
        verify.ok(endAlpha > startAlpha, "high-pressure end should have more coverage than low-pressure start");

        return {
          assertions: verify.count,
          metrics: [["Pressure range", "0.1 → 1.0"], ["pressureSize", 0.8], ["pressureOpacity", 0.7], ["Start alpha", startAlpha], ["End alpha", endAlpha]],
          views: [rgbaView("Pressure taper", rgba, 220, 80, { maxDisplay: 320 })],
        };
      } finally {
        composition.free();
      }
    },
  },
];

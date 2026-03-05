import { Composition } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView } from "../helpers/views.js";

async function buildTransformSetterVariant(context, mode) {
  const composition = await Composition.create({ width: 124, height: 124 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });

  if (mode === "rotated") {
    composition.addShapeLayer({ fill: [24, 24, 24, 48], height: 2, name: "guide-h", type: "rectangle", width: 92, x: 16, y: 61 });
    composition.addShapeLayer({ fill: [24, 24, 24, 48], height: 92, name: "guide-v", type: "rectangle", width: 2, x: 61, y: 16 });
    composition.addShapeLayer({ fill: [201, 73, 45, 180], height: 10, name: "pivot-dot", type: "ellipse", width: 10, x: 57, y: 57 });
  }

  const layerId = composition.addImageLayer({
    height: context.glyph.height, name: mode,
    rgba: context.glyph.rgba, width: context.glyph.width, x: 14, y: 14,
  });

  if (mode === "baseline") composition.setLayerOpacity(layerId, 0.94);
  else if (mode === "flipX") composition.setLayerFlip(layerId, { flipX: true });
  else if (mode === "flipY") composition.setLayerFlip(layerId, { flipY: true });
  else if (mode === "rotated") {
    composition.setLayerAnchor(layerId, "center");
    composition.setLayerRotation(layerId, 34);
    composition.setLayerPosition(layerId, { x: 62, y: 62 });
  }

  const result = { height: composition.height, layer: composition.getLayer(layerId), rgba: composition.renderRgba(), width: composition.width };
  composition.free();
  return result;
}

async function buildPatchedTransformScene(context, mutate) {
  const composition = await Composition.create({ width: 248, height: 172 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const imageId = composition.addImageLayer({
    height: context.fixture.height, name: "teapot",
    rgba: context.fixture.rgba, width: context.fixture.width, x: 26, y: 14,
  });

  if (mutate) {
    composition.updateLayer(imageId, { anchor: "center", flipX: true, opacity: 0.84, rotation: 18, scaleX: 0.82, scaleY: 1.14, x: 178, y: 96 });
  }

  return {
    dispose() { composition.free(); },
    height: composition.height, layer: composition.getLayer(imageId), rgba: composition.renderRgba(), width: composition.width,
  };
}

async function mutateGlyphLayer(context, mutation) {
  const composition = await Composition.create({ width: context.glyph.width, height: context.glyph.height });
  const layerId = composition.addImageLayer({ height: context.glyph.height, name: "glyph", rgba: context.glyph.rgba, width: context.glyph.width });
  mutation(composition, layerId);
  const info = composition.getLayer(layerId);
  const result = { height: info?.height ?? context.glyph.height, rgba: composition.getLayerRgba(layerId), width: info?.width ?? context.glyph.width };
  composition.free();
  return result;
}

async function mutateBorderedGlyph(context, mutation) {
  const composition = await Composition.create({ width: context.borderedGlyph.width, height: context.borderedGlyph.height });
  const layerId = composition.addImageLayer({ height: context.borderedGlyph.height, name: "bordered-glyph", rgba: context.borderedGlyph.rgba, width: context.borderedGlyph.width });
  mutation(composition, layerId);
  const info = composition.getLayer(layerId);
  const result = { height: info?.height ?? context.borderedGlyph.height, rgba: composition.getLayerRgba(layerId), width: info?.width ?? context.borderedGlyph.width };
  composition.free();
  return result;
}

async function buildResampleVariants(context) {
  const nearest = await mutateGlyphLayer(context, (c, id) => c.resizeLayerNearest(id, { width: 144, height: 144 }));
  const bilinear = await mutateGlyphLayer(context, (c, id) => c.resizeLayerBilinear(id, { width: 144, height: 144 }));
  const lanczos = await mutateGlyphLayer(context, (c, id) => c.resizeLayerLanczos3(id, { width: 144, height: 144 }));
  const cropped = await mutateGlyphLayer(context, (c, id) => c.cropLayer(id, { height: 48, width: 48, x: 24, y: 24 }));
  const trimmed = await mutateBorderedGlyph(context, (c, id) => c.trimLayerAlpha(id));
  const rotated = await mutateGlyphLayer(context, (c, id) => c.rotateLayer(id, 33));
  return { bilinear, cropped, lanczos, nearest, rotated, trimmed };
}

export const transformTests = [
  {
    expectation:
      "Flip and rotation should be obvious because the calibration glyph is asymmetric. The rotated copy should pivot around its center, not its top-left corner.",
    section: "transforms",
    title: "Transform Setters",
    async run(context) {
      const verify = createVerifier();
      const baseline = await buildTransformSetterVariant(context, "baseline");
      const flipX = await buildTransformSetterVariant(context, "flipX");
      const flipY = await buildTransformSetterVariant(context, "flipY");
      const rotated = await buildTransformSetterVariant(context, "rotated");

      verify.equal(baseline.layer?.opacity, 0.94, "baseline opacity should update");
      verify.equal(flipX.layer?.flipX, true, "flipX should update");
      verify.equal(flipY.layer?.flipY, true, "flipY should update");
      verify.equal(rotated.layer?.anchor, "center", "anchor should become center");
      verify.equal(Math.round(rotated.layer?.rotation ?? 0), 34, "rotation should update");

      return {
        assertions: verify.count,
        metrics: [["baseline opacity", baseline.layer?.opacity ?? "n/a"], ["flipX", String(flipX.layer?.flipX)], ["flipY", String(flipY.layer?.flipY)], ["rotation", rotated.layer?.rotation?.toFixed(1) ?? "n/a"]],
        views: [
          rgbaView("Baseline", baseline.rgba, baseline.width, baseline.height, { maxDisplay: 220 }),
          rgbaView("flipX", flipX.rgba, flipX.width, flipX.height, { maxDisplay: 220 }),
          rgbaView("flipY", flipY.rgba, flipY.width, flipY.height, { maxDisplay: 220 }),
          rgbaView("anchor=center, rotation=34", rotated.rgba, rotated.width, rotated.height, { maxDisplay: 220 }),
        ],
      };
    },
  },
  {
    expectation:
      "The patched teapot should move right, shrink slightly, tilt, and mirror without needing a cascade of setter calls.",
    section: "transforms",
    title: "updateLayer Combined Patch",
    async run(context) {
      const verify = createVerifier();
      const before = await buildPatchedTransformScene(context, false);
      const after = await buildPatchedTransformScene(context, true);

      try {
        verify.equal(after.layer?.anchor, "center", "updateLayer should set anchor");
        verify.equal(after.layer?.flipX, true, "updateLayer should set flipX");
        verify.equal(after.layer?.scaleX, 0.82, "updateLayer should set scaleX");
        verify.equal(after.layer?.scaleY, 1.14, "updateLayer should set scaleY");

        return {
          assertions: verify.count,
          metrics: [["rotation", after.layer?.rotation?.toFixed(1) ?? "n/a"], ["scale", `${after.layer?.scaleX ?? "?"} x ${after.layer?.scaleY ?? "?"}`], ["position", `${after.layer?.x}, ${after.layer?.y}`]],
          views: [rgbaView("Before patch", before.rgba, before.width, before.height), rgbaView("After updateLayer()", after.rgba, after.width, after.height)],
        };
      } finally {
        before.dispose();
        after.dispose();
      }
    },
  },
  {
    expectation:
      "Nearest should stay blocky, bilinear should soften edges, Lanczos should be cleaner, crop should isolate the center, trim should remove the transparent border, and rotateLayer should grow the raster.",
    featured: true,
    fullSpan: true,
    previewMin: 250,
    section: "transforms",
    title: "Destructive Resample and Trim",
    async run(context) {
      const verify = createVerifier();
      const variants = await buildResampleVariants(context);

      verify.ok(variants.trimmed.width < context.borderedGlyph.width, "trimmed glyph should lose horizontal transparent border");
      verify.ok(variants.trimmed.height < context.borderedGlyph.height, "trimmed glyph should lose vertical transparent border");
      verify.ok(variants.rotated.width > context.glyph.width, "rotateLayer should enlarge the raster bounds");

      return {
        assertions: verify.count,
        metrics: [["nearest", `${variants.nearest.width}x${variants.nearest.height}`], ["bilinear", `${variants.bilinear.width}x${variants.bilinear.height}`], ["lanczos3", `${variants.lanczos.width}x${variants.lanczos.height}`], ["rotated", `${variants.rotated.width}x${variants.rotated.height}`]],
        views: [
          rgbaView("Nearest", variants.nearest.rgba, variants.nearest.width, variants.nearest.height, { maxDisplay: 260 }),
          rgbaView("Bilinear", variants.bilinear.rgba, variants.bilinear.width, variants.bilinear.height, { maxDisplay: 260 }),
          rgbaView("Lanczos3", variants.lanczos.rgba, variants.lanczos.width, variants.lanczos.height, { maxDisplay: 260 }),
          rgbaView("Crop", variants.cropped.rgba, variants.cropped.width, variants.cropped.height, { maxDisplay: 260 }),
          rgbaView("Trim alpha", variants.trimmed.rgba, variants.trimmed.width, variants.trimmed.height, { maxDisplay: 260 }),
          rgbaView("rotateLayer()", variants.rotated.rgba, variants.rotated.width, variants.rotated.height, { maxDisplay: 260 }),
        ],
      };
    },
  },
];

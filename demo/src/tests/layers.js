import { Composition } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView } from "../helpers/views.js";
import { rgbaEquals, tintFixture, createMaskFixture } from "../helpers/context.js";

// ── Scene builders ────────────────────────────────────────────────────────────

async function buildLayerSurgeryScene(context, mutate) {
  const composition = await Composition.create({ width: 232, height: 164 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const groupId = composition.addGroupLayer({ name: "cluster" });
  const redId = composition.addImageLayer({
    height: context.glyph.height, name: "red", parentId: groupId,
    rgba: tintFixture(context.glyph, [201, 73, 45]), width: context.glyph.width, x: 20, y: 24,
  });
  const blueId = composition.addImageLayer({
    height: context.glyph.height, name: "blue", parentId: groupId,
    rgba: tintFixture(context.glyph, [35, 79, 221]), width: context.glyph.width, x: 86, y: 42,
  });
  const yellowId = composition.addImageLayer({
    height: context.glyph.height, name: "yellow",
    rgba: tintFixture(context.glyph, [228, 181, 64]), width: context.glyph.width, x: 152, y: 34,
  });
  const ghostId = composition.addShapeLayer({
    fill: [18, 18, 18, 210], height: 12, name: "ghost", type: "rectangle", width: 120, x: 96, y: 138,
  });

  if (mutate) {
    composition.moveLayer(yellowId, { index: 1, parentId: groupId });
    composition.updateLayer(yellowId, { x: 108, y: 34 });
    composition.removeFromGroup(groupId, blueId);
    composition.removeLayer(ghostId);
    composition.flattenGroup(groupId);
    composition.resizeCanvas({ width: 296, height: 184 });
  }

  return {
    dispose() { composition.free(); },
    height: composition.height,
    layerCount: composition.layerCount(),
    rgba: composition.renderRgba(),
    width: composition.width,
  };
}

async function buildClipScene(context) {
  const fixture = context.volumeFixtures?.flower ?? context.fixture;
  const composition = await Composition.create({ width: 284, height: 188 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const imageX = Math.floor((composition.width - fixture.width) / 2);
  const imageY = Math.floor((composition.height - fixture.height) / 2);
  const clipWidth = Math.max(40, Math.round(fixture.width * 0.22));
  const clipHeight = Math.max(96, Math.round(fixture.height * 0.72));
  const clipX = imageX + Math.floor((fixture.width - clipWidth) / 2);
  const clipY = imageY + Math.floor(fixture.height * 0.14);

  composition.addShapeLayer({
    fill: [255, 255, 255, 255],
    height: clipHeight,
    name: "clip-shape",
    radius: 24,
    type: "rectangle",
    width: clipWidth,
    x: clipX,
    y: clipY,
  });

  const subjectId = composition.addImageLayer({
    height: fixture.height,
    name: "subject",
    rgba: fixture.rgba,
    width: fixture.width,
    x: imageX,
    y: imageY,
  });
  composition.addShapeLayer({
    fill: [255, 255, 255, 0],
    height: clipHeight,
    name: "outline",
    radius: 24,
    stroke: { color: [24, 24, 24, 255], width: 4 },
    type: "rectangle",
    width: clipWidth,
    x: clipX,
    y: clipY,
  });
  const unclipped = composition.renderRgba();
  composition.setLayerClipToBelow(subjectId, true);
  const clipped = composition.renderRgba();
  return {
    dispose() { composition.free(); },
    clipped,
    height: composition.height,
    layerCount: composition.layerCount(),
    unclipped,
    width: composition.width,
  };
}

async function buildMaskStates(context) {
  const fixture = context.volumeFixtures?.flower ?? context.fixture;
  const composition = await Composition.create({ width: 284, height: 188 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const imageX = Math.floor((composition.width - fixture.width) / 2);
  const imageY = Math.floor((composition.height - fixture.height) / 2);
  const imageId = composition.addImageLayer({
    height: fixture.height,
    name: "subject",
    rgba: fixture.rgba,
    width: fixture.width,
    x: imageX,
    y: imageY,
  });
  const baseline = composition.renderRgba();
  const mask = createReadableMaskFixture(fixture.width, fixture.height);
  const maskMatte = maskToMatteRgba(mask.rgba, mask.width, mask.height);
  const revealedOverlay = maskOverlayRgba(fixture.rgba, mask.rgba, mask.width, mask.height, false);
  const invertedOverlay = maskOverlayRgba(fixture.rgba, mask.rgba, mask.width, mask.height, true);
  composition.setLayerMask(imageId, { height: mask.height, rgba: mask.rgba, width: mask.width });

  const masked = { hasMask: composition.getLayer(imageId)?.hasMask, height: composition.height, rgba: composition.renderRgba(), width: composition.width };
  composition.setLayerMaskInverted(imageId, true);
  const inverted = { hasMask: composition.getLayer(imageId)?.hasMask, height: composition.height, maskInverted: composition.getLayer(imageId)?.maskInverted, rgba: composition.renderRgba(), width: composition.width };
  composition.clearLayerMask(imageId);
  const cleared = { hasMask: composition.getLayer(imageId)?.hasMask, height: composition.height, rgba: composition.renderRgba(), width: composition.width };

  composition.free();
  return {
    baseline,
    cleared,
    inverted,
    invertedOverlay,
    maskMatte,
    maskWidth: mask.width,
    maskHeight: mask.height,
    masked,
    revealedOverlay,
  };
}

function createReadableMaskFixture(width, height) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "rgba(0,0,0,1)";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "rgba(255,255,255,1)";
  ctx.beginPath();
  ctx.ellipse(width * 0.5, height * 0.37, width * 0.26, height * 0.24, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillRect(width * 0.1, height * 0.58, width * 0.8, height * 0.24);
  ctx.fillStyle = "rgba(0,0,0,1)";
  ctx.fillRect(width * 0.44, height * 0.52, width * 0.12, height * 0.36);
  const imageData = ctx.getImageData(0, 0, width, height);
  return { width, height, rgba: new Uint8Array(imageData.data) };
}

function maskToMatteRgba(maskRgba, width, height) {
  const output = new Uint8Array(width * height * 4);
  for (let i = 0; i < output.length; i += 4) {
    const value = maskRgba[i];
    output[i] = value;
    output[i + 1] = value;
    output[i + 2] = value;
    output[i + 3] = 255;
  }
  return output;
}

function maskOverlayRgba(sourceRgba, maskRgba, width, height, inverted) {
  const output = new Uint8Array(sourceRgba);
  for (let i = 0; i < output.length; i += 4) {
    if (output[i + 3] === 0) continue;
    const onMask = maskRgba[i] > 127;
    const revealed = inverted ? !onMask : onMask;
    const tint = revealed ? [61, 214, 140] : [240, 82, 82];
    const strength = revealed ? 0.32 : 0.45;
    output[i] = Math.round(output[i] * (1 - strength) + tint[0] * strength);
    output[i + 1] = Math.round(output[i + 1] * (1 - strength) + tint[1] * strength);
    output[i + 2] = Math.round(output[i + 2] * (1 - strength) + tint[2] * strength);
  }

  const edgeColor = [20, 20, 20];
  for (let y = 0; y < height - 1; y++) {
    for (let x = 0; x < width - 1; x++) {
      const idx = (y * width + x) * 4;
      const current = maskRgba[idx] > 127;
      const right = maskRgba[idx + 4] > 127;
      const down = maskRgba[idx + width * 4] > 127;
      if (current !== right || current !== down) {
        output[idx] = edgeColor[0];
        output[idx + 1] = edgeColor[1];
        output[idx + 2] = edgeColor[2];
        output[idx + 3] = 255;
      }
    }
  }
  return output;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

export const layerTests = [
  {
    expectation:
      "Background wash, teapot group, paint veil, and hidden layer handling should compose into a readable poster without the hidden bar appearing.",
    section: "layers",
    title: "Layer Stack, Visibility, and Metadata",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 280, height: 184 });

      try {
        composition.addSolidColorLayer({ color: [244, 235, 218, 255], name: "paper" });
        composition.addGradientLayer({
          direction: "diagonalDown", name: "wash",
          stops: [{ color: [255, 255, 255, 0], position: 0 }, { color: [227, 117, 69, 68], position: 1 }],
        });
        const groupId = composition.addGroupLayer({ name: "teapot-group" });
        const imageId = composition.addImageLayer({
          height: context.fixture.height, name: "teapot", parentId: groupId,
          rgba: context.fixture.rgba, width: context.fixture.width, x: 38, y: 10,
        });
        const paintId = composition.addPaintLayer({ height: 184, name: "paint-veil", width: 280 });
        composition.bucketFillLayer(paintId, { color: [24, 77, 163, 30], x: 0, y: 0 });
        composition.setLayerOpacity(paintId, 0.7);
        composition.setLayerPosition(paintId, { x: 0, y: 0 });
        const filterId = composition.addFilterLayer({ name: "group-filter", parentId: groupId });
        composition.setFilterLayerConfig(filterId, { contrast: 0.14, saturation: 0.12 });
        composition.addShapeLayer({
          fill: [38, 94, 225, 34], height: 108, name: "halo",
          stroke: { color: [38, 94, 225, 120], width: 3 }, type: "ellipse", width: 108, x: 152, y: 16,
        });
        const hiddenId = composition.addShapeLayer({
          fill: [201, 73, 45, 255], height: 18, name: "hidden-bar", type: "rectangle", width: 280, x: 0, y: 166,
        });
        composition.setLayerVisibility(hiddenId, false);

        const render = composition.renderRgba();
        const layers = composition.listLayers({ recursive: true });
        const imageLayer = composition.getLayer(imageId);

        verify.equal(composition.layerCount(), 6, "layerCount should track top-level layers");
        verify.equal(layers.length, 8, "recursive listLayers should return every layer");
        verify.equal(imageLayer?.kind, "raster", "getLayer should describe the teapot as a raster layer");
        verify.ok(
          layers.some((l) => l.name === "hidden-bar" && l.visible === false),
          "hidden layer should remain in metadata but stay invisible",
        );

        return {
          assertions: verify.count,
          layers,
          metrics: [["layerCount()", composition.layerCount()], ["group id", groupId], ["filter id", filterId], ["image anchor", imageLayer?.anchor ?? "n/a"]],
          views: [rgbaView("Poster stack", render, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "Reordering, regrouping, flattening, removal, and canvas resizing should end with a larger sheet and a single flattened cluster on the left.",
    section: "layers",
    title: "Move, Remove, Flatten, Resize",
    async run(context) {
      const verify = createVerifier();
      const before = await buildLayerSurgeryScene(context, false);
      const after = await buildLayerSurgeryScene(context, true);

      try {
        verify.equal(before.layerCount, 4, "initial top-level layer count should include paper, group, accent, and ghost");
        verify.equal(after.layerCount, 2, "flattened scene should end with two top-level layers");
        verify.equal(after.width, 296, "resizeCanvas should change the width");
        verify.equal(after.height, 184, "resizeCanvas should change the height");

        return {
          assertions: verify.count,
          metrics: [["Before", `${before.width}x${before.height}, ${before.layerCount} layers`], ["After", `${after.width}x${after.height}, ${after.layerCount} layers`]],
          views: [rgbaView("Before surgery", before.rgba, before.width, before.height), rgbaView("After regroup + flatten", after.rgba, after.width, after.height)],
        };
      } finally {
        before.dispose();
        after.dispose();
      }
    },
  },
  {
    expectation:
      "The multiply badge should darken where circles overlap, and the moved layer should sit noticeably off-center with reduced opacity.",
    section: "layers",
    title: "Blend Mode, Position, and Opacity",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 220, height: 154 });

      try {
        composition.addSolidColorLayer({ color: [249, 242, 230, 255], name: "paper" });
        composition.addShapeLayer({ fill: [226, 77, 56, 235], height: 86, name: "circle-a", type: "ellipse", width: 86, x: 28, y: 36 });
        const badgeId = composition.addShapeLayer({ fill: [48, 101, 239, 235], height: 86, name: "circle-b", type: "ellipse", width: 86, x: 84, y: 26 });

        composition.setLayerBlendMode(badgeId, "multiply");
        composition.setLayerOpacity(badgeId, 0.68);
        composition.setLayerPosition(badgeId, { x: 98, y: 30 });

        const info = composition.getLayer(badgeId);
        verify.equal(info?.blendMode, "multiply", "blend mode should update");
        verify.equal(info?.opacity, 0.68, "opacity should update");
        verify.equal(info?.x, 98, "x position should update");

        return {
          assertions: verify.count,
          metrics: [["blend mode", info?.blendMode ?? "n/a"], ["opacity", info?.opacity ?? "n/a"], ["position", `${info?.x}, ${info?.y}`]],
          views: [rgbaView("Multiply overlap", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "Unclipped vs clipToBelow should clearly differ. Mask matte and tinted reveal maps should explain exactly which flower regions survive normal vs inverted masking.",
    featured: true,
    fullSpan: true,
    previewMin: 270,
    section: "layers",
    title: "Clip and Mask Pipeline",
    async run(context) {
      const verify = createVerifier();
      const clip = await buildClipScene(context);
      const maskStates = await buildMaskStates(context);

      try {
        verify.ok(!rgbaEquals(clip.unclipped, clip.clipped), "clipToBelow should alter the subject compared with unclipped render");
        verify.equal(maskStates.cleared.hasMask, false, "clearLayerMask should remove the mask metadata");
        verify.equal(maskStates.masked.hasMask, true, "masked state should report a mask");
        verify.ok(!rgbaEquals(maskStates.baseline, maskStates.masked.rgba), "masked render should differ from baseline render");
        verify.ok(!rgbaEquals(maskStates.masked.rgba, maskStates.cleared.rgba), "masked render should differ from the cleared render");

        return {
          assertions: verify.count,
          metrics: [["fixture", context.volumeFixtures?.flower ? "flower.png" : "teapot.png"], ["clip layers", clip.layerCount], ["masked hasMask", String(maskStates.masked.hasMask)], ["inverted mask", String(maskStates.inverted.maskInverted)]],
          views: [
            rgbaView("Unclipped subject", clip.unclipped, clip.width, clip.height, { compact: true, group: "Case A · Clip-to-below", maxDisplay: 320 }),
            rgbaView("clipToBelow", clip.clipped, clip.width, clip.height, { compact: true, group: "Case A · Clip-to-below", maxDisplay: 320 }),
            rgbaView("Mask matte (white reveals)", maskStates.maskMatte, maskStates.maskWidth, maskStates.maskHeight, { compact: true, group: "Case B · Mask (normal)", maxDisplay: 320 }),
            rgbaView("Reveal map (normal mask)", maskStates.revealedOverlay, maskStates.maskWidth, maskStates.maskHeight, { compact: true, group: "Case B · Mask (normal)", maxDisplay: 320 }),
            rgbaView("Mask applied", maskStates.masked.rgba, maskStates.masked.width, maskStates.masked.height, { compact: true, group: "Case B · Mask (normal)", maxDisplay: 320 }),
            rgbaView("Reveal map (inverted mask)", maskStates.invertedOverlay, maskStates.maskWidth, maskStates.maskHeight, { compact: true, group: "Case C · Mask inverted", maxDisplay: 320 }),
            rgbaView("Mask inverted", maskStates.inverted.rgba, maskStates.inverted.width, maskStates.inverted.height, { compact: true, group: "Case C · Mask inverted", maxDisplay: 320 }),
            rgbaView("Mask cleared", maskStates.cleared.rgba, maskStates.cleared.width, maskStates.cleared.height, { compact: true, group: "Case C · Mask inverted", maxDisplay: 320 }),
          ],
        };
      } finally {
        clip.dispose();
      }
    },
  },

  // ── New tests ───────────────────────────────────────────────────────────────

  {
    expectation:
      "updateLayer should apply a multi-field patch atomically: opacity, name, position, and rotation should all reflect the new values in a single getLayer call.",
    section: "layers",
    title: "updateLayer Multi-Field Patch",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 140 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const id = composition.addImageLayer({
          height: context.glyph.height, name: "original-name",
          rgba: context.glyph.rgba, width: context.glyph.width, x: 10, y: 10,
        });

        composition.updateLayer(id, { name: "patched-name", opacity: 0.72, x: 50, y: 40, anchor: "center", rotation: 22 });
        const info = composition.getLayer(id);

        verify.equal(info?.name, "patched-name", "name should update via patch");
        verify.equal(info?.opacity, 0.72, "opacity should update via patch");
        verify.equal(info?.x, 50, "x should update via patch");
        verify.equal(info?.y, 40, "y should update via patch");
        verify.equal(info?.anchor, "center", "anchor should update via patch");

        return {
          assertions: verify.count,
          metrics: [["name", info?.name ?? "n/a"], ["opacity", info?.opacity ?? "n/a"], ["position", `${info?.x}, ${info?.y}`], ["anchor", info?.anchor ?? "n/a"], ["rotation", info?.rotation?.toFixed(1) ?? "n/a"]],
          views: [rgbaView("Patched layer", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "moveLayer should place the layer inside the target group. listLayers should show the layer under the new parent after the move.",
    section: "layers",
    title: "moveLayer Reparent",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 220, height: 160 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const groupId = composition.addGroupLayer({ name: "target-group" });
        const layerId = composition.addImageLayer({
          height: context.glyph.height, name: "nomad",
          rgba: context.glyph.rgba, width: context.glyph.width, x: 60, y: 30,
        });

        const before = composition.getLayer(layerId);
        verify.ok(before?.parentId == null, "layer should be top-level before move");

        composition.moveLayer(layerId, { index: 0, parentId: groupId });
        const after = composition.getLayer(layerId);
        verify.equal(after?.parentId, groupId, "layer should report the group as parent after move");

        const allLayers = composition.listLayers({ recursive: true });
        verify.ok(allLayers.some((l) => l.id === layerId && l.parentId === groupId), "listLayers should confirm the new parent");

        return {
          assertions: verify.count,
          metrics: [["Before parentId", before?.parentId ?? "null (top-level)"], ["After parentId", after?.parentId ?? "null"]],
          views: [rgbaView("Post-reparent render", composition.renderRgba(), composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "resizeCanvas should grow the document while preserving all layers. Existing content should still appear in the original position.",
    section: "layers",
    title: "resizeCanvas Grow",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 180, height: 120 });

      try {
        composition.addSolidColorLayer({ color: [35, 79, 221, 255], name: "paper" });
        composition.addImageLayer({ height: context.glyph.height, name: "glyph", rgba: context.glyph.rgba, width: context.glyph.width, x: 42, y: 12 });
        const before = composition.renderRgba();

        composition.resizeCanvas({ width: 280, height: 200 });
        const after = composition.renderRgba();

        verify.equal(composition.width, 280, "width should grow to 280");
        verify.equal(composition.height, 200, "height should grow to 200");
        verify.equal(after.length, 280 * 200 * 4, "render buffer should reflect new canvas size");
        verify.ok(after.length > before.length, "grown canvas should produce more pixels");

        return {
          assertions: verify.count,
          metrics: [["Before", "180x120"], ["After", `${composition.width}x${composition.height}`], ["Delta pixels", ((after.length - before.length) / 4).toLocaleString()]],
          views: [rgbaView("Grown canvas", after, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "setLayerVisibility false should remove a layer from the render while leaving it accessible via getLayer. Re-enabling should restore it.",
    section: "layers",
    title: "Visibility Toggle",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 180, height: 140 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const id = composition.addShapeLayer({ fill: [201, 73, 45, 255], height: 80, name: "target", type: "rectangle", width: 100, x: 40, y: 30 });

        const visible = composition.renderRgba();
        composition.setLayerVisibility(id, false);
        const hidden = composition.renderRgba();
        composition.setLayerVisibility(id, true);
        const restored = composition.renderRgba();

        verify.ok(!rgbaEquals(visible, hidden), "hiding layer should change render");
        verify.ok(rgbaEquals(visible, restored), "restoring visibility should restore render");
        verify.equal(composition.getLayer(id)?.visible, true, "getLayer should report visible=true after restore");

        return {
          assertions: verify.count,
          metrics: [["visible → hidden → restored", "3 renders"], ["Layer preserved in metadata", "yes"]],
          views: [
            rgbaView("Visible", visible, composition.width, composition.height, { maxDisplay: 200 }),
            rgbaView("Hidden", hidden, composition.width, composition.height, { maxDisplay: 200 }),
            rgbaView("Restored", restored, composition.width, composition.height, { maxDisplay: 200 }),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "setLayerMaskInverted should flip which region is revealed. Normal mask hides the bottom half; inverted mask should expose it instead.",
    section: "layers",
    title: "Mask Inverted Standalone",
    async run(context) {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 200, height: 160 });

      try {
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const imageId = composition.addImageLayer({
          height: context.clipPattern.height, name: "pattern",
          rgba: context.clipPattern.rgba, width: context.clipPattern.width, x: 32, y: 12,
        });
        const mask = createMaskFixture(context.clipPattern.width, context.clipPattern.height);
        composition.setLayerMask(imageId, { height: mask.height, rgba: mask.rgba, width: mask.width });

        const normal = composition.renderRgba();
        composition.setLayerMaskInverted(imageId, true);
        const inverted = composition.renderRgba();

        verify.ok(!rgbaEquals(normal, inverted), "inverting mask should change the render");
        verify.equal(composition.getLayer(imageId)?.maskInverted, true, "maskInverted should read true after set");

        return {
          assertions: verify.count,
          metrics: [["maskInverted", "true"]],
          views: [
            rgbaView("Normal mask", normal, composition.width, composition.height, { maxDisplay: 240 }),
            rgbaView("Inverted mask", inverted, composition.width, composition.height, { maxDisplay: 240 }),
          ],
        };
      } finally {
        composition.free();
      }
    },
  },
  {
    expectation:
      "setLayerAnchor center should pivot rotation around the layer center. The same rotation angle with topLeft anchor should produce a visibly different result.",
    section: "layers",
    title: "Anchor vs Rotation",
    async run(context) {
      const verify = createVerifier();

      async function buildRotated(anchorValue) {
        const comp = await Composition.create({ width: 160, height: 160 });
        comp.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const id = comp.addImageLayer({ height: context.glyph.height, name: "glyph", rgba: context.glyph.rgba, width: context.glyph.width, x: 32, y: 32 });
        comp.setLayerAnchor(id, anchorValue);
        comp.setLayerRotation(id, 45);
        const result = { rgba: comp.renderRgba(), anchor: comp.getLayer(id)?.anchor, width: comp.width, height: comp.height };
        comp.free();
        return result;
      }

      const topLeft = await buildRotated("topLeft");
      const center = await buildRotated("center");

      verify.equal(topLeft.anchor, "topLeft", "topLeft anchor should report correctly");
      verify.equal(center.anchor, "center", "center anchor should report correctly");
      verify.ok(!rgbaEquals(topLeft.rgba, center.rgba), "different anchors at same rotation should produce different renders");

      return {
        assertions: verify.count,
        metrics: [["Rotation angle", "45°"], ["topLeft pivot", "corner"], ["center pivot", "center"]],
        views: [
          rgbaView("anchor=topLeft", topLeft.rgba, topLeft.width, topLeft.height, { maxDisplay: 220 }),
          rgbaView("anchor=center", center.rgba, center.width, center.height, { maxDisplay: 220 }),
        ],
      };
    },
  },
];

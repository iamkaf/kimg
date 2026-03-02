import { describe, expect, test } from "vitest";

import { Composition, detectFormat, hexToRgb, preload, rgbToHex } from "../dist/index.js";

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
    expect(Array.from(await hexToRgb("#ff8000"))).toEqual([255, 128, 0]);
    expect(await rgbToHex({ r: 255, g: 128, b: 0 })).toBe("#ff8000");

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
      type: "roundedRect",
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
      shapeType: "roundedRect",
      width: 4,
      height: 3,
      radius: 1,
      strokeWidth: 1,
    });
    expect(composition.getLayer(paintId)).toMatchObject({
      anchor: "center",
      flipY: true,
      kind: "paint",
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
});

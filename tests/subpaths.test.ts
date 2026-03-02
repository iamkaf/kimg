import { readFileSync } from "node:fs";

import { describe, expect, test } from "vitest";

import { base64ToRgba, rgbaToBase64 } from "../dist/base64.js";
import { readableTextColor } from "../dist/color-utils.js";
import { Composition as RawComposition, initSync } from "../dist/raw.js";

describe("subpath exports", () => {
  test("base64 helpers round-trip RGBA bytes", () => {
    const rgba = new Uint8Array([255, 0, 0, 255, 0, 255, 0, 255]);

    expect(base64ToRgba(rgbaToBase64(rgba))).toEqual(rgba);
  });

  test("color utils work without explicit caller-managed init", () => {
    expect(readableTextColor("#ffffff")).toBe("#000000");
    expect(readableTextColor("#111111")).toBe("#ffffff");
  });

  test("raw subpath still supports explicit initSync", () => {
    const wasm = readFileSync(new URL("../dist/kimg_wasm_bg.wasm", import.meta.url));

    initSync({ module: wasm });

    const composition = new RawComposition(2, 2);
    composition.add_solid_color_layer("background", 0, 0, 0, 255);

    expect(composition.export_png()).toBeInstanceOf(Uint8Array);

    composition.free();
  });

  test("raw subpath exposes shape-layer entrypoints", () => {
    const wasm = readFileSync(new URL("../dist/kimg_wasm_bg.wasm", import.meta.url));

    initSync({ module: wasm });

    const composition = new RawComposition(4, 4);
    const shapeId = composition.add_shape_layer(
      "shape",
      "rectangle",
      2,
      2,
      0,
      new Uint8Array([255, 0, 0, 255]),
      new Uint8Array(),
      0,
      new Int32Array(),
      1,
      1,
    );

    const shape = composition.get_layer(shapeId) as {
      kind: string;
      shapeType: string;
      width: number;
      height: number;
    };
    expect(shape.kind).toBe("shape");
    expect(shape.shapeType).toBe("rectangle");
    expect(shape.width).toBe(2);
    expect(shape.height).toBe(2);

    composition.free();
  });
});

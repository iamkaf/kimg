import { VOLUME_RASTER_ASSETS } from "../../constants.js";
import {
  addRasterLayer,
  centeredPosition,
  createVerifier,
  createVolumeComposition,
  getRasterFixture,
  rgbaView,
} from "./shared.js";

const TRANSFORM_VARIANTS = [
  {
    id: "translate",
    label: "Translate",
    expectation:
      "The subject should shift right/up from center while preserving scale and orientation.",
    apply(composition, layerId, fixture, base) {
      composition.setLayerPosition(layerId, { x: base.x + 18, y: base.y - 10 });
    },
    assert(verify, info, fixture, base) {
      verify.equal(info?.x, base.x + 18, "translate variant should update x");
      verify.equal(info?.y, base.y - 10, "translate variant should update y");
    },
    metrics(info) {
      return [["x,y", `${info?.x ?? "?"}, ${info?.y ?? "?"}`]];
    },
  },
  {
    id: "rotate-center",
    label: "Rotate Center 17°",
    expectation:
      "Rotation should pivot around center, not top-left, and the frame should keep the subject readable.",
    apply(composition, layerId, fixture, base) {
      const centerX = Math.round(base.x + fixture.width / 2);
      const centerY = Math.round(base.y + fixture.height / 2);
      composition.updateLayer(layerId, { anchor: "center", rotation: 17, x: centerX, y: centerY });
    },
    assert(verify, info) {
      verify.equal(info?.anchor, "center", "rotate-center variant should use center anchor");
      verify.equal(Math.round(info?.rotation ?? 0), 17, "rotate-center variant should set rotation");
    },
    metrics(info) {
      return [
        ["anchor", info?.anchor ?? "n/a"],
        ["rotation", info?.rotation?.toFixed(1) ?? "n/a"],
      ];
    },
  },
  {
    id: "flip-x",
    label: "Flip X",
    expectation: "Horizontal orientation should invert while preserving layer bounds.",
    apply(composition, layerId) {
      composition.setLayerFlip(layerId, { flipX: true });
    },
    assert(verify, info) {
      verify.equal(info?.flipX, true, "flip-x variant should set flipX");
      verify.equal(info?.flipY, false, "flip-x variant should not set flipY");
    },
    metrics(info) {
      return [["flip", `x=${String(info?.flipX)} y=${String(info?.flipY)}`]];
    },
  },
  {
    id: "flip-y",
    label: "Flip Y",
    expectation: "Vertical orientation should invert while preserving layer bounds.",
    apply(composition, layerId) {
      composition.setLayerFlip(layerId, { flipY: true });
    },
    assert(verify, info) {
      verify.equal(info?.flipY, true, "flip-y variant should set flipY");
      verify.equal(info?.flipX, false, "flip-y variant should not set flipX");
    },
    metrics(info) {
      return [["flip", `x=${String(info?.flipX)} y=${String(info?.flipY)}`]];
    },
  },
  {
    id: "scale-down",
    label: "Scale 0.72",
    expectation: "Scaled-down raster should stay centered and visibly smaller than the frame.",
    apply(composition, layerId) {
      composition.updateLayer(layerId, { anchor: "center", scaleX: 0.72, scaleY: 0.72 });
    },
    assert(verify, info) {
      verify.equal(info?.anchor, "center", "scale-down variant should use center anchor");
      verify.equal(info?.scaleX, 0.72, "scale-down variant should set scaleX");
      verify.equal(info?.scaleY, 0.72, "scale-down variant should set scaleY");
    },
    metrics(info) {
      return [["scale", `${info?.scaleX ?? "?"} × ${info?.scaleY ?? "?"}`]];
    },
  },
  {
    id: "scale-up",
    label: "Scale 1.18",
    expectation: "Scaled-up raster should grow beyond its original footprint without clipping artifacts.",
    apply(composition, layerId) {
      composition.updateLayer(layerId, { anchor: "center", scaleX: 1.18, scaleY: 1.18 });
    },
    assert(verify, info) {
      verify.equal(info?.anchor, "center", "scale-up variant should use center anchor");
      verify.equal(info?.scaleX, 1.18, "scale-up variant should set scaleX");
      verify.equal(info?.scaleY, 1.18, "scale-up variant should set scaleY");
    },
    metrics(info) {
      return [["scale", `${info?.scaleX ?? "?"} × ${info?.scaleY ?? "?"}`]];
    },
  },
  {
    id: "rotate-negative",
    label: "Rotate -24°",
    expectation: "Negative rotation should be obvious on asymmetric details.",
    apply(composition, layerId, fixture, base) {
      const centerX = Math.round(base.x + fixture.width / 2);
      const centerY = Math.round(base.y + fixture.height / 2);
      composition.updateLayer(layerId, { anchor: "center", rotation: -24, x: centerX, y: centerY });
    },
    assert(verify, info) {
      verify.equal(info?.anchor, "center", "rotate-negative variant should use center anchor");
      verify.equal(Math.round(info?.rotation ?? 0), -24, "rotate-negative variant should set rotation");
    },
    metrics(info) {
      return [["rotation", info?.rotation?.toFixed(1) ?? "n/a"]];
    },
  },
  {
    id: "combo",
    label: "Combo Flip + Rotate + Opacity",
    expectation:
      "Combined transform patch should produce a visibly mirrored and rotated result with reduced opacity.",
    apply(composition, layerId, fixture, base) {
      const centerX = Math.round(base.x + fixture.width / 2);
      const centerY = Math.round(base.y + fixture.height / 2);
      composition.updateLayer(layerId, {
        anchor: "center",
        flipX: true,
        opacity: 0.7,
        rotation: 13,
        x: centerX,
        y: centerY,
      });
    },
    assert(verify, info) {
      verify.equal(info?.flipX, true, "combo variant should set flipX");
      verify.equal(info?.opacity, 0.7, "combo variant should set opacity");
      verify.equal(Math.round(info?.rotation ?? 0), 13, "combo variant should set rotation");
    },
    metrics(info) {
      return [
        ["flipX", String(info?.flipX)],
        ["opacity", info?.opacity ?? "n/a"],
        ["rotation", info?.rotation?.toFixed(1) ?? "n/a"],
      ];
    },
  },
];

export const volumeTransformTests = VOLUME_RASTER_ASSETS.flatMap((asset) =>
  TRANSFORM_VARIANTS.map((variant) => ({
    expectation: `${asset.label}. ${variant.expectation}`,
    section: "volume",
    title: `Volume Transform · ${asset.label} · ${variant.label}`,
    async run(context) {
      const verify = createVerifier();
      const composition = await createVolumeComposition();
      const fixture = getRasterFixture(context, asset.key);

      try {
        composition.addShapeLayer({
          fill: [0, 0, 0, 0],
          height: composition.height - 18,
          name: "frame",
          stroke: { color: [25, 25, 25, 55], width: 2 },
          type: "rectangle",
          width: composition.width - 18,
          x: 9,
          y: 9,
        });

        const base = centeredPosition(composition.width, composition.height, fixture);
        const layerId = addRasterLayer(composition, fixture, `${asset.key}-${variant.id}`, base.x, base.y);

        variant.apply(composition, layerId, fixture, base);
        const info = composition.getLayer(layerId);
        const render = composition.renderRgba();

        verify.equal(info?.kind, "raster", "volume transform matrix should keep raster layers");
        variant.assert(verify, info, fixture, base);
        verify.ok(render.some((value) => value !== 0), "volume transform matrix should render visible bytes");

        return {
          assertions: verify.count,
          metrics: [
            ["asset", `${fixture.originalWidth}×${fixture.originalHeight} -> ${fixture.width}×${fixture.height}`],
            ["variant", variant.id],
            ...variant.metrics(info),
          ],
          views: [rgbaView(`${asset.label} · ${variant.label}`, render, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  })),
);

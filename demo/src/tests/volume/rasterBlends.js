import { VOLUME_RASTER_ASSETS } from "../../constants.js";
import {
  addRasterLayer,
  centeredPosition,
  createVerifier,
  createVolumeComposition,
  getRasterFixture,
  rgbaView,
} from "./shared.js";

const BLEND_MODES = [
  "multiply",
  "screen",
  "overlay",
  "softLight",
  "hardLight",
  "difference",
  "colorBurn",
  "colorDodge",
];

const OPACITY_BY_MODE = {
  colorBurn: 0.62,
  colorDodge: 0.62,
  difference: 0.7,
  hardLight: 0.68,
  multiply: 0.72,
  overlay: 0.7,
  screen: 0.7,
  softLight: 0.72,
};

function normalizeBlend(value) {
  return String(value)
    .replace(/([a-z])([A-Z])/g, "$1-$2")
    .toLowerCase();
}

export const volumeBlendTests = VOLUME_RASTER_ASSETS.flatMap((baseAsset, index) => {
  const overlayAsset = VOLUME_RASTER_ASSETS[(index + 1) % VOLUME_RASTER_ASSETS.length];
  return BLEND_MODES.map((blendMode) => ({
    expectation:
      `${baseAsset.label} base with ${overlayAsset.label} overlay. ` +
      `Blend mode ${blendMode} should produce a distinct overlap look while preserving both inputs.`,
    section: "volume",
    title: `Volume Blend · ${baseAsset.label} x ${overlayAsset.label} · ${blendMode}`,
    async run(context) {
      const verify = createVerifier();
      const composition = await createVolumeComposition(228, 164);
      const baseFixture = getRasterFixture(context, baseAsset.key);
      const overlayFixture = getRasterFixture(context, overlayAsset.key);

      try {
        const base = centeredPosition(composition.width, composition.height, baseFixture);
        const baseId = addRasterLayer(composition, baseFixture, `${baseAsset.key}-base`, base.x - 16, base.y);

        const overlayX = base.x + Math.floor(baseFixture.width * 0.3);
        const overlayY = base.y + Math.floor(baseFixture.height * 0.08);
        const overlayId = addRasterLayer(
          composition,
          overlayFixture,
          `${overlayAsset.key}-overlay`,
          overlayX,
          overlayY,
        );

        composition.updateLayer(overlayId, {
          anchor: "center",
          opacity: OPACITY_BY_MODE[blendMode] ?? 0.7,
          rotation: 8,
          scaleX: 0.86,
          scaleY: 0.86,
        });
        composition.setLayerBlendMode(overlayId, blendMode);

        const baseInfo = composition.getLayer(baseId);
        const overlayInfo = composition.getLayer(overlayId);
        const render = composition.renderRgba();

        verify.equal(baseInfo?.kind, "raster", "base layer should stay raster");
        verify.equal(
          normalizeBlend(overlayInfo?.blendMode),
          normalizeBlend(blendMode),
          "overlay blend mode should match matrix mode",
        );
        verify.ok(render.some((value) => value !== 0), "volume blend matrix should render visible bytes");

        return {
          assertions: verify.count,
          metrics: [
            ["base", `${baseFixture.width}×${baseFixture.height}`],
            ["overlay", `${overlayFixture.width}×${overlayFixture.height}`],
            ["blend", overlayInfo?.blendMode ?? "n/a"],
            ["opacity", overlayInfo?.opacity ?? "n/a"],
          ],
          views: [rgbaView(`${baseAsset.label} + ${overlayAsset.label} · ${blendMode}`, render, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  }));
});

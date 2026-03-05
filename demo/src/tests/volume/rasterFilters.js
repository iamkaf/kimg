import { VOLUME_RASTER_ASSETS } from "../../constants.js";
import { rgbaEquals } from "../../helpers/context.js";
import {
  addRasterLayer,
  centeredPosition,
  createVerifier,
  createVolumeComposition,
  getRasterFixture,
  rgbaView,
} from "./shared.js";

const FILTER_VARIANTS = [
  {
    id: "boxBlur-r3",
    label: "boxBlur(3)",
    expectation: "Edges should soften with visible blur.",
    apply(composition, layerId) {
      composition.boxBlurLayer(layerId, { radius: 3 });
    },
  },
  {
    id: "gaussianBlur-r3",
    label: "gaussianBlur(3)",
    expectation: "Blur should look smoother than box blur.",
    apply(composition, layerId) {
      composition.gaussianBlurLayer(layerId, { radius: 3 });
    },
  },
  {
    id: "sharpen",
    label: "sharpen()",
    expectation: "Edges should gain local contrast.",
    apply(composition, layerId) {
      composition.sharpenLayer(layerId);
    },
  },
  {
    id: "edgeDetect",
    label: "edgeDetect()",
    expectation: "High-contrast edge outlines should dominate.",
    apply(composition, layerId) {
      composition.edgeDetectLayer(layerId);
    },
  },
  {
    id: "emboss",
    label: "emboss()",
    expectation: "Relief-style shading should appear.",
    apply(composition, layerId) {
      composition.embossLayer(layerId);
    },
  },
  {
    id: "invert",
    label: "invert()",
    expectation: "Colors should invert to complements.",
    apply(composition, layerId) {
      composition.invertLayer(layerId);
    },
  },
  {
    id: "posterize-4",
    label: "posterize(4)",
    expectation: "Tonal steps should collapse to fewer bands.",
    apply(composition, layerId) {
      composition.posterizeLayer(layerId, { levels: 4 });
    },
  },
  {
    id: "threshold-140",
    label: "threshold(140)",
    expectation: "Image should reduce to hard black/white regions.",
    apply(composition, layerId) {
      composition.thresholdLayer(layerId, { threshold: 140 });
    },
  },
  {
    id: "levels-punch",
    label: "levels(punch)",
    expectation: "Contrast should increase with compressed mids.",
    apply(composition, layerId) {
      composition.levelsLayer(layerId, {
        shadows: 0.1,
        midtones: 0.64,
        highlights: 0.9,
      });
    },
  },
  {
    id: "gradientMap-coolwarm",
    label: "gradientMap(coolwarm)",
    expectation: "Luminance should remap into cool/warm ramp.",
    apply(composition, layerId) {
      composition.gradientMapLayer(layerId, {
        stops: [
          { color: [10, 22, 58, 255], position: 0 },
          { color: [64, 133, 255, 255], position: 0.55 },
          { color: [255, 219, 138, 255], position: 1 },
        ],
      });
    },
  },
];

function countChangedPixels(before, after) {
  let changed = 0;
  for (let i = 0; i < before.length; i += 4) {
    if (
      before[i] !== after[i] ||
      before[i + 1] !== after[i + 1] ||
      before[i + 2] !== after[i + 2] ||
      before[i + 3] !== after[i + 3]
    ) {
      changed += 1;
    }
  }
  return changed;
}

export const volumeFilterTests = VOLUME_RASTER_ASSETS.flatMap((asset) =>
  FILTER_VARIANTS.map((variant) => ({
    expectation: `${asset.label}. ${variant.expectation}`,
    section: "volume",
    title: `Volume Filter · ${asset.label} · ${variant.label}`,
    async run(context) {
      const verify = createVerifier();
      const composition = await createVolumeComposition();
      const fixture = getRasterFixture(context, asset.key);

      try {
        const base = centeredPosition(composition.width, composition.height, fixture);
        const layerId = addRasterLayer(composition, fixture, `${asset.key}-${variant.id}`, base.x, base.y);
        const before = composition.getLayerRgba(layerId);

        variant.apply(composition, layerId);
        const after = composition.getLayerRgba(layerId);
        const changedPixels = countChangedPixels(before, after);
        const render = composition.renderRgba();

        verify.equal(before.length, after.length, "destructive filter output should preserve buffer length");
        verify.ok(!rgbaEquals(before, after), "destructive filter should alter pixel bytes");
        verify.ok(changedPixels > Math.max(16, Math.floor(before.length / 64)), "destructive filter should alter a meaningful region");
        verify.ok(render.some((value) => value !== 0), "volume filter matrix should render visible bytes");

        return {
          assertions: verify.count,
          metrics: [
            ["asset", `${fixture.width}×${fixture.height}`],
            ["variant", variant.id],
            ["changed pixels", changedPixels.toLocaleString()],
          ],
          views: [rgbaView(`${asset.label} · ${variant.label}`, render, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  })),
);

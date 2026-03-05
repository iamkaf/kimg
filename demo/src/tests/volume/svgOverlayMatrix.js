import { VOLUME_RASTER_ASSETS, VOLUME_SVG_ASSETS } from "../../constants.js";
import {
  addRasterLayer,
  centeredPosition,
  createVerifier,
  createVolumeComposition,
  getRasterFixture,
  getSvgFixture,
  rgbaView,
} from "./shared.js";

const SVG_VARIANTS = [
  {
    id: "retained",
    label: "retained",
    expectation:
      "SVG should stay vector-backed and scale/rotate cleanly over the raster base.",
    rasterize: false,
  },
  {
    id: "rasterized",
    label: "rasterized",
    expectation:
      "After rasterization, the same transform should hold while kind switches to raster.",
    rasterize: true,
  },
];

const SVG_ASSET_KEY = VOLUME_SVG_ASSETS[0].key;

export const volumeSvgOverlayTests = VOLUME_RASTER_ASSETS.flatMap((baseAsset, index) =>
  SVG_VARIANTS.map((variant) => ({
    expectation: `${baseAsset.label} background with croissant SVG overlay. ${variant.expectation}`,
    section: "volume",
    title: `Volume SVG · ${baseAsset.label} · ${variant.label}`,
    async run(context) {
      const verify = createVerifier();
      const composition = await createVolumeComposition(236, 168);
      const baseFixture = getRasterFixture(context, baseAsset.key);
      const svgFixture = getSvgFixture(context, SVG_ASSET_KEY);

      try {
        const base = centeredPosition(composition.width, composition.height, baseFixture);
        addRasterLayer(composition, baseFixture, `${baseAsset.key}-base`, base.x, base.y);

        const layerId = composition.addSvgLayer({
          name: `${svgFixture.key}-${variant.id}`,
          svg: svgFixture.svg,
          width: 104,
          height: 104,
          x: Math.floor(composition.width * 0.62),
          y: Math.floor(composition.height * 0.54),
        });

        composition.updateLayer(layerId, {
          anchor: "center",
          opacity: 0.86,
          rotation: index % 2 === 0 ? 14 : -14,
          scaleX: variant.rasterize ? 1.04 : 1.12,
          scaleY: variant.rasterize ? 1.04 : 1.12,
        });
        composition.setLayerBlendMode(layerId, "overlay");

        if (variant.rasterize) {
          verify.equal(composition.rasterizeSvgLayer(layerId), true, "rasterized variant should rasterize successfully");
        }

        const info = composition.getLayer(layerId);
        const render = composition.renderRgba();

        verify.equal(
          info?.kind,
          variant.rasterize ? "raster" : "svg",
          "svg matrix should report expected layer kind",
        );
        verify.equal(info?.blendMode, "overlay", "svg matrix should keep overlay blend");
        verify.ok(render.some((value) => value !== 0), "svg matrix should render visible bytes");

        return {
          assertions: verify.count,
          metrics: [
            ["base", `${baseFixture.width}×${baseFixture.height}`],
            ["svg mode", variant.label],
            ["kind", info?.kind ?? "n/a"],
            ["rotation", info?.rotation?.toFixed(1) ?? "n/a"],
          ],
          views: [rgbaView(`${baseAsset.label} · ${variant.label}`, render, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  })),
);

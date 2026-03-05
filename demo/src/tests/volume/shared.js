import { Composition } from "#kimg/index.js";
import { VOLUME_RASTER_ASSETS, VOLUME_SVG_ASSETS } from "../../constants.js";
import { createVerifier } from "../../helpers/verifier.js";
import { rgbaView } from "../../helpers/views.js";

const PAPER = [247, 241, 232, 255];

export async function createVolumeComposition(width = 216, height = 156) {
  const composition = await Composition.create({ width, height });
  composition.addSolidColorLayer({ color: PAPER, name: "paper" });
  return composition;
}

export function getRasterAssetSpec(assetKey) {
  const spec = VOLUME_RASTER_ASSETS.find((entry) => entry.key === assetKey);
  if (!spec) throw new Error(`Unknown raster asset spec: ${assetKey}`);
  return spec;
}

export function getSvgAssetSpec(assetKey) {
  const spec = VOLUME_SVG_ASSETS.find((entry) => entry.key === assetKey);
  if (!spec) throw new Error(`Unknown SVG asset spec: ${assetKey}`);
  return spec;
}

export function getRasterFixture(context, assetKey) {
  const fixture = context.volumeFixtures?.[assetKey];
  if (!fixture) throw new Error(`Missing loaded raster fixture: ${assetKey}`);
  return fixture;
}

export function getSvgFixture(context, assetKey) {
  const fixture = context.volumeSvgFixtures?.[assetKey];
  if (!fixture) throw new Error(`Missing loaded SVG fixture: ${assetKey}`);
  return fixture;
}

export function centeredPosition(canvasWidth, canvasHeight, fixture) {
  return {
    x: Math.floor((canvasWidth - fixture.width) / 2),
    y: Math.floor((canvasHeight - fixture.height) / 2),
  };
}

export function addRasterLayer(composition, fixture, name, x, y) {
  return composition.addImageLayer({
    name,
    rgba: fixture.rgba,
    width: fixture.width,
    height: fixture.height,
    x,
    y,
  });
}

export { createVerifier, rgbaView };

import { volumeTransformTests } from "./rasterTransforms.js";
import { volumeFilterTests } from "./rasterFilters.js";
import { volumeBlendTests } from "./rasterBlends.js";
import { volumeSvgOverlayTests } from "./svgOverlayMatrix.js";

export const volumeTests = [
  ...volumeTransformTests,
  ...volumeFilterTests,
  ...volumeBlendTests,
  ...volumeSvgOverlayTests,
];

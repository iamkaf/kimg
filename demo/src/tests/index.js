import { setupTests } from "./setup.js";
import { layerTests } from "./layers.js";
import { transformTests } from "./transforms.js";
import { filterTests } from "./filters.js";
import { shapeTests } from "./shapes.js";
import { textTests } from "./text.js";
import { brushStrokeTests } from "./brushStrokes.js";
import { colorUtilTests } from "./colorUtils.js";
import { ioTests } from "./io.js";
import { volumeTests } from "./volume/index.js";
import { experimentalTests } from "./experimental.js";

export function createTests() {
  return [
    ...setupTests,
    ...layerTests,
    ...transformTests,
    ...filterTests,
    ...shapeTests,
    ...textTests,
    ...brushStrokeTests,
    ...colorUtilTests,
    ...ioTests,
    ...volumeTests,
    ...experimentalTests,
  ];
}

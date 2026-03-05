import { Composition, detectFormat, decodeImage } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView } from "../helpers/views.js";

export const setupTests = [
  {
    expectation:
      "Teapot fixture should appear intact, the calibration glyph should stay asymmetric, and PNG decode utilities should agree on byte length.",
    section: "setup",
    title: "Fixture Intake and Decode",
    async run(context) {
      const verify = createVerifier();
      const format = await detectFormat(context.fixture.pngBytes);
      const decoded = await decodeImage(context.fixture.pngBytes);

      verify.equal(format, "png", "working teapot fixture should identify as png");
      verify.ok(decoded.length > 0, "decodeImage should return pixel bytes");
      verify.equal(decoded.length % 4, 0, "decodeImage output should stay RGBA aligned");
      verify.equal(context.fixture.originalWidth, 1024, "source teapot fixture should keep the expected source width");

      return {
        assertions: verify.count,
        metrics: [
          ["Source asset", `${context.fixture.originalWidth}x${context.fixture.originalHeight}`],
          ["Working asset", `${context.fixture.width}x${context.fixture.height}`],
          ["detectFormat()", format],
          ["decodeImage()", `${decoded.length.toLocaleString()} bytes`],
        ],
        views: [
          rgbaView("Teapot guinea pig", context.fixture.rgba, context.fixture.width, context.fixture.height),
          rgbaView("Calibration glyph", context.glyph.rgba, context.glyph.width, context.glyph.height),
        ],
      };
    },
  },
  {
    expectation:
      "The empty composition should be transparent everywhere. Only the checkerboard shell should be visible.",
    section: "setup",
    title: "Empty Composition",
    async run() {
      const verify = createVerifier();
      const composition = await Composition.create({ width: 176, height: 112 });

      try {
        const rgba = composition.renderRgba();
        verify.equal(rgba.length, 176 * 112 * 4, "empty render should have the expected byte size");
        verify.ok(
          rgba.every((v, i) => (i + 1) % 4 !== 0 || v === 0),
          "empty render should remain transparent",
        );

        return {
          assertions: verify.count,
          metrics: [
            ["Canvas", `${composition.width}x${composition.height}`],
            ["Rendered bytes", rgba.length.toLocaleString()],
          ],
          views: [rgbaView("Transparent output", rgba, composition.width, composition.height)],
        };
      } finally {
        composition.free();
      }
    },
  },
];

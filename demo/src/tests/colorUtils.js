import { Composition, hexToRgb, rgbToHex, relativeLuminance, contrastRatio, dominantRgbFromRgba, histogramRgba, extractPaletteFromRgba } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView, swatchView } from "../helpers/views.js";

export const colorUtilTests = [
  {
    expectation:
      "hexToRgb and rgbToHex should round-trip 10 known colors. rgb→hex→rgb should produce the original hex string.",
    section: "colorUtils",
    title: "Hex ↔ RGB Round Trip",
    async run() {
      const verify = createVerifier();
      const knownColors = [
        { hex: "#c9492d", r: 201, g: 73, b: 45 },
        { hex: "#234fdd", r: 35, g: 79, b: 221 },
        { hex: "#157347", r: 21, g: 115, b: 71 },
        { hex: "#f0bf3d", r: 240, g: 191, b: 61 },
        { hex: "#ffffff", r: 255, g: 255, b: 255 },
        { hex: "#000000", r: 0, g: 0, b: 0 },
        { hex: "#7f7f7f", r: 127, g: 127, b: 127 },
        { hex: "#617bff", r: 97, g: 123, b: 255 },
        { hex: "#3dd68c", r: 61, g: 214, b: 140 },
        { hex: "#f05252", r: 240, g: 82, b: 82 },
      ];

      for (const { hex, r, g, b } of knownColors) {
        const rgb = await hexToRgb(hex);
        verify.equal(rgb[0], r, `hexToRgb ${hex} → R should be ${r}`);
        verify.equal(rgb[1], g, `hexToRgb ${hex} → G should be ${g}`);
        verify.equal(rgb[2], b, `hexToRgb ${hex} → B should be ${b}`);
        const backToHex = await rgbToHex({ r, g, b });
        verify.equal(backToHex, hex, `rgbToHex should round-trip ${hex}`);
      }

      const paletteBytes = new Uint8Array(knownColors.flatMap(({ r, g, b }) => [r, g, b, 255]));

      return {
        assertions: verify.count,
        metrics: [["Colors tested", knownColors.length], ["Round trips", knownColors.length]],
        views: [swatchView("Known colors", paletteBytes)],
      };
    },
  },
  {
    expectation:
      "White should have luminance near 1.0, mid gray near 0.22, and black near 0.0. These match the WCAG 2.1 relative luminance formula.",
    section: "colorUtils",
    title: "Relative Luminance",
    async run() {
      const verify = createVerifier();
      const white = await relativeLuminance("#ffffff");
      const midGray = await relativeLuminance("#777777");
      const black = await relativeLuminance("#000000");
      const red = await relativeLuminance("#ff0000");

      verify.ok(white > 0.99, "white luminance should be near 1");
      verify.ok(black < 0.001, "black luminance should be near 0");
      verify.ok(midGray > 0.1 && midGray < 0.3, "mid gray luminance should be in 0.1–0.3 range");
      verify.ok(red > 0.1 && red < 0.3, "pure red luminance should be in 0.1–0.3 range (WCAG formula)");

      return {
        assertions: verify.count,
        metrics: [
          ["white", white.toFixed(4)],
          ["midGray #777", midGray.toFixed(4)],
          ["black", black.toFixed(4)],
          ["red", red.toFixed(4)],
        ],
      };
    },
  },
  {
    expectation:
      "White on dark background should exceed 4.5:1 (AA) and 7:1 (AAA). A low-contrast gray pair should fail both thresholds.",
    section: "colorUtils",
    title: "Contrast Ratio",
    async run() {
      const verify = createVerifier();
      const highContrast = await contrastRatio("#ffffff", "#0c0c10");
      const aaPass = await contrastRatio("#ffffff", "#595959");
      const aaaFail = await contrastRatio("#aaaaaa", "#888888");

      verify.ok(highContrast > 15, "white on near-black should exceed 15:1");
      verify.ok(aaPass > 4.5, "white on dark gray should pass AA (4.5:1)");
      verify.ok(aaaFail < 3, "near-same gray pair should fail AA");

      return {
        assertions: verify.count,
        metrics: [
          ["white/#0c0c10", highContrast.toFixed(2) + ":1"],
          ["white/#595959", aaPass.toFixed(2) + ":1"],
          ["#aaa/#888", aaaFail.toFixed(2) + ":1"],
        ],
      };
    },
  },
  {
    expectation:
      "dominantRgbFromRgba on the teapot fixture should return a 3-byte array with non-zero values representing the most visually dominant color.",
    section: "colorUtils",
    title: "Dominant Color Extraction",
    async run(context) {
      const verify = createVerifier();
      const dominant = await dominantRgbFromRgba(context.fixture.rgba, { height: context.fixture.height, width: context.fixture.width });

      verify.equal(dominant.length, 3, "dominant should return [R, G, B]");
      verify.ok(dominant[0] > 0 || dominant[1] > 0 || dominant[2] > 0, "dominant color should be non-black");

      const swatchBytes = new Uint8Array([dominant[0], dominant[1], dominant[2], 255]);

      return {
        assertions: verify.count,
        metrics: [["Dominant RGB", `${dominant[0]}, ${dominant[1]}, ${dominant[2]}`]],
        views: [
          rgbaView("Teapot source", context.fixture.rgba, context.fixture.width, context.fixture.height, { maxDisplay: 280 }),
          swatchView("Dominant color", swatchBytes),
        ],
      };
    },
  },
  {
    expectation:
      "histogramRgba on a solid red layer should show a spike in the R channel at bin 255, while G and B channels remain flat at zero.",
    section: "colorUtils",
    title: "Histogram RGBA",
    async run(context) {
      const verify = createVerifier();
      const w = 64, h = 64;
      const redPixels = new Uint8Array(w * h * 4);
      for (let i = 0; i < redPixels.length; i += 4) {
        redPixels[i] = 255; redPixels[i + 1] = 0; redPixels[i + 2] = 0; redPixels[i + 3] = 255;
      }

      const histogram = await histogramRgba(redPixels, { width: w, height: h });
      verify.equal(histogram.length, 1024, "histogram should have 4 * 256 = 1024 bins");

      const rAt255 = histogram[255];
      const gAt255 = histogram[256 + 255];
      const bAt255 = histogram[512 + 255];
      verify.equal(rAt255, w * h, "R bin at 255 should equal pixel count");
      verify.equal(gAt255, 0, "G bin at 255 should be zero for solid red");
      verify.equal(bAt255, 0, "B bin at 255 should be zero for solid red");

      const teapotHistogram = await histogramRgba(context.fixture.rgba, { width: context.fixture.width, height: context.fixture.height });
      verify.equal(teapotHistogram.length, 1024, "teapot histogram should also have 1024 bins");

      const barCanvasRgba = buildHistogramCanvas(teapotHistogram, 256, 64);

      return {
        assertions: verify.count,
        metrics: [["Bins", histogram.length], ["R at 255", rAt255], ["G at 255", gAt255], ["B at 255", bAt255]],
        views: [
          rgbaView("Solid red (source)", redPixels, w, h, { maxDisplay: 180 }),
          rgbaView("Teapot histogram (RGBA channels)", barCanvasRgba, 256, 64, { maxDisplay: 360 }),
        ],
      };
    },
  },
  {
    expectation:
      "extractPaletteFromRgba on the teapot fixture should return a flat RGBA byte array representing up to 8 distinct colors extracted from the image.",
    section: "colorUtils",
    title: "Palette Extraction (free function)",
    async run(context) {
      const verify = createVerifier();
      const palette4 = await extractPaletteFromRgba(context.fixture.rgba, { height: context.fixture.height, maxColors: 4, width: context.fixture.width });
      const palette8 = await extractPaletteFromRgba(context.fixture.rgba, { height: context.fixture.height, maxColors: 8, width: context.fixture.width });

      verify.equal(palette4.length % 4, 0, "4-color palette should be RGBA aligned");
      verify.equal(palette8.length % 4, 0, "8-color palette should be RGBA aligned");
      verify.ok(palette4.length >= 4, "4-color palette should have at least 1 color");
      verify.ok(palette8.length >= palette4.length, "8-color request should return at least as many colors as 4-color");
      verify.ok(palette8.some((v, i) => i % 4 < 3 && v > 0), "palette should contain non-black colors");

      return {
        assertions: verify.count,
        metrics: [["4-color palette", `${palette4.length / 4} colors`], ["8-color palette", `${palette8.length / 4} colors`]],
        views: [
          rgbaView("Teapot", context.fixture.rgba, context.fixture.width, context.fixture.height, { maxDisplay: 280 }),
          swatchView("4-color palette", palette4),
          swatchView("8-color palette", palette8),
        ],
      };
    },
  },
];

function buildHistogramCanvas(histogram, width, height) {
  const canvas = document.createElement("canvas");
  canvas.width = width; canvas.height = height;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#0c0c10";
  ctx.fillRect(0, 0, width, height);

  const channels = [
    { offset: 0, color: "rgba(240,82,82,0.8)" },
    { offset: 256, color: "rgba(61,214,140,0.8)" },
    { offset: 512, color: "rgba(97,123,255,0.8)" },
  ];

  let max = 0;
  for (let c = 0; c < 3; c++) {
    for (let i = 0; i < 256; i++) {
      max = Math.max(max, histogram[channels[c].offset + i]);
    }
  }
  if (max === 0) max = 1;

  for (const { offset, color } of channels) {
    ctx.fillStyle = color;
    for (let i = 0; i < 256; i++) {
      const val = histogram[offset + i];
      const barH = Math.round((val / max) * (height - 4));
      if (barH > 0) ctx.fillRect(i, height - barH, 1, barH);
    }
  }

  const imageData = ctx.getImageData(0, 0, width, height);
  return new Uint8Array(imageData.data);
}

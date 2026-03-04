import preload, {
  clearRegisteredFonts,
  Composition,
  contrastRatio,
  decodeImage,
  detectFormat,
  dominantRgbFromRgba,
  extractPaletteFromRgba,
  hexToRgb,
  histogramRgba,
  loadGoogleFont,
  quantizeRgba,
  registerFont,
  relativeLuminance,
  rgbToHex,
  simdSupported,
} from "../dist/index.js";
import { base64ToRgba, rgbaToBase64 } from "../dist/base64.js";
import { readableTextColor } from "../dist/color-utils.js";

const dom = {
  body: document.body,
  diagnosticCount: document.getElementById("diagnostic-count"),
  diagnosticList: document.getElementById("diagnostic-list"),
  rerunButton: document.getElementById("rerun-button"),
  runtimeCount: document.getElementById("runtime-count"),
  runtimeExperimental: document.getElementById("runtime-experimental"),
  runtimeFail: document.getElementById("runtime-fail"),
  runtimePass: document.getElementById("runtime-pass"),
  runtimeSimd: document.getElementById("runtime-simd"),
  runtimeStatus: document.getElementById("runtime-status"),
  suiteMachineState: document.getElementById("suite-machine-state"),
  suite: document.getElementById("suite"),
};

const SECTION_INFO = {
  setup: {
    chip: "Calibration",
    description:
      "Source fixtures, preload, and decode utilities. These cards establish the known-good pixels the rest of the page will manipulate.",
    title: "Boot and Fixture Intake",
  },
  layers: {
    chip: "Document",
    description:
      "Layer creation, ordering, grouping, visibility, masking, clipping, flattening, and canvas resizing through the stable Composition facade.",
    title: "Layer Graph and Compositing",
  },
  transforms: {
    chip: "Transforms",
    description:
      "Non-destructive translate, rotate, flip, anchor, scale, and destructive raster resampling paths side by side.",
    title: "Transform and Resample Paths",
  },
  filters: {
    chip: "Filters",
    description:
      "Scoped filter layers and destructive layer filters. The same source image is reused so regressions are easy to spot.",
    title: "Color and Filter Paths",
  },
  shapes: {
    chip: "Shapes",
    description:
      "Vector-style shape layers, brush strokes, bucket fill behavior, palette extraction, and quantization outputs that should be visually legible without any interaction.",
    title: "Shape, Fill, and Palette Tools",
  },
  text: {
    chip: "Text",
    description:
      "Registered real-font text layers rendered through the public API. Registration, multiline layout, tracking, color, and transform updates should all stay visually obvious.",
    title: "Text Layer Surface",
  },
  io: {
    chip: "Formats",
    description:
      "Retained SVG layers, serialization, PNG/JPEG/WebP import-export, GIF frame import, sprite helpers, and utility outputs that verify package-level APIs beyond rendering.",
    title: "Format, SVG, Sprite, and Utility Surface",
  },
  experimental: {
    chip: "Experimental",
    description:
      "Unstable or intentionally deprioritized paths that remain visible on the page without being counted as stable pass/fail requirements.",
    title: "Experimental Surface",
  },
};

const diagnostics = [];
const sectionNodes = new Map();
const suiteState = {
  cards: 0,
  diagnostics: 0,
  experimental: 0,
  fail: 0,
  pass: 0,
  simd: "Checking",
  status: "booting",
  statusText: "Booting",
};
let runSequence = 0;

const SIMPLE_SVG = `
  <svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96">
    <defs>
      <linearGradient id="chip" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#d9482b" />
        <stop offset="100%" stop-color="#2d55d7" />
      </linearGradient>
    </defs>
    <rect x="10" y="10" width="76" height="76" rx="18" fill="url(#chip)" />
    <circle cx="34" cy="34" r="10" fill="#f2c94c" />
    <path d="M28 62 L48 42 L67 61" fill="none" stroke="#fffaf1" stroke-width="9" stroke-linecap="round" />
    <rect x="54" y="24" width="12" height="24" rx="6" fill="#fffaf1" />
  </svg>
`;

installDiagnostics();
dom.rerunButton.addEventListener("click", () => {
  void runSuite();
});

void runSuite();

async function runSuite() {
  const runId = ++runSequence;
  resetSuiteUi();
  setRuntimeStatus("running", "Initializing");
  setSimdStatus("Checking");

  try {
    await preload(resolveDemoPreloadInput());
    const context = await buildContext();
    if (runId !== runSequence) {
      return;
    }

    setRuntimeStatus("running", `Ready · ${context.fixture.width}x${context.fixture.height} working teapot`);
    setSimdStatus(context.runtime.simd ? "Available" : "Scalar");

    const counts = {
      experimental: 0,
      fail: 0,
      pass: 0,
      total: 0,
    };

    for (const test of createTests()) {
      if (runId !== runSequence) {
        return;
      }

      const card = createCard(test);
      const startedAt = performance.now();
      counts.total += 1;
      updateCounters(counts);

      try {
        const result = await test.run(context);
        renderCardResult(card, result, performance.now() - startedAt);

        if (test.experimental) {
          setCardStatus(card, "experimental", "Experimental");
          counts.experimental += 1;
        } else {
          setCardStatus(card, "pass", `${result.assertions} checks`);
          counts.pass += 1;
        }
      } catch (error) {
        setCardStatus(card, "fail", toErrorMessage(error));
        appendMessageView(card.views, "Failure", toErrorMessage(error));
        appendMeta(card.meta, "Elapsed", `${Math.round(performance.now() - startedAt)} ms`);
        counts.fail += 1;
        recordDiagnostic("error", `[${test.title}] ${toErrorMessage(error)}`);
      }

      updateCounters(counts);
    }

    setRuntimeStatus(
      counts.fail === 0 ? "completed" : "failed",
      counts.fail === 0 ? "Completed without runtime failures" : "Completed with failures",
    );
  } catch (error) {
    setRuntimeStatus("fatal", "Fatal error");
    recordDiagnostic("error", `[fatal] ${toErrorMessage(error)}`);
    const fatal = document.createElement("section");
    fatal.className = "section-intro";
    fatal.innerHTML = `
      <p class="eyebrow">Fatal</p>
      <h2>The visual suite could not boot.</h2>
      <p>${escapeHtml(toErrorMessage(error))}</p>
    `;
    dom.suite.replaceChildren(fatal);
  }
}

function createTests() {
  return [
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
        verify.equal(
          context.fixture.originalWidth,
          1024,
          "source teapot fixture should keep the expected source width",
        );

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
          verify.ok(rgba.every((value, index) => (index + 1) % 4 !== 0 || value === 0), "empty render should remain transparent");

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
    {
      expectation:
        "Background wash, teapot group, paint veil, and hidden layer handling should compose into a readable poster without the hidden bar appearing.",
      section: "layers",
      title: "Layer Stack, Visibility, and Metadata",
      async run(context) {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 280, height: 184 });

        try {
          composition.addSolidColorLayer({ color: [244, 235, 218, 255], name: "paper" });
          composition.addGradientLayer({
            direction: "diagonalDown",
            name: "wash",
            stops: [
              { color: [255, 255, 255, 0], position: 0 },
              { color: [227, 117, 69, 68], position: 1 },
            ],
          });

          const groupId = composition.addGroupLayer({ name: "teapot-group" });
          const imageId = composition.addImageLayer({
            height: context.fixture.height,
            name: "teapot",
            parentId: groupId,
            rgba: context.fixture.rgba,
            width: context.fixture.width,
            x: 38,
            y: 10,
          });
          const paintId = composition.addPaintLayer({
            height: 184,
            name: "paint-veil",
            width: 280,
          });
          composition.bucketFillLayer(paintId, {
            color: [24, 77, 163, 30],
            x: 0,
            y: 0,
          });
          composition.setLayerOpacity(paintId, 0.7);
          composition.setLayerPosition(paintId, { x: 0, y: 0 });

          const filterId = composition.addFilterLayer({ name: "group-filter", parentId: groupId });
          composition.setFilterLayerConfig(filterId, {
            contrast: 0.14,
            saturation: 0.12,
          });

          composition.addShapeLayer({
            fill: [38, 94, 225, 34],
            height: 108,
            name: "halo",
            stroke: { color: [38, 94, 225, 120], width: 3 },
            type: "ellipse",
            width: 108,
            x: 152,
            y: 16,
          });

          const hiddenId = composition.addShapeLayer({
            fill: [201, 73, 45, 255],
            height: 18,
            name: "hidden-bar",
            type: "rectangle",
            width: 280,
            x: 0,
            y: 166,
          });
          composition.setLayerVisibility(hiddenId, false);

          const render = composition.renderRgba();
          const layers = composition.listLayers({ recursive: true });
          const imageLayer = composition.getLayer(imageId);

          verify.equal(composition.layerCount(), 6, "layerCount should track top-level layers");
          verify.equal(layers.length, 8, "recursive listLayers should return every layer");
          verify.equal(imageLayer?.kind, "raster", "getLayer should describe the teapot as a raster layer");
          verify.ok(layers.some((layer) => layer.name === "hidden-bar" && layer.visible === false), "hidden layer should remain in metadata but stay invisible");

          return {
            assertions: verify.count,
            layers,
            metrics: [
              ["layerCount()", composition.layerCount()],
              ["group id", groupId],
              ["filter id", filterId],
              ["image anchor", imageLayer?.anchor ?? "n/a"],
            ],
            views: [rgbaView("Poster stack", render, composition.width, composition.height)],
          };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "Reordering, regrouping, flattening, removal, and canvas resizing should end with a larger sheet and a single flattened cluster on the left.",
      section: "layers",
      title: "Move, Remove, Flatten, Resize",
      async run(context) {
        const verify = createVerifier();
        const before = await buildLayerSurgeryScene(context, false);
        const after = await buildLayerSurgeryScene(context, true);

        try {
          verify.equal(before.layerCount, 4, "initial top-level layer count should include paper, group, accent, and ghost");
          verify.equal(after.layerCount, 2, "flattened scene should end with two top-level layers");
          verify.equal(after.width, 296, "resizeCanvas should change the width");
          verify.equal(after.height, 184, "resizeCanvas should change the height");

          return {
            assertions: verify.count,
            metrics: [
              ["Before", `${before.width}x${before.height}, ${before.layerCount} layers`],
              ["After", `${after.width}x${after.height}, ${after.layerCount} layers`],
            ],
            views: [
              rgbaView("Before surgery", before.rgba, before.width, before.height),
              rgbaView("After regroup + flatten", after.rgba, after.width, after.height),
            ],
          };
        } finally {
          before.dispose();
          after.dispose();
        }
      },
    },
    {
      expectation:
        "The multiply badge should darken where circles overlap, and the moved layer should sit noticeably off-center with reduced opacity.",
      section: "layers",
      title: "Blend Mode, Position, and Opacity",
      async run() {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 220, height: 154 });

        try {
          composition.addSolidColorLayer({ color: [249, 242, 230, 255], name: "paper" });
          composition.addShapeLayer({
            fill: [226, 77, 56, 235],
            height: 86,
            name: "circle-a",
            type: "ellipse",
            width: 86,
            x: 28,
            y: 36,
          });
          const badgeId = composition.addShapeLayer({
            fill: [48, 101, 239, 235],
            height: 86,
            name: "circle-b",
            type: "ellipse",
            width: 86,
            x: 84,
            y: 26,
          });

          composition.setLayerBlendMode(badgeId, "multiply");
          composition.setLayerOpacity(badgeId, 0.68);
          composition.setLayerPosition(badgeId, { x: 98, y: 30 });

          const info = composition.getLayer(badgeId);
          verify.equal(info?.blendMode, "multiply", "blend mode should update");
          verify.equal(info?.opacity, 0.68, "opacity should update");
          verify.equal(info?.x, 98, "x position should update");

          return {
            assertions: verify.count,
            metrics: [
              ["blend mode", info?.blendMode ?? "n/a"],
              ["opacity", info?.opacity ?? "n/a"],
              ["position", `${info?.x}, ${info?.y}`],
            ],
            views: [rgbaView("Multiply overlap", composition.renderRgba(), composition.width, composition.height)],
          };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "Clip-to-below should keep the teapot inside the ellipse only. Masked, inverted, and cleared states should visibly diverge.",
      featured: true,
      fullSpan: true,
      previewMin: 270,
      section: "layers",
      title: "Clip and Mask Pipeline",
      async run(context) {
        const verify = createVerifier();
        const clip = await buildClipScene(context);
        const maskStates = await buildMaskStates(context);

        try {
          verify.equal(maskStates.cleared.hasMask, false, "clearLayerMask should remove the mask metadata");
          verify.equal(maskStates.masked.hasMask, true, "masked state should report a mask");
          verify.ok(
            !rgbaEquals(maskStates.masked.rgba, maskStates.cleared.rgba),
            "masked render should differ from the cleared render",
          );

          return {
            assertions: verify.count,
            metrics: [
              ["clip layers", clip.layerCount],
              ["masked hasMask", String(maskStates.masked.hasMask)],
              ["inverted mask", String(maskStates.inverted.maskInverted)],
            ],
            views: [
              rgbaView("clipToBelow", clip.rgba, clip.width, clip.height, { maxDisplay: 320 }),
              rgbaView("Mask", maskStates.masked.rgba, maskStates.masked.width, maskStates.masked.height, {
                maxDisplay: 320,
              }),
              rgbaView(
                "Mask inverted",
                maskStates.inverted.rgba,
                maskStates.inverted.width,
                maskStates.inverted.height,
                { maxDisplay: 320 },
              ),
              rgbaView("Mask cleared", maskStates.cleared.rgba, maskStates.cleared.width, maskStates.cleared.height, {
                maxDisplay: 320,
              }),
            ],
          };
        } finally {
          clip.dispose();
        }
      },
    },
    {
      expectation:
        "Flip and rotation should be obvious because the calibration glyph is asymmetric. The rotated copy should pivot around its center, not its top-left corner.",
      section: "transforms",
      title: "Transform Setters",
      async run(context) {
        const verify = createVerifier();
        const baseline = await buildTransformSetterVariant(context, "baseline");
        const flipX = await buildTransformSetterVariant(context, "flipX");
        const flipY = await buildTransformSetterVariant(context, "flipY");
        const rotated = await buildTransformSetterVariant(context, "rotated");

        verify.equal(baseline.layer?.opacity, 0.94, "baseline opacity should update");
        verify.equal(flipX.layer?.flipX, true, "flipX should update");
        verify.equal(flipY.layer?.flipY, true, "flipY should update");
        verify.equal(rotated.layer?.anchor, "center", "anchor should become center");
        verify.equal(Math.round(rotated.layer?.rotation ?? 0), 34, "rotation should update");

        return {
          assertions: verify.count,
          metrics: [
            ["baseline opacity", baseline.layer?.opacity ?? "n/a"],
            ["flipX", String(flipX.layer?.flipX)],
            ["flipY", String(flipY.layer?.flipY)],
            ["rotation", rotated.layer?.rotation?.toFixed(1) ?? "n/a"],
          ],
          views: [
            rgbaView("Baseline", baseline.rgba, baseline.width, baseline.height, { maxDisplay: 220 }),
            rgbaView("flipX", flipX.rgba, flipX.width, flipX.height, { maxDisplay: 220 }),
            rgbaView("flipY", flipY.rgba, flipY.width, flipY.height, { maxDisplay: 220 }),
            rgbaView("anchor=center, rotation=34", rotated.rgba, rotated.width, rotated.height, {
              maxDisplay: 220,
            }),
          ],
        };
      },
    },
    {
      expectation:
        "The patched teapot should move right, shrink slightly, tilt, and mirror without needing a cascade of setter calls.",
      section: "transforms",
      title: "updateLayer Combined Patch",
      async run(context) {
        const verify = createVerifier();
        const before = await buildPatchedTransformScene(context, false);
        const after = await buildPatchedTransformScene(context, true);

        try {
          verify.equal(after.layer?.anchor, "center", "updateLayer should set anchor");
          verify.equal(after.layer?.flipX, true, "updateLayer should set flipX");
          verify.equal(after.layer?.scaleX, 0.82, "updateLayer should set scaleX");
          verify.equal(after.layer?.scaleY, 1.14, "updateLayer should set scaleY");

          return {
            assertions: verify.count,
            metrics: [
              ["rotation", after.layer?.rotation?.toFixed(1) ?? "n/a"],
              ["scale", `${after.layer?.scaleX ?? "?"} x ${after.layer?.scaleY ?? "?"}`],
              ["position", `${after.layer?.x}, ${after.layer?.y}`],
            ],
            views: [
              rgbaView("Before patch", before.rgba, before.width, before.height),
              rgbaView("After updateLayer()", after.rgba, after.width, after.height),
            ],
          };
        } finally {
          before.dispose();
          after.dispose();
        }
      },
    },
    {
      expectation:
        "Nearest should stay blocky, bilinear should soften edges, Lanczos should be cleaner, crop should isolate the center, trim should remove the transparent border, and rotateLayer should grow the raster.",
      featured: true,
      fullSpan: true,
      previewMin: 250,
      section: "transforms",
      title: "Destructive Resample and Trim",
      async run(context) {
        const verify = createVerifier();
        const variants = await buildResampleVariants(context);

        verify.ok(
          variants.trimmed.width < context.borderedGlyph.width,
          "trimmed glyph should lose horizontal transparent border",
        );
        verify.ok(
          variants.trimmed.height < context.borderedGlyph.height,
          "trimmed glyph should lose vertical transparent border",
        );
        verify.ok(variants.rotated.width > context.glyph.width, "rotateLayer should enlarge the raster bounds");

        return {
          assertions: verify.count,
          metrics: [
            ["nearest", `${variants.nearest.width}x${variants.nearest.height}`],
            ["bilinear", `${variants.bilinear.width}x${variants.bilinear.height}`],
            ["lanczos3", `${variants.lanczos.width}x${variants.lanczos.height}`],
            ["rotated", `${variants.rotated.width}x${variants.rotated.height}`],
          ],
          views: [
            rgbaView("Nearest", variants.nearest.rgba, variants.nearest.width, variants.nearest.height, {
              maxDisplay: 260,
            }),
            rgbaView("Bilinear", variants.bilinear.rgba, variants.bilinear.width, variants.bilinear.height, {
              maxDisplay: 260,
            }),
            rgbaView("Lanczos3", variants.lanczos.rgba, variants.lanczos.width, variants.lanczos.height, {
              maxDisplay: 260,
            }),
            rgbaView("Crop", variants.cropped.rgba, variants.cropped.width, variants.cropped.height, {
              maxDisplay: 260,
            }),
            rgbaView("Trim alpha", variants.trimmed.rgba, variants.trimmed.width, variants.trimmed.height, {
              maxDisplay: 260,
            }),
            rgbaView("rotateLayer()", variants.rotated.rgba, variants.rotated.width, variants.rotated.height, {
              maxDisplay: 260,
            }),
          ],
        };
      },
    },
    {
      expectation:
        "Only the grouped color chart should shift hue and contrast. The matching control chart on the left should stay unchanged.",
      section: "filters",
      title: "Scoped Filter Layer",
      async run(context) {
        const verify = createVerifier();
        const base = await buildScopedFilterScene(context, false);
        const filtered = await buildScopedFilterScene(context, true);

        try {
          verify.ok(!rgbaEquals(base.rgba, filtered.rgba), "filter layer should alter the group render");
          verify.equal(filtered.filter?.kind, "filter", "filter layer should appear in metadata");

          return {
            assertions: verify.count,
            metrics: [
              ["filter hue", filtered.filter?.filterConfig?.hueDeg?.toFixed(1) ?? "n/a"],
              ["filter contrast", filtered.filter?.filterConfig?.contrast ?? "n/a"],
            ],
            views: [
              rgbaView("Without scoped filter", base.rgba, base.width, base.height),
              rgbaView("With scoped filter", filtered.rgba, filtered.width, filtered.height),
            ],
          };
        } finally {
          base.dispose();
          filtered.dispose();
        }
      },
    },
    {
      expectation:
        "Each destructive filter should visibly alter the same hard-edged color chart in a distinct way. This card is meant for fast scanning, not pixel-perfect comparison.",
      featured: true,
      fullSpan: true,
      previewMin: 250,
      section: "filters",
      title: "Destructive Filter Strip",
      async run(context) {
        const verify = createVerifier();
        const variants = await buildFilterVariants(context);

        verify.ok(variants.length >= 10, "expected all destructive filter variants");
        verify.ok(variants.every((variant) => variant.rgba.length > 0), "each variant should render bytes");

        return {
          assertions: verify.count,
          metrics: [
            ["Variants", variants.length],
            ["Source", `${context.fixture.width}x${context.fixture.height}`],
          ],
          views: variants.map((variant) =>
            rgbaView(variant.label, variant.rgba, variant.width, variant.height, {
              maxDisplay: 240,
            }),
          ),
        };
      },
    },
    {
      expectation:
        "Rectangle, rounded rect, ellipse, line, and polygon layers should all appear in one composition with both fill and stroke visible.",
      section: "shapes",
      title: "Shape Layer Gallery",
      async run() {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 272, height: 180 });

        try {
          composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
          composition.addShapeLayer({
            fill: [201, 73, 45, 224],
            height: 52,
            name: "rectangle",
            stroke: { color: [73, 32, 24, 255], width: 3 },
            type: "rectangle",
            width: 90,
            x: 18,
            y: 24,
          });
          composition.addShapeLayer({
            fill: [35, 79, 221, 160],
            height: 52,
            name: "rounded",
            radius: 14,
            stroke: { color: [35, 79, 221, 255], width: 4 },
            type: "rectangle",
            width: 90,
            x: 130,
            y: 24,
          });
          composition.addShapeLayer({
            fill: [42, 155, 106, 140],
            height: 70,
            name: "ellipse",
            stroke: { color: [18, 84, 56, 255], width: 3 },
            type: "ellipse",
            width: 70,
            x: 192,
            y: 84,
          });
          composition.addShapeLayer({
            name: "line",
            stroke: { color: [14, 14, 14, 255], width: 5 },
            type: "line",
            width: 96,
            height: 54,
            x: 20,
            y: 102,
          });
          composition.addShapeLayer({
            fill: [242, 190, 62, 210],
            name: "polygon",
            points: [
              { x: 0, y: 46 },
              { x: 36, y: 0 },
              { x: 76, y: 20 },
              { x: 56, y: 70 },
              { x: 12, y: 78 },
            ],
            stroke: { color: [94, 55, 7, 255], width: 3 },
            type: "polygon",
            x: 116,
            y: 92,
          });

          const layers = composition.listLayers();
          verify.equal(layers.filter((layer) => layer.kind === "shape").length, 5, "expected five shape layers");

          return {
            assertions: verify.count,
            layers,
            metrics: [["Shape layers", 5]],
            views: [rgbaView("Shape composition", composition.renderRgba(), composition.width, composition.height)],
          };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "Contiguous fill should stay inside the island, non-contiguous fill should hit every matching island, and tolerance fill should smooth out noisy neighbors.",
      section: "shapes",
      title: "Bucket Fill Modes",
      async run(context) {
        const verify = createVerifier();
        const contiguous = await buildFillVariant(context, "contiguous");
        const nonContiguous = await buildFillVariant(context, "non-contiguous");
        const tolerance = await buildFillVariant(context, "tolerance");

        verify.ok(!rgbaEquals(contiguous.rgba, nonContiguous.rgba), "fill modes should not collapse to the same output");
        verify.ok(!rgbaEquals(nonContiguous.rgba, tolerance.rgba), "tolerance fill should differ from exact non-contiguous fill");

        return {
          assertions: verify.count,
          metrics: [
            ["Contiguous target", "single enclosed island"],
            ["Non-contiguous target", "all shared color islands"],
            ["Tolerance", "alpha-aware match threshold"],
          ],
          views: [
            rgbaView("Contiguous", contiguous.rgba, contiguous.width, contiguous.height),
            rgbaView("Non-contiguous", nonContiguous.rgba, nonContiguous.width, nonContiguous.height),
            rgbaView("Tolerance", tolerance.rgba, tolerance.width, tolerance.height),
          ],
        };
      },
    },
    {
      expectation:
        "The brush engine should show a hard red stroke, a blue modeler-smoothed stroke with tilt-shaped dabs, a grainy ochre textured stroke, and an eraser cut that visibly clears a diagonal path through the paint layer.",
      section: "shapes",
      title: "Brush Stroke Engine",
      async run() {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 272, height: 156 });

        try {
          composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
          composition.addShapeLayer({
            fill: [236, 222, 207, 255],
            height: 104,
            name: "panel",
            stroke: { color: [214, 193, 171, 255], width: 2 },
            type: "rectangle",
            width: 232,
            x: 20,
            y: 24,
          });
          composition.addShapeLayer({
            fill: [0, 0, 0, 0],
            height: 84,
            name: "guide-a",
            stroke: { color: [120, 112, 101, 72], width: 2 },
            type: "line",
            width: 80,
            x: 40,
            y: 36,
          });
          composition.addShapeLayer({
            fill: [0, 0, 0, 0],
            height: 84,
            name: "guide-b",
            stroke: { color: [120, 112, 101, 72], width: 2 },
            type: "line",
            width: 80,
            x: 152,
            y: 36,
          });

          const paintId = composition.addPaintLayer({
            name: "brush-layer",
            width: 232,
            height: 104,
          });
          composition.setLayerPosition(paintId, { x: 20, y: 24 });

          const hardStroke = Array.from({ length: 12 }, (_, index) => ({
            pressure: 1,
            x: 18 + index * 16,
            y: 26 + Math.sin(index / 2) * 12,
          }));
          const softStroke = Array.from({ length: 14 }, (_, index) => ({
            pressure: 0.2 + (index / 13) * 0.8,
            tiltX: 0.15 + (index / 13) * 0.85,
            tiltY: 0.8 - (index / 13) * 0.65,
            timeMs: index * 18,
            x: 32 + index * 12,
            y: 72 + Math.cos(index / 2.2) * 10,
          }));
          const grainStroke = Array.from({ length: 11 }, (_, index) => ({
            pressure: 0.55 + Math.sin(index / 2.5) * 0.2,
            tiltX: -0.55 + index * 0.1,
            tiltY: 0.3,
            timeMs: index * 14,
            x: 42 + index * 15,
            y: 48 + Math.sin(index / 1.8) * 9,
          }));
          const eraseStroke = Array.from({ length: 10 }, (_, index) => ({
            pressure: 1,
            x: 36 + index * 18,
            y: 14 + index * 8,
          }));

          verify.ok(
            (() => {
              const strokeId = composition.beginBrushStroke(paintId, {
                color: [201, 73, 45, 255],
                hardness: 0.95,
                size: 10,
                spacing: 0.4,
              });
              const midpoint = Math.ceil(hardStroke.length / 2);
              return (
                strokeId > 0 &&
                composition.pushBrushPoints(strokeId, hardStroke.slice(0, midpoint)) &&
                composition.pushBrushPoints(strokeId, hardStroke.slice(midpoint)) &&
                composition.endBrushStroke(strokeId)
              );
            })(),
            "streamed hard brush stroke should paint into the raster layer",
          );
          verify.ok(
            (() => {
              const strokeId = composition.beginBrushStroke(paintId, {
                color: [35, 79, 221, 255],
                flow: 0.65,
                hardness: 0.25,
                pressureOpacity: 0.35,
                pressureSize: 1,
                size: 16,
                smoothing: 0.2,
                smoothingMode: "modeler",
                spacing: 0.35,
              });
              const oneThird = Math.ceil(softStroke.length / 3);
              return (
                strokeId > 0 &&
                composition.pushBrushPoints(strokeId, softStroke.slice(0, oneThird)) &&
                composition.pushBrushPoints(strokeId, softStroke.slice(oneThird, oneThird * 2)) &&
                composition.pushBrushPoints(strokeId, softStroke.slice(oneThird * 2)) &&
                composition.endBrushStroke(strokeId)
              );
            })(),
            "streamed soft pressure stroke should paint into the raster layer",
          );
          verify.ok(
            composition.paintStrokeLayer(paintId, {
              color: [183, 132, 52, 255],
              hardness: 0.45,
              points: grainStroke,
              size: 13,
              smoothing: 0.3,
              smoothingMode: "modeler",
              spacing: 0.32,
              tip: "grain",
            }),
            "grain tip stroke should leave a textured path in the raster layer",
          );
          verify.ok(
            (() => {
              const strokeId = composition.beginBrushStroke(paintId, {
                size: 12,
                spacing: 0.3,
                tool: "erase",
              });
              return (
                strokeId > 0 &&
                composition.pushBrushPoints(strokeId, eraseStroke.slice(0, 4)) &&
                composition.pushBrushPoints(strokeId, eraseStroke.slice(4)) &&
                composition.endBrushStroke(strokeId)
              );
            })(),
            "streamed eraser stroke should clear alpha from the raster layer",
          );
          const beforeCancel = Array.from(composition.getLayerRgba(paintId));
          const cancelStrokeId = composition.beginBrushStroke(paintId, {
            color: [201, 73, 45, 255],
            size: 8,
          });
          verify.ok(
            cancelStrokeId > 0 &&
              composition.pushBrushPoints(cancelStrokeId, [
                { pressure: 1, x: 28, y: 88 },
                { pressure: 1, x: 88, y: 94 },
              ]) &&
              composition.cancelBrushStroke(cancelStrokeId),
            "canceled streamed stroke should restore the original raster",
          );

          const paintLayer = composition.getLayer(paintId);
          const paintRgba = composition.getLayerRgba(paintId);
          const alphaSamples = [
            paintRgba[(30 * 232 + 34) * 4 + 3],
            paintRgba[(70 * 232 + 52) * 4 + 3],
            paintRgba[(40 * 232 + 96) * 4 + 3],
          ];

          verify.equal(paintLayer?.kind, "raster", "brush strokes should target a raster layer");
          verify.ok(alphaSamples.some((value) => value > 0), "brush layer should contain painted alpha");
          verify.ok(alphaSamples.some((value) => value < 255), "eraser or soft brush should leave partial alpha");
          verify.ok(
            paintRgba[(58 * 232 + 120) * 4 + 3] !== paintRgba[(58 * 232 + 122) * 4 + 3],
            "grain tip should create visible local alpha variation",
          );
          verify.equal(
            Array.from(paintRgba).join(","),
            beforeCancel.join(","),
            "canceled stroke should not leave any pixels behind",
          );

          return {
            assertions: verify.count,
            metrics: [
              ["Streamed / direct strokes", "3 + 1"],
              ["Canceled sessions", 1],
              ["Hard / modeler / grain / erase", "10px / 16px / 13px / 12px"],
              ["Brush target", paintLayer?.kind ?? "missing"],
            ],
            views: [
              rgbaView("Brush composition", composition.renderRgba(), composition.width, composition.height),
              rgbaView("Paint layer only", paintRgba, 232, 104),
            ],
          };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "Quantization should visibly flatten the teapot palette, and the extracted swatches plus dominant color facts should line up with the rendered result.",
      featured: true,
      fullSpan: true,
      previewMin: 250,
      section: "shapes",
      title: "Palette Extraction and Quantization",
      async run(context) {
        const verify = createVerifier();
        const composition = await Composition.create({ width: context.fixture.width, height: context.fixture.height });

        try {
          const layerId = composition.addImageLayer({
            height: context.fixture.height,
            name: "teapot",
            rgba: context.fixture.rgba,
            width: context.fixture.width,
          });
          const palette = composition.extractPalette(layerId, { maxColors: 8 });
          const dominant = await dominantRgbFromRgba(context.fixture.rgba, {
            height: context.fixture.height,
            width: context.fixture.width,
          });
          const histogram = await histogramRgba(context.fixture.rgba, {
            height: context.fixture.height,
            width: context.fixture.width,
          });
          const topPalette = await extractPaletteFromRgba(context.fixture.rgba, {
            height: context.fixture.height,
            maxColors: 8,
            width: context.fixture.width,
          });

          composition.quantizeLayer(layerId, { palette });
          const quantizedLayer = composition.getLayerRgba(layerId);
          const topQuantized = await quantizeRgba(context.fixture.rgba, {
            height: context.fixture.height,
            palette: topPalette,
            width: context.fixture.width,
          });

          verify.equal(palette.length, 32, "layer palette should expose 8 RGBA swatches");
          verify.equal(topPalette.length, 32, "top-level palette extraction should expose 8 RGBA swatches");
          verify.equal(dominant.length, 3, "dominantRgbFromRgba should return RGB");
          verify.equal(histogram.length, 1024, "histogramRgba should expose 4 * 256 bins");

          return {
            assertions: verify.count,
            metrics: [
              ["Dominant RGB", `${dominant[0]}, ${dominant[1]}, ${dominant[2]}`],
              ["Histogram bins", histogram.length],
              ["Layer palette", `${palette.length / 4} colors`],
            ],
            views: [
              rgbaView("Original", context.fixture.rgba, context.fixture.width, context.fixture.height, {
                maxDisplay: 300,
              }),
              rgbaView("quantizeLayer()", quantizedLayer, context.fixture.width, context.fixture.height, {
                maxDisplay: 300,
              }),
              rgbaView("quantizeRgba()", topQuantized, context.fixture.width, context.fixture.height, {
                maxDisplay: 300,
              }),
              swatchView("Layer palette", palette),
              swatchView("Top-level palette", topPalette),
            ],
          };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "The first sheet should show a loud display headline plus centered rotation. The second sheet should make Bodoni weight, italic style, and left/center/right alignment obvious with wrapped cupcake ipsum.",
      featured: true,
      fullSpan: true,
      previewMin: 250,
      section: "text",
      title: "Display Font Text Layers",
      async run(context) {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 320, height: 168 });
        const styleComposition = await Composition.create({ width: 320, height: 176 });

        try {
          const loadedFaces =
            (await registerFont({
              bytes: context.bungeeKimgWoff2,
              family: "Bungee",
              style: "normal",
              weight: 400,
            })) +
            (await registerFont({
              bytes: context.bodoniModaRegularWoff2,
              family: "Bodoni Moda",
              style: "normal",
              weight: 400,
            })) +
            (await registerFont({
              bytes: context.bodoniModaItalicWoff2,
              family: "Bodoni Moda",
              style: "italic",
              weight: 400,
            }));
          composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
          composition.addShapeLayer({
            fill: [228, 113, 76, 28],
            height: 112,
            name: "backplate",
            stroke: { color: [228, 113, 76, 96], width: 3 },
            type: "rectangle",
            width: 128,
            x: 14,
            y: 18,
          });

          const headlineId = composition.addTextLayer({
            color: [201, 73, 45, 255],
            fontFamily: "Bungee",
            fontSize: 24,
            letterSpacing: 2,
            lineHeight: 28,
            name: "headline",
            text: "HELLO",
            x: 28,
            y: 32,
          });
          const badgeId = composition.addTextLayer({
            color: [24, 77, 163, 255],
            fontFamily: "Bungee",
            fontSize: 16,
            letterSpacing: 1,
            lineHeight: 20,
            name: "badge",
            text: "KIMG\nTEXT",
            x: 236,
            y: 98,
          });

          composition.updateLayer(badgeId, {
            anchor: "center",
            rotation: -12,
            textConfig: {
              color: [35, 79, 221, 255],
              fontFamily: "Bungee",
              fontSize: 24,
              letterSpacing: 2,
              lineHeight: 28,
              text: "KIMG\nTEXT",
            },
          });

          const layers = composition.listLayers();
          const headline = composition.getLayer(headlineId);
          const badge = composition.getLayer(badgeId);
          const render = composition.renderRgba();

          styleComposition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
          const cupcakeText = "Cupcake ipsum\ndolor sit amet\nfrosting";
          const panelSpecs = [
            {
              align: "left",
              color: [201, 73, 45, 255],
              fontStyle: "normal",
              fontWeight: 400,
              label: "left / 400",
              x: 16,
            },
            {
              align: "center",
              color: [35, 79, 221, 255],
              fontStyle: "normal",
              fontWeight: 900,
              label: "center / 900",
              x: 116,
            },
            {
              align: "right",
              color: [24, 119, 92, 255],
              fontStyle: "italic",
              fontWeight: 400,
              label: "right / italic",
              x: 216,
            },
          ];

          const styleLayerIds = [];
          for (const spec of panelSpecs) {
            styleComposition.addShapeLayer({
              fill: [0, 0, 0, 0],
              height: 136,
              name: `${spec.label}-panel`,
              stroke: { color: [120, 112, 101, 90], width: 2 },
              type: "rectangle",
              width: 88,
              x: spec.x,
              y: 16,
            });
            styleLayerIds.push(
              styleComposition.addTextLayer({
                align: spec.align,
                boxWidth: 72,
                color: spec.color,
                fontFamily: "Bodoni Moda",
                fontSize: 17,
                fontStyle: spec.fontStyle,
                fontWeight: spec.fontWeight,
                lineHeight: 23,
                name: spec.label,
                text: cupcakeText,
                wrap: "word",
                x: spec.x + 8,
                y: 28,
              }),
            );
          }

          const styleLayers = styleComposition.listLayers();
          const styleRender = styleComposition.renderRgba();
          const [leftText, centerText, rightText] = styleLayerIds.map((id) => styleComposition.getLayer(id));

          verify.equal(
            layers.filter((layer) => layer.kind === "text").length,
            2,
            "expected two text layers in metadata",
          );
          verify.ok(loadedFaces > 0, "registerFont should load at least one usable face");
          verify.equal(headline?.text, "HELLO", "headline text should stay readable in metadata");
          verify.equal(badge?.anchor, "center", "rotated text should switch to center anchor");
          verify.equal(badge?.letterSpacing, 2, "textConfig update should widen tracking");
          verify.equal(
            styleLayers.filter((layer) => layer.kind === "text").length,
            3,
            "expected three styled text layers in the second composition",
          );
          verify.equal(leftText?.align, "left", "left panel should keep left alignment");
          verify.equal(centerText?.fontWeight, 900, "center panel should use heavier weight");
          verify.equal(rightText?.fontStyle, "italic", "right panel should keep italic style");
          verify.ok(render.some((value) => value !== 0), "text composition should render non-empty pixels");
          verify.ok(styleRender.some((value) => value !== 0), "styled text composition should render non-empty pixels");

          return {
            assertions: verify.count,
            metrics: [
              [
                "Text layers",
                `${
                  layers.filter((layer) => layer.kind === "text").length +
                  styleLayers.filter((layer) => layer.kind === "text").length
                }`,
              ],
              ["Registered faces", loadedFaces],
              ["display families", "Bungee + Bodoni Moda"],
              ["headline size", headline?.fontSize ?? "n/a"],
              ["badge tracking", badge?.letterSpacing ?? "n/a"],
              ["badge rotation", badge?.rotation?.toFixed(1) ?? "n/a"],
              ["styled columns", `${leftText?.align} / ${centerText?.fontWeight} / ${rightText?.fontStyle}`],
            ],
            views: [
              rgbaView("Transform and tracking", render, composition.width, composition.height),
              rgbaView(
                "Weight, italics, and alignment",
                styleRender,
                styleComposition.width,
                styleComposition.height,
              ),
            ],
          };
        } finally {
          composition.free();
          styleComposition.free();
        }
      },
    },
    {
      expectation:
        "The mocked Google Fonts helper should register a theatrical serif family and render regular plus italic Bodoni columns without manual byte registration.",
      fullSpan: true,
      previewMin: 250,
      section: "text",
      title: "Google Fonts Display Loader",
      async run(context) {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 320, height: 124 });
        const originalFetch = globalThis.fetch;
        const regularUrl = "https://fonts.gstatic.com/mock/bodoni-moda-regular.woff2";
        const italicUrl = "https://fonts.gstatic.com/mock/bodoni-moda-italic.woff2";
        const css = [
          "/* latin */",
          "@font-face {",
          "  font-family: 'Bodoni Moda';",
          "  font-style: normal;",
          "  font-weight: 400;",
          "  font-display: swap;",
          `  src: url(${regularUrl}) format('woff2');`,
          "}",
          "/* latin */",
          "@font-face {",
          "  font-family: 'Bodoni Moda';",
          "  font-style: italic;",
          "  font-weight: 400;",
          "  font-display: swap;",
          `  src: url(${italicUrl}) format('woff2');`,
          "}",
        ].join("\n");

        try {
          await clearRegisteredFonts();
          globalThis.fetch = async (input, init) => {
            const url =
              typeof input === "string" ? input : input instanceof URL ? input.href : String(input);
            if (url.startsWith("https://fonts.googleapis.com/css2?")) {
              return new Response(css, {
                headers: { "content-type": "text/css; charset=utf-8" },
                status: 200,
              });
            }

            if (url === regularUrl) {
              return new Response(context.bodoniModaRegularWoff2, {
                headers: { "content-type": "font/woff2" },
                status: 200,
              });
            }

            if (url === italicUrl) {
              return new Response(context.bodoniModaItalicWoff2, {
                headers: { "content-type": "font/woff2" },
                status: 200,
              });
            }

            return originalFetch(input, init);
          };

          const loaded = await loadGoogleFont({
            family: "Bodoni Moda",
            ital: [0, 1],
            text: "Cupcakeipsumdolorsitamet",
            weights: [400],
          });

          composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
          composition.addShapeLayer({
            fill: [0, 0, 0, 0],
            height: 92,
            name: "regular-panel",
            stroke: { color: [120, 112, 101, 90], width: 2 },
            type: "rectangle",
            width: 132,
            x: 16,
            y: 16,
          });
          composition.addShapeLayer({
            fill: [0, 0, 0, 0],
            height: 92,
            name: "italic-panel",
            stroke: { color: [120, 112, 101, 90], width: 2 },
            type: "rectangle",
            width: 132,
            x: 172,
            y: 16,
          });

          const regularId = composition.addTextLayer({
            align: "left",
            boxWidth: 108,
            color: [201, 73, 45, 255],
            fontFamily: "Bodoni Moda",
            fontSize: 17,
            fontWeight: 400,
            lineHeight: 23,
            name: "google-regular",
            text: "Cupcake ipsum\ndolor sit amet",
            wrap: "word",
            x: 28,
            y: 28,
          });
          const italicId = composition.addTextLayer({
            align: "left",
            boxWidth: 108,
            color: [35, 79, 221, 255],
            fontFamily: "Bodoni Moda",
            fontSize: 17,
            fontStyle: "italic",
            fontWeight: 400,
            lineHeight: 23,
            name: "google-italic",
            text: "Cupcake ipsum\ndolor sit amet",
            wrap: "word",
            x: 184,
            y: 28,
          });

          const render = composition.renderRgba();
          const regular = composition.getLayer(regularId);
          const italic = composition.getLayer(italicId);

          verify.equal(loaded.family, "Bodoni Moda", "loader should report the requested family");
          verify.equal(loaded.faces.length, 2, "mocked stylesheet should expose two faces");
          verify.ok(loaded.registeredFaces >= 2, "loadGoogleFont should register both faces");
          verify.equal(regular?.fontWeight, 400, "regular text should keep weight 400");
          verify.equal(italic?.fontStyle, "italic", "italic text should keep italic style");
          verify.ok(render.some((value) => value !== 0), "Google Fonts composition should render non-empty pixels");

          return {
            assertions: verify.count,
            metrics: [
              ["Loader faces", loaded.faces.length],
              ["Registered faces", loaded.registeredFaces],
              ["Regular weight", regular?.fontWeight ?? "n/a"],
              ["Italic style", italic?.fontStyle ?? "n/a"],
            ],
            views: [rgbaView("Mocked CSS2 font load", render, composition.width, composition.height)],
          };
        } finally {
          globalThis.fetch = originalFetch;
          await clearRegisteredFonts();
          composition.free();
        }
      },
    },
    {
      expectation:
        "The retained SVG should stay crisp under scaling and rotation, and the explicit rasterized copy should preserve the same overall silhouette after conversion.",
      fullSpan: true,
      previewMin: 250,
      section: "io",
      title: "SVG Layer Import and Rasterize",
      async run() {
        const verify = createVerifier();
        const retained = await Composition.create({ width: 208, height: 132 });
        const rasterized = await Composition.create({ width: 208, height: 132 });

        try {
          retained.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
          rasterized.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });

          const retainedId = retained.addSvgLayer({
            name: "logo",
            svg: SIMPLE_SVG,
            width: 96,
            height: 96,
            x: 56,
            y: 18,
          });
          const rasterizedId = rasterized.addSvgLayer({
            name: "logo",
            svg: SIMPLE_SVG,
            width: 96,
            height: 96,
            x: 56,
            y: 18,
          });

          retained.updateLayer(retainedId, {
            anchor: "center",
            rotation: -14,
            scaleX: 1.45,
            scaleY: 1.45,
          });
          rasterized.updateLayer(rasterizedId, {
            anchor: "center",
            rotation: -14,
            scaleX: 1.45,
            scaleY: 1.45,
          });

          const retainedInfo = retained.getLayer(retainedId);
          const retainedLocal = retained.getLayerRgba(retainedId);
          verify.equal(retainedInfo?.kind, "svg", "retained layer should stay svg-backed before rasterize");
          verify.equal(retainedLocal.length, 96 * 96 * 4, "getLayerRgba should rasterize the local SVG bounds");

          verify.equal(rasterized.rasterizeSvgLayer(rasterizedId), true, "rasterizeSvgLayer should succeed");
          const rasterizedInfo = rasterized.getLayer(rasterizedId);
          verify.equal(rasterizedInfo?.kind, "raster", "rasterized layer should become a raster layer");
          verify.ok(retained.renderRgba().some((value) => value !== 0), "retained svg composition should render");
          verify.ok(rasterized.renderRgba().some((value) => value !== 0), "rasterized svg composition should render");

          return {
            assertions: verify.count,
            metrics: [
              ["Retained kind", retainedInfo?.kind ?? "n/a"],
              ["Rasterized kind", rasterizedInfo?.kind ?? "n/a"],
              ["Local bounds", `${retainedInfo?.width ?? "?"}x${retainedInfo?.height ?? "?"}`],
              ["Transform", `${retainedInfo?.scaleX?.toFixed(2) ?? "n/a"}x @ ${retainedInfo?.rotation?.toFixed(1) ?? "n/a"}°`],
            ],
            views: [
              rgbaView("Retained SVG layer", retained.renderRgba(), retained.width, retained.height),
              rgbaView("After rasterizeSvgLayer()", rasterized.renderRgba(), rasterized.width, rasterized.height),
            ],
          };
        } finally {
          retained.free();
          rasterized.free();
        }
      },
    },
    {
      expectation:
        "Pixel scaling should keep edges crisp, the atlas should pack each sprite once, and the contact sheet should lay them out in a regular grid.",
      featured: true,
      fullSpan: true,
      previewMin: 250,
      section: "io",
      title: "Sprite Helpers and Contact Sheet",
      async run(context) {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 196, height: 96 });

        try {
          const sprites = context.spriteFixtures.map((sprite, index) =>
            composition.addImageLayer({
              height: sprite.height,
              name: `sprite-${index + 1}`,
              rgba: sprite.rgba,
              width: sprite.width,
              x: 8 + index * 44,
              y: 10,
            }),
          );

          composition.pixelScaleLayer(sprites[0], { factor: 4 });
          const scaled = composition.getLayerRgba(sprites[0]);
          const atlasJson = JSON.parse(composition.packSpritesJson({ layerIds: sprites, maxWidth: 128, padding: 6 }));
          const atlas = composition.packSprites({ layerIds: sprites, maxWidth: 128, padding: 6 });
          const sheet = composition.contactSheet({
            background: [0, 0, 0, 0],
            columns: 2,
            layerIds: sprites,
            padding: 8,
          });

          verify.equal(atlasJson.sprites.length, sprites.length, "atlas json should describe every sprite");
          verify.equal(scaled.length, 32 * 32 * 4, "pixelScaleLayer should upscale the first sprite to 32x32");
          verify.ok(atlasJson.width >= 32, "atlas width should reflect the packed sprite sizes");
          verify.ok(atlasJson.height >= 32, "atlas height should reflect the packed sprite sizes");

          return {
            assertions: verify.count,
            metrics: [
              ["Scaled sprite", "32x32"],
              ["Atlas", `${atlasJson.width}x${atlasJson.height}`],
              ["Packed entries", atlasJson.sprites.length],
              ["Contact sheet", "72x72"],
            ],
          views: [
            rgbaView("pixelScaleLayer()", scaled, 32, 32, { maxDisplay: 220 }),
            rgbaView("packSprites()", atlas, atlasJson.width, atlasJson.height, { maxDisplay: 260 }),
            rgbaView("contactSheet()", sheet, 72, 72, { maxDisplay: 220 }),
            codeView("packSpritesJson()", JSON.stringify(atlasJson, null, 2)),
          ],
        };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "The deserialized composition should match the original exactly, and both PNG import paths should reproduce the same visual layer.",
      section: "io",
      title: "Serialization, addPngLayer, importImage",
      async run(context) {
        const verify = createVerifier();
        const source = await buildExportScene(context);

        try {
          const serialized = source.serialize();
          const roundTripped = await Composition.deserialize(serialized);

          try {
            const before = source.renderRgba();
            const after = roundTripped.renderRgba();
            const exportedPng = source.exportPng();
            const addPngComp = await Composition.create({ width: source.width, height: source.height });
            const importPngComp = await Composition.create({ width: source.width, height: source.height });

            try {
              addPngComp.addPngLayer({ name: "from-export", png: exportedPng });
              importPngComp.importImage({ bytes: context.fixture.pngBytes, name: "from-fixture" });

              verify.ok(rgbaEquals(before, after), "serialize/deserialize should preserve the render");
              verify.equal((await detectFormat(exportedPng)), "png", "exportPng should detect as png");

              return {
                assertions: verify.count,
                metrics: [
                  ["Serialized bytes", serialized.length.toLocaleString()],
                  ["exportPng()", `${exportedPng.length.toLocaleString()} bytes`],
                ],
                views: [
                  rgbaView("Original", before, source.width, source.height),
                  rgbaView("deserialize()", after, roundTripped.width, roundTripped.height),
                  rgbaView("addPngLayer()", addPngComp.renderRgba(), addPngComp.width, addPngComp.height),
                  rgbaView("importImage()", importPngComp.renderRgba(), importPngComp.width, importPngComp.height),
                ],
              };
            } finally {
              addPngComp.free();
              importPngComp.free();
            }
          } finally {
            roundTripped.free();
          }
        } finally {
          source.free();
        }
      },
    },
    {
      expectation:
        "JPEG and WebP round trips should stay recognizably close to the source, with format detection reflecting the exported bytes.",
      section: "io",
      title: "JPEG and WebP Export / Import",
      async run(context) {
        const verify = createVerifier();
        const source = await buildExportScene(context);

        try {
          const original = source.renderRgba();
          const jpeg = source.exportJpeg({ quality: 82 });
          const webp = source.exportWebp();

          const jpegComp = await Composition.create({ width: source.width, height: source.height });
          const webpComp = await Composition.create({ width: source.width, height: source.height });

          try {
            jpegComp.importJpeg({ bytes: jpeg, name: "jpeg-roundtrip" });
            webpComp.importWebp({ bytes: webp, name: "webp-roundtrip" });

            verify.equal(await detectFormat(jpeg), "jpeg", "exportJpeg should detect as jpeg");
            verify.equal(await detectFormat(webp), "webp", "exportWebp should detect as webp");

            return {
              assertions: verify.count,
              metrics: [
                ["JPEG bytes", jpeg.length.toLocaleString()],
                ["WebP bytes", webp.length.toLocaleString()],
              ],
              views: [
                rgbaView("Original", original, source.width, source.height),
                rgbaView("importJpeg()", jpegComp.renderRgba(), jpegComp.width, jpegComp.height),
                rgbaView("importWebp()", webpComp.renderRgba(), webpComp.width, webpComp.height),
              ],
            };
          } finally {
            jpegComp.free();
            webpComp.free();
          }
        } finally {
          source.free();
        }
      },
    },
    {
      expectation:
        "A valid GIF should import as one or more layers. The card uses a tiny embedded sample and scales it up so failures are obvious.",
      section: "io",
      title: "GIF Frame Import",
      async run() {
        const verify = createVerifier();
        const composition = await Composition.create({ width: 128, height: 92 });

        try {
          const gifBytes = decodeBase64("R0lGODlhAQABAIABAP8AAP///yH5BAEAAAEALAAAAAABAAEAAAICRAEAOw==");
          const format = await detectFormat(gifBytes);
          const ids = composition.importGifFrames({ bytes: gifBytes });
          composition.updateLayer(ids[0], {
            scaleX: 56,
            scaleY: 56,
            x: 22,
            y: 18,
          });

          verify.equal(format, "gif", "embedded sample should detect as gif");
          verify.ok(ids.length >= 1, "importGifFrames should add at least one layer");

          return {
            assertions: verify.count,
            metrics: [
              ["detectFormat()", format],
              ["imported layers", ids.length],
            ],
            views: [rgbaView("Scaled GIF frame", composition.renderRgba(), composition.width, composition.height)],
          };
        } finally {
          composition.free();
        }
      },
    },
    {
      expectation:
        "Utility outputs should agree on basic color conversions, luminance, contrast, and base64 round trips.",
      featured: true,
      fullSpan: true,
      previewMin: 250,
      section: "io",
      title: "Package Utilities and Subpaths",
      async run(context) {
        const verify = createVerifier();
        const rgb = await hexToRgb("#234fdd");
        const hex = await rgbToHex({ r: 35, g: 79, b: 221 });
        const luminance = await relativeLuminance("#ffffff");
        const ratio = await contrastRatio("#ffffff", "#1d1c1a");
        const base64 = rgbaToBase64(context.utilityTile.rgba);
        const roundTrip = base64ToRgba(base64);
        const readable = readableTextColor("#234fdd");

        verify.equal(hex, "#234fdd", "rgbToHex should round-trip the accent blue");
        verify.equal(rgb.length, 3, "hexToRgb should return three bytes");
        verify.ok(luminance > 0.9, "white luminance should stay high");
        verify.ok(ratio > 10, "contrast ratio should remain strong");
        verify.ok(rgbaEquals(roundTrip, context.utilityTile.rgba), "base64 round trip should preserve bytes");
        verify.equal(readable, "#ffffff", "readableTextColor should choose white on blue");

        return {
          assertions: verify.count,
          metrics: [
            ["hexToRgb()", `${rgb[0]}, ${rgb[1]}, ${rgb[2]}`],
            ["rgbToHex()", hex],
            ["relativeLuminance()", luminance.toFixed(3)],
            ["contrastRatio()", ratio.toFixed(2)],
            ["readableTextColor()", readable],
          ],
          views: [
            swatchView("Utility tile", context.utilityPalette),
            rgbaView("base64 round trip", roundTrip, context.utilityTile.width, context.utilityTile.height, {
              maxDisplay: 240,
            }),
            codeView("Base64 sample", base64.slice(0, 88) + "..."),
          ],
        };
      },
    },
    {
      expectation:
        "PSD stays visible on the page as experimental only. This card is intentionally excluded from stable pass/fail accounting.",
      experimental: true,
      section: "experimental",
      title: "PSD Import Placeholder",
      async run() {
        const verify = createVerifier();
        const header = new Uint8Array([0x38, 0x42, 0x50, 0x53, 0x00, 0x01]);
        const format = await detectFormat(header);
        verify.equal(format, "psd", "PSD header bytes should still detect as psd");

        return {
          assertions: verify.count,
          metrics: [
            ["detectFormat()", format],
            ["Status", "Experimental only; no stable fixture included"],
          ],
          note:
            "PSD import is intentionally out of the stable visual pass set. The path remains labeled experimental in docs and demo UI.",
          views: [
            messageView(
              "Experimental note",
              "No PSD render fixture is shipped with the suite. This keeps the unstable parser visible without turning it into a release gate.",
            ),
          ],
        };
      },
    },
  ];
}

function resolveDemoPreloadInput() {
  const mode = new URLSearchParams(window.location.search).get("wasm");
  if (mode === "baseline") {
    return { module_or_path: new URL("../dist/kimg_wasm_bg.wasm", import.meta.url) };
  }
  if (mode === "simd") {
    return { module_or_path: new URL("../dist/kimg_wasm_simd_bg.wasm", import.meta.url) };
  }
  return undefined;
}

async function buildContext() {
  const fixture = await loadTeapotFixture("./assets/teapot.png", 192);
  const bodoniModaItalicWoff2 = await loadBinaryFixture("./assets/bodoni-moda-italic.woff2");
  const bodoniModaRegularWoff2 = await loadBinaryFixture("./assets/bodoni-moda-regular.woff2");
  const bungeeKimgWoff2 = await loadBinaryFixture("./assets/bungee-kimg.woff2");
  const filterFixture = createFilterFixture();
  const glyph = createGlyphFixture();
  const borderedGlyph = createBorderedFixture(glyph);
  const clipPattern = createClipPatternFixture();
  const utilityTile = createUtilityTile();

  return {
    bodoniModaItalicWoff2,
    bodoniModaRegularWoff2,
    borderedGlyph,
    bungeeKimgWoff2,
    clipPattern,
    filterFixture,
    fixture,
    glyph,
    runtime: {
      simd: simdSupported(),
    },
    spriteFixtures: createSpriteFixtures(),
    utilityPalette: [await hexToRgb("#234fdd"), await hexToRgb("#c9492d"), await hexToRgb("#157347")].flatMap(
      (rgb) => [rgb[0], rgb[1], rgb[2], 255],
    ),
    utilityTile,
  };
}

async function buildLayerSurgeryScene(context, mutate) {
  const composition = await Composition.create({ width: 232, height: 164 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const groupId = composition.addGroupLayer({ name: "cluster" });
  const redId = composition.addImageLayer({
    height: context.glyph.height,
    name: "red",
    parentId: groupId,
    rgba: tintFixture(context.glyph, [201, 73, 45]),
    width: context.glyph.width,
    x: 20,
    y: 24,
  });
  const blueId = composition.addImageLayer({
    height: context.glyph.height,
    name: "blue",
    parentId: groupId,
    rgba: tintFixture(context.glyph, [35, 79, 221]),
    width: context.glyph.width,
    x: 86,
    y: 42,
  });
  const yellowId = composition.addImageLayer({
    height: context.glyph.height,
    name: "yellow",
    rgba: tintFixture(context.glyph, [228, 181, 64]),
    width: context.glyph.width,
    x: 152,
    y: 34,
  });
  const ghostId = composition.addShapeLayer({
    fill: [18, 18, 18, 210],
    height: 12,
    name: "ghost",
    type: "rectangle",
    width: 120,
    x: 96,
    y: 138,
  });

  if (mutate) {
    composition.moveLayer(yellowId, { index: 1, parentId: groupId });
    composition.updateLayer(yellowId, {
      x: 108,
      y: 34,
    });
    composition.removeFromGroup(groupId, blueId);
    composition.removeLayer(ghostId);
    composition.flattenGroup(groupId);
    composition.resizeCanvas({ width: 296, height: 184 });
  }

  return {
    dispose() {
      composition.free();
    },
    height: composition.height,
    layerCount: composition.layerCount(),
    rgba: composition.renderRgba(),
    width: composition.width,
  };
}

async function buildClipScene(context) {
  const composition = await Composition.create({ width: 236, height: 176 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  composition.addShapeLayer({
    fill: [255, 255, 255, 255],
    height: 134,
    name: "clip-shape",
    type: "ellipse",
    width: 144,
    x: 46,
    y: 18,
  });
  const patternId = composition.addImageLayer({
    height: context.clipPattern.height,
    name: "pattern",
    rgba: context.clipPattern.rgba,
    width: context.clipPattern.width,
    x: 34,
    y: 18,
  });
  composition.setLayerClipToBelow(patternId, true);
  composition.addShapeLayer({
    fill: [255, 255, 255, 0],
    height: 134,
    name: "outline",
    stroke: { color: [24, 24, 24, 255], width: 4 },
    type: "ellipse",
    width: 144,
    x: 46,
    y: 18,
  });

  return {
    dispose() {
      composition.free();
    },
    height: composition.height,
    layerCount: composition.layerCount(),
    rgba: composition.renderRgba(),
    width: composition.width,
  };
}

async function buildMaskStates(context) {
  const composition = await Composition.create({ width: 236, height: 176 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const imageId = composition.addImageLayer({
    height: context.clipPattern.height,
    name: "pattern",
    rgba: context.clipPattern.rgba,
    width: context.clipPattern.width,
    x: 34,
    y: 18,
  });
  const mask = createMaskFixture(context.clipPattern.width, context.clipPattern.height);
  composition.setLayerMask(imageId, {
    height: mask.height,
    rgba: mask.rgba,
    width: mask.width,
  });

  const masked = {
    hasMask: composition.getLayer(imageId)?.hasMask,
    height: composition.height,
    rgba: composition.renderRgba(),
    width: composition.width,
  };

  composition.setLayerMaskInverted(imageId, true);
  const inverted = {
    hasMask: composition.getLayer(imageId)?.hasMask,
    height: composition.height,
    maskInverted: composition.getLayer(imageId)?.maskInverted,
    rgba: composition.renderRgba(),
    width: composition.width,
  };

  composition.clearLayerMask(imageId);
  const cleared = {
    hasMask: composition.getLayer(imageId)?.hasMask,
    height: composition.height,
    rgba: composition.renderRgba(),
    width: composition.width,
  };

  composition.free();
  return { cleared, inverted, masked };
}

async function buildTransformSetterVariant(context, mode) {
  const composition = await Composition.create({ width: 124, height: 124 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });

  if (mode === "rotated") {
    composition.addShapeLayer({
      fill: [24, 24, 24, 48],
      height: 2,
      name: "guide-horizontal",
      type: "rectangle",
      width: 92,
      x: 16,
      y: 61,
    });
    composition.addShapeLayer({
      fill: [24, 24, 24, 48],
      height: 92,
      name: "guide-vertical",
      type: "rectangle",
      width: 2,
      x: 61,
      y: 16,
    });
    composition.addShapeLayer({
      fill: [201, 73, 45, 180],
      height: 10,
      name: "pivot-dot",
      type: "ellipse",
      width: 10,
      x: 57,
      y: 57,
    });
  }

  const layerId = composition.addImageLayer({
    height: context.glyph.height,
    name: mode,
    rgba: context.glyph.rgba,
    width: context.glyph.width,
    x: 14,
    y: 14,
  });

  if (mode === "baseline") {
    composition.setLayerOpacity(layerId, 0.94);
  } else if (mode === "flipX") {
    composition.setLayerFlip(layerId, { flipX: true });
  } else if (mode === "flipY") {
    composition.setLayerFlip(layerId, { flipY: true });
  } else if (mode === "rotated") {
    composition.setLayerAnchor(layerId, "center");
    composition.setLayerRotation(layerId, 34);
    composition.setLayerPosition(layerId, { x: 62, y: 62 });
  }

  const result = {
    height: composition.height,
    layer: composition.getLayer(layerId),
    rgba: composition.renderRgba(),
    width: composition.width,
  };
  composition.free();
  return result;
}

async function buildPatchedTransformScene(context, mutate) {
  const composition = await Composition.create({ width: 248, height: 172 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  const imageId = composition.addImageLayer({
    height: context.fixture.height,
    name: "teapot",
    rgba: context.fixture.rgba,
    width: context.fixture.width,
    x: 26,
    y: 14,
  });

  if (mutate) {
    composition.updateLayer(imageId, {
      anchor: "center",
      flipX: true,
      opacity: 0.84,
      rotation: 18,
      scaleX: 0.82,
      scaleY: 1.14,
      x: 178,
      y: 96,
    });
  }

  return {
    dispose() {
      composition.free();
    },
    height: composition.height,
    layer: composition.getLayer(imageId),
    rgba: composition.renderRgba(),
    width: composition.width,
  };
}

async function buildResampleVariants(context) {
  const nearest = await mutateGlyphLayer(context, (composition, layerId) => {
    composition.resizeLayerNearest(layerId, { width: 144, height: 144 });
  });
  const bilinear = await mutateGlyphLayer(context, (composition, layerId) => {
    composition.resizeLayerBilinear(layerId, { width: 144, height: 144 });
  });
  const lanczos = await mutateGlyphLayer(context, (composition, layerId) => {
    composition.resizeLayerLanczos3(layerId, { width: 144, height: 144 });
  });
  const cropped = await mutateGlyphLayer(context, (composition, layerId) => {
    composition.cropLayer(layerId, { height: 48, width: 48, x: 24, y: 24 });
  });
  const trimmed = await mutateBorderedGlyph(context, (composition, layerId) => {
    composition.trimLayerAlpha(layerId);
  });
  const rotated = await mutateGlyphLayer(context, (composition, layerId) => {
    composition.rotateLayer(layerId, 33);
  });

  return { bilinear, cropped, lanczos, nearest, rotated, trimmed };
}

async function buildScopedFilterScene(context, withFilter) {
  const composition = await Composition.create({ width: 292, height: 168 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  composition.addShapeLayer({
    fill: [18, 18, 18, 18],
    height: 126,
    name: "control-frame",
    stroke: { color: [18, 18, 18, 72], width: 2 },
    type: "rectangle",
    width: 118,
    x: 14,
    y: 20,
  });
  composition.addImageLayer({
    height: context.filterFixture.height,
    name: "outside-control",
    rgba: context.filterFixture.rgba,
    width: context.filterFixture.width,
    x: 20,
    y: 26,
  });

  const groupId = composition.addGroupLayer({ name: "subject-group" });
  composition.addShapeLayer({
    fill: [35, 79, 221, 18],
    height: 126,
    name: "subject-frame",
    stroke: { color: [35, 79, 221, 84], width: 2 },
    parentId: groupId,
    type: "rectangle",
    width: 118,
    x: 158,
    y: 20,
  });
  composition.addImageLayer({
    height: context.filterFixture.height,
    name: "inside-subject",
    parentId: groupId,
    rgba: context.filterFixture.rgba,
    width: context.filterFixture.width,
    x: 164,
    y: 26,
  });

  const filterId = composition.addFilterLayer({ name: "group-filter", parentId: groupId });
  if (withFilter) {
    composition.setFilterLayerConfig(filterId, {
      brightness: 0.18,
      contrast: 0.46,
      hueDeg: 118,
      saturation: 0.28,
      temperature: -0.22,
      tint: 0.18,
    });
  }

  return {
    dispose() {
      composition.free();
    },
    filter: composition.getLayer(filterId),
    height: composition.height,
    rgba: composition.renderRgba(),
    width: composition.width,
  };
}

async function buildFilterVariants(context) {
  const operations = [
    ["Original", (composition, id) => composition.updateLayer(id, {})],
    ["boxBlur", (composition, id) => composition.boxBlurLayer(id, { radius: 4 })],
    ["gaussianBlur", (composition, id) => composition.gaussianBlurLayer(id, { radius: 4 })],
    ["sharpen", (composition, id) => composition.sharpenLayer(id)],
    ["edgeDetect", (composition, id) => composition.edgeDetectLayer(id)],
    ["emboss", (composition, id) => composition.embossLayer(id)],
    ["invert", (composition, id) => composition.invertLayer(id)],
    ["posterize", (composition, id) => composition.posterizeLayer(id, { levels: 3 })],
    ["threshold", (composition, id) => composition.thresholdLayer(id, { threshold: 150 })],
    ["levels", (composition, id) =>
      composition.levelsLayer(id, { highlights: 0.88, midtones: 0.64, shadows: 0.18 })],
    ["gradientMap", (composition, id) =>
      composition.gradientMapLayer(id, {
        stops: [
          { color: [14, 25, 73, 255], position: 0 },
          { color: [255, 224, 132, 255], position: 1 },
        ],
      })],
  ];

  const variants = [];
  for (const [label, operation] of operations) {
    variants.push(await mutateTeapotLayer(context, operation, label));
  }
  return variants;
}

async function buildFillVariant(context, mode) {
  const composition = await Composition.create({ width: 156, height: 116 });
  composition.addSolidColorLayer({ color: [248, 243, 232, 255], name: "paper" });
  const pattern = createFillFixture(mode);
  const layerId = composition.addImageLayer({
    height: pattern.height,
    name: `${mode}-fill`,
    rgba: pattern.rgba,
    width: pattern.width,
    x: 18,
    y: 16,
  });

  if (mode === "contiguous") {
    composition.bucketFillLayer(layerId, {
      color: [35, 79, 221, 255],
      contiguous: true,
      tolerance: 0,
      x: 20,
      y: 20,
    });
  } else if (mode === "non-contiguous") {
    composition.bucketFillLayer(layerId, {
      color: [201, 73, 45, 255],
      contiguous: false,
      tolerance: 0,
      x: 20,
      y: 20,
    });
  } else {
    composition.bucketFillLayer(layerId, {
      color: [24, 140, 93, 255],
      contiguous: true,
      tolerance: 20,
      x: 20,
      y: 20,
    });
  }

  const result = {
    height: composition.height,
    rgba: composition.renderRgba(),
    width: composition.width,
  };
  composition.free();
  return result;
}

async function buildExportScene(context) {
  const composition = await Composition.create({ width: 264, height: 184 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  composition.addGradientLayer({
    direction: "diagonalUp",
    name: "shade",
    stops: [
      { color: [255, 255, 255, 0], position: 0 },
      { color: [35, 79, 221, 42], position: 1 },
    ],
  });
  composition.addImageLayer({
    height: context.fixture.height,
    name: "teapot",
    rgba: context.fixture.rgba,
    width: context.fixture.width,
    x: 38,
    y: 8,
  });
  composition.addShapeLayer({
    fill: [201, 73, 45, 32],
    height: 110,
    name: "halo",
    stroke: { color: [201, 73, 45, 110], width: 3 },
    type: "ellipse",
    width: 110,
    x: 144,
    y: 26,
  });
  return composition;
}

async function mutateGlyphLayer(context, mutation) {
  const composition = await Composition.create({ width: context.glyph.width, height: context.glyph.height });
  const layerId = composition.addImageLayer({
    height: context.glyph.height,
    name: "glyph",
    rgba: context.glyph.rgba,
    width: context.glyph.width,
  });
  mutation(composition, layerId);
  const info = composition.getLayer(layerId);
  const result = {
    height: info?.height ?? context.glyph.height,
    rgba: composition.getLayerRgba(layerId),
    width: info?.width ?? context.glyph.width,
  };
  composition.free();
  return result;
}

async function mutateBorderedGlyph(context, mutation) {
  const composition = await Composition.create({
    width: context.borderedGlyph.width,
    height: context.borderedGlyph.height,
  });
  const layerId = composition.addImageLayer({
    height: context.borderedGlyph.height,
    name: "bordered-glyph",
    rgba: context.borderedGlyph.rgba,
    width: context.borderedGlyph.width,
  });
  mutation(composition, layerId);
  const info = composition.getLayer(layerId);
  const result = {
    height: info?.height ?? context.borderedGlyph.height,
    rgba: composition.getLayerRgba(layerId),
    width: info?.width ?? context.borderedGlyph.width,
  };
  composition.free();
  return result;
}

async function mutateFilterLayer(context, mutation, label) {
  const composition = await Composition.create({
    width: context.filterFixture.width,
    height: context.filterFixture.height,
  });
  const layerId = composition.addImageLayer({
    height: context.filterFixture.height,
    name: label,
    rgba: context.filterFixture.rgba,
    width: context.filterFixture.width,
  });
  mutation(composition, layerId);
  const result = {
    height: context.filterFixture.height,
    label,
    rgba: composition.getLayerRgba(layerId),
    width: context.filterFixture.width,
  };
  composition.free();
  return result;
}

async function mutateTeapotLayer(context, mutation, label) {
  const composition = await Composition.create({ width: context.fixture.width, height: context.fixture.height });
  const layerId = composition.addImageLayer({
    height: context.fixture.height,
    name: label,
    rgba: context.fixture.rgba,
    width: context.fixture.width,
  });
  mutation(composition, layerId);
  const result = {
    height: context.fixture.height,
    label,
    rgba: composition.getLayerRgba(layerId),
    width: context.fixture.width,
  };
  composition.free();
  return result;
}

function createCard(test) {
  const section = getSectionNode(test.section);
  const article = document.createElement("article");
  article.className = "test-card";
  if (test.featured) {
    article.classList.add("is-featured");
  }
  if (test.fullSpan) {
    article.classList.add("is-full");
  }

  const header = document.createElement("header");
  header.className = "card-header";
  header.innerHTML = `
    <div class="card-header-top">
      <h3 class="card-title">${escapeHtml(test.title)}</h3>
      <div class="card-actions">
        <button class="card-download" type="button" disabled>Download PNG</button>
        <span class="card-status running">Running</span>
      </div>
    </div>
    <p class="card-expectation"><strong>Look for:</strong> ${escapeHtml(test.expectation)}</p>
  `;

  const views = document.createElement("div");
  views.className = "preview-grid";
  if (test.previewMin) {
    views.style.setProperty("--preview-min", `${test.previewMin}px`);
  } else if (test.featured) {
    views.style.setProperty("--preview-min", "210px");
  }

  const meta = document.createElement("ul");
  meta.className = "meta-list";

  const layerList = document.createElement("ul");
  layerList.className = "layer-list";

  const footer = document.createElement("p");
  footer.className = "card-footer";

  article.append(header, views, meta, layerList, footer);
  section.grid.append(article);

  const downloadButton = header.querySelector(".card-download");
  downloadButton.addEventListener("click", () => {
    if (card.downloadPayload !== null) {
      downloadCardImage(card.downloadPayload);
    }
  });

  const card = {
    article,
    downloadButton,
    downloadPayload: null,
    footer,
    header,
    layerList,
    meta,
    status: header.querySelector(".card-status"),
    test,
    views,
  };

  return card;
}

function renderCardResult(card, result, elapsedMs) {
  card.views.replaceChildren();
  card.meta.replaceChildren();
  card.layerList.replaceChildren();

  const existingNote = card.article.querySelector(".card-note");
  existingNote?.remove();

  for (const view of result.views ?? []) {
    appendView(card.views, view);
  }

  for (const [label, value] of result.metrics ?? []) {
    appendMeta(card.meta, label, value);
  }

  if ((result.layers ?? []).length > 0) {
    for (const layer of result.layers) {
      const item = document.createElement("li");
      item.innerHTML = `
        <span>
          <span class="layer-kind">${escapeHtml(layer.kind)}</span>
          <span class="layer-name">${escapeHtml(layer.name)}</span>
        </span>
        <span>${layer.x},${layer.y}${layer.parentId == null ? "" : ` · parent ${layer.parentId}`}</span>
      `;
      card.layerList.append(item);
    }
  }

  if (result.note) {
    const note = document.createElement("p");
    note.className = "card-note";
    note.innerHTML = `<strong>Note:</strong> ${escapeHtml(result.note)}`;
    card.article.insertBefore(note, card.footer);
  }

  card.footer.textContent = `${result.assertions} checks in ${Math.round(elapsedMs)} ms`;
  card.downloadPayload = {
    elapsedMs,
    expectation: card.test.expectation,
    result,
    title: card.test.title,
  };
  card.downloadButton.disabled = false;
}

function downloadCardImage(payload) {
  const canvas = renderCardExport(payload);
  canvas.toBlob((blob) => {
    if (blob === null) {
      recordDiagnostic("error", `[${payload.title}] card export failed`);
      return;
    }
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${slugify(payload.title)}.png`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }, "image/png");
}

function renderCardExport(payload) {
  const width = 1380;
  const padding = 44;
  const gap = 24;
  const contentWidth = width - padding * 2;
  const metrics = payload.result.metrics ?? [];
  const views = payload.result.views ?? [];
  const columns = chooseExportColumns(views);
  const cellWidth =
    columns === 1 ? contentWidth : Math.floor((contentWidth - gap * (columns - 1)) / columns);
  const measureCanvas = document.createElement("canvas");
  const measureContext = measureCanvas.getContext("2d");

  let height = padding;
  height += measureWrappedText(measureContext, payload.title, "700 48px Iowan Old Style", contentWidth, 54);
  height += 18;
  height += measureWrappedText(
    measureContext,
    `Look for: ${payload.expectation ?? ""}`.trim(),
    "400 28px Iowan Old Style",
    contentWidth,
    40,
  );

  if (payload.result.note) {
    height += 20;
    height += measureWrappedText(
      measureContext,
      `Note: ${payload.result.note}`,
      "400 25px Iowan Old Style",
      contentWidth,
      36,
    );
  }

  if (metrics.length > 0) {
    height += 28;
    height += measureMetricGrid(measureContext, metrics, contentWidth, gap);
  }

  if (views.length > 0) {
    height += 30;
    height += measureViewGrid(measureContext, views, columns, cellWidth, gap);
  }

  height += 26;
  height += 34;
  height += padding;

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = Math.ceil(height);
  const context = canvas.getContext("2d");

  context.fillStyle = "#fffaf0";
  context.fillRect(0, 0, width, canvas.height);
  context.fillStyle = "#1d1c1a";

  let y = padding;
  context.font = "700 48px Iowan Old Style";
  y = drawWrappedText(context, payload.title, padding, y, contentWidth, 54, "#1d1c1a");

  y += 10;
  context.font = "400 28px Iowan Old Style";
  y = drawWrappedText(
    context,
    `Look for: ${payload.expectation ?? ""}`.trim(),
    padding,
    y,
    contentWidth,
    40,
    "#5d574c",
  );

  if (payload.result.note) {
    y += 20;
    context.font = "400 25px Iowan Old Style";
    y = drawWrappedText(
      context,
      `Note: ${payload.result.note}`,
      padding,
      y,
      contentWidth,
      36,
      "#5d574c",
    );
  }

  if (metrics.length > 0) {
    y += 24;
    y = drawMetricGrid(context, metrics, padding, y, contentWidth, gap);
  }

  if (views.length > 0) {
    y += 26;
    y = drawViewGrid(context, views, padding, y, columns, cellWidth, gap);
  }

  y += 24;
  context.strokeStyle = "rgba(29, 28, 26, 0.12)";
  context.beginPath();
  context.moveTo(padding, y);
  context.lineTo(width - padding, y);
  context.stroke();

  y += 28;
  context.font = "600 22px Cascadia Code";
  context.fillStyle = "#5d574c";
  context.fillText(`${payload.result.assertions} checks in ${Math.round(payload.elapsedMs)} ms`, padding, y);

  return canvas;
}

function setCardStatus(card, status, text) {
  card.article.classList.remove("is-pass", "is-fail", "is-experimental");
  card.status.className = `card-status ${status}`;
  card.status.textContent = text;

  if (status === "pass") {
    card.article.classList.add("is-pass");
  } else if (status === "fail") {
    card.article.classList.add("is-fail");
  } else if (status === "experimental") {
    card.article.classList.add("is-experimental");
  }
}

function slugify(text) {
  return text
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, "-")
    .replaceAll(/^-+|-+$/g, "")
    .slice(0, 80);
}

function getSectionNode(sectionKey) {
  const existing = sectionNodes.get(sectionKey);
  if (existing) {
    return existing;
  }

  const section = SECTION_INFO[sectionKey];
  const wrapper = document.createElement("section");
  wrapper.className = "section-block";
  wrapper.innerHTML = `
    <div class="section-header">
      <div>
        <p class="eyebrow">${escapeHtml(section.chip)}</p>
        <h2>${escapeHtml(section.title)}</h2>
        <p>${escapeHtml(section.description)}</p>
      </div>
      <div class="section-chip">${escapeHtml(section.chip)}</div>
    </div>
  `;

  const grid = document.createElement("div");
  grid.className = "section-grid";
  wrapper.append(grid);
  dom.suite.append(wrapper);

  const entry = { grid, wrapper };
  sectionNodes.set(sectionKey, entry);
  return entry;
}

function updateCounters(counts) {
  dom.runtimeCount.textContent = String(counts.total);
  dom.runtimeExperimental.textContent = String(counts.experimental);
  dom.runtimeFail.textContent = String(counts.fail);
  dom.runtimePass.textContent = String(counts.pass);
  suiteState.cards = counts.total;
  suiteState.experimental = counts.experimental;
  suiteState.fail = counts.fail;
  suiteState.pass = counts.pass;
  syncSuiteState();
}

function resetSuiteUi() {
  sectionNodes.clear();
  dom.suite.replaceChildren();
  dom.runtimeCount.textContent = "0";
  dom.runtimeExperimental.textContent = "0";
  dom.runtimeFail.textContent = "0";
  dom.runtimePass.textContent = "0";
  suiteState.cards = 0;
  suiteState.experimental = 0;
  suiteState.fail = 0;
  suiteState.pass = 0;
  setRuntimeStatus("running", "Initializing");
  setSimdStatus("Checking");
  syncSuiteState();
}

function installDiagnostics() {
  const originalError = console.error.bind(console);
  const originalWarn = console.warn.bind(console);

  console.error = (...args) => {
    recordDiagnostic("error", stringifyArgs(args));
    originalError(...args);
  };
  console.warn = (...args) => {
    recordDiagnostic("warn", stringifyArgs(args));
    originalWarn(...args);
  };

  window.addEventListener("error", (event) => {
    recordDiagnostic("error", event.message || "Unknown window error");
  });

  window.addEventListener("unhandledrejection", (event) => {
    recordDiagnostic("error", toErrorMessage(event.reason));
  });
}

function recordDiagnostic(level, message) {
  diagnostics.push({ level, message });
  renderDiagnostics();
}

function renderDiagnostics() {
  dom.diagnosticCount.textContent = String(diagnostics.length);
  dom.diagnosticList.replaceChildren();
  suiteState.diagnostics = diagnostics.length;
  syncSuiteState();

  if (diagnostics.length === 0) {
    const item = document.createElement("li");
    item.className = "diagnostic-empty";
    item.textContent = "No errors captured.";
    dom.diagnosticList.append(item);
    return;
  }

  for (const diagnostic of diagnostics.slice(-20).reverse()) {
    const item = document.createElement("li");
    item.className = diagnostic.level;
    item.textContent = diagnostic.message;
    dom.diagnosticList.append(item);
  }
}

function setRuntimeStatus(status, text) {
  dom.runtimeStatus.textContent = text;
  suiteState.status = status;
  suiteState.statusText = text;
  syncSuiteState();
}

function setSimdStatus(text) {
  dom.runtimeSimd.textContent = text;
  suiteState.simd = text;
  syncSuiteState();
}

function syncSuiteState() {
  dom.body.dataset.suiteStatus = suiteState.status;
  dom.body.dataset.suiteCount = String(suiteState.cards);
  dom.body.dataset.suitePass = String(suiteState.pass);
  dom.body.dataset.suiteFail = String(suiteState.fail);
  dom.body.dataset.suiteExperimental = String(suiteState.experimental);
  dom.body.dataset.suiteDiagnostics = String(suiteState.diagnostics);
  dom.suiteMachineState.textContent = JSON.stringify(suiteState);
  window.__KIMG_DEMO__ = { ...suiteState };
}

function appendView(container, view) {
  const figure = document.createElement("figure");
  figure.className = "figure-box";
  if (view.kind === "code" || view.kind === "message" || view.wide === true) {
    figure.classList.add("is-wide");
  } else if (view.kind === "rgba" && view.width > view.height * 1.2) {
    figure.classList.add("is-wide");
  } else if (view.kind === "rgba" && view.width >= 160 && view.height >= 160) {
    figure.classList.add("is-medium");
  }

  let shell;
  if (view.kind === "swatches") {
    shell = document.createElement("div");
    shell.className = "swatch-shell";
    const grid = document.createElement("div");
    grid.className = "swatch-grid";
    for (const color of splitPalette(view.palette)) {
      const swatch = document.createElement("div");
      swatch.className = "swatch";
      swatch.style.background = `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${color[3] / 255})`;
      grid.append(swatch);
    }
    shell.append(grid);
  } else if (view.kind === "code") {
    shell = document.createElement("div");
    shell.className = "code-shell";
    const pre = document.createElement("pre");
    pre.textContent = view.text;
    shell.append(pre);
  } else if (view.kind === "message") {
    shell = document.createElement("div");
    shell.className = "message-shell";
    const paragraph = document.createElement("p");
    paragraph.textContent = view.text;
    shell.append(paragraph);
  } else {
    shell = document.createElement("div");
    shell.className = "canvas-shell";
    shell.append(
      canvasFromRgba(view.rgba, view.width, view.height, {
        maxDisplay: view.maxDisplay ?? chooseViewMaxDisplay(view),
      }),
    );
  }

  const caption = document.createElement("figcaption");
  caption.className = "figure-caption";
  caption.textContent = view.label;

  figure.append(shell, caption);
  container.append(figure);
}

function chooseExportColumns(views) {
  const denseCount = views.filter((view) => view.kind === "rgba" || view.kind === "swatches").length;
  if (denseCount >= 8) {
    return 3;
  }
  if (denseCount >= 2) {
    return 2;
  }
  return 1;
}

function appendMessageView(container, label, text) {
  appendView(container, messageView(label, text));
}

function appendMeta(container, label, value) {
  const item = document.createElement("li");
  item.innerHTML = `
    <span class="meta-label">${escapeHtml(String(label))}</span>
    <span class="meta-value">${escapeHtml(String(value))}</span>
  `;
  container.append(item);
}

function rgbaView(label, rgba, width, height, options = {}) {
  return {
    kind: "rgba",
    label,
    rgba,
    width,
    height,
    ...options,
  };
}

function swatchView(label, palette) {
  return {
    kind: "swatches",
    label,
    palette,
    wide: true,
  };
}

function codeView(label, text) {
  return {
    kind: "code",
    label,
    text,
    wide: true,
  };
}

function messageView(label, text) {
  return {
    kind: "message",
    label,
    text,
    wide: true,
  };
}

function canvasFromRgba(rgba, width, height, options = {}) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  context.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);

  const maxDisplay = options.maxDisplay ?? 220;
  const scale = Math.max(1, Math.floor(maxDisplay / Math.max(width, height)));
  canvas.style.width = `${width * scale}px`;
  canvas.style.height = `${height * scale}px`;
  return canvas;
}

function chooseViewMaxDisplay(view) {
  if (view.width > view.height * 1.25) {
    return 320;
  }
  if (view.width >= 160 && view.height >= 160) {
    return 280;
  }
  if (view.width <= 96 && view.height <= 96) {
    return 200;
  }
  return 220;
}

function measureWrappedText(context, text, font, maxWidth, lineHeight) {
  context.font = font;
  return wrapText(context, text, maxWidth).length * lineHeight;
}

function drawWrappedText(context, text, x, y, maxWidth, lineHeight, color) {
  const lines = wrapText(context, text, maxWidth);
  context.fillStyle = color;
  for (const line of lines) {
    context.fillText(line, x, y);
    y += lineHeight;
  }
  return y;
}

function wrapText(context, text, maxWidth) {
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length === 0) {
    return [""];
  }

  const lines = [];
  let line = words[0];
  for (let index = 1; index < words.length; index += 1) {
    const candidate = `${line} ${words[index]}`;
    if (context.measureText(candidate).width <= maxWidth) {
      line = candidate;
    } else {
      lines.push(line);
      line = words[index];
    }
  }
  lines.push(line);
  return lines;
}

function measureMetricGrid(context, metrics, contentWidth, gap) {
  const metricWidth = Math.floor((contentWidth - gap) / 2);
  context.font = "600 18px Cascadia Code";
  let height = 0;
  for (let index = 0; index < metrics.length; index += 2) {
    const left = metrics[index];
    const right = metrics[index + 1];
    const rowHeight = Math.max(
      measureMetricBlock(context, left, metricWidth),
      right ? measureMetricBlock(context, right, metricWidth) : 0,
    );
    height += rowHeight + gap;
  }
  return height - gap;
}

function measureMetricBlock(context, [label, value], width) {
  context.font = "600 18px Cascadia Code";
  const labelHeight = wrapText(context, String(label), width).length * 26;
  context.font = "700 22px Cascadia Code";
  const valueHeight = wrapText(context, String(value), width).length * 30;
  return Math.max(86, 20 + labelHeight + valueHeight);
}

function drawMetricGrid(context, metrics, x, y, contentWidth, gap) {
  const metricWidth = Math.floor((contentWidth - gap) / 2);
  for (let index = 0; index < metrics.length; index += 2) {
    const left = metrics[index];
    const right = metrics[index + 1];
    const leftHeight = drawMetricBlock(context, left, x, y, metricWidth);
    const rightHeight = right ? drawMetricBlock(context, right, x + metricWidth + gap, y, metricWidth) : 0;
    y += Math.max(leftHeight, rightHeight) + gap;
  }
  return y - gap;
}

function drawMetricBlock(context, [label, value], x, y, width) {
  const height = measureMetricBlock(context, [label, value], width);
  drawRoundedRect(context, x, y, width, height, 18, "#ffffff", "rgba(29, 28, 26, 0.12)");

  context.font = "600 18px Cascadia Code";
  y = drawWrappedText(context, String(label), x + 18, y + 28, width - 36, 26, "#5d574c");
  context.font = "700 22px Cascadia Code";
  drawWrappedText(context, String(value), x + 18, y + 8, width - 36, 30, "#1d1c1a");
  return height;
}

function measureViewGrid(context, views, columns, cellWidth, gap) {
  let height = 0;
  let rowHeight = 0;
  let column = 0;

  for (const view of views) {
    const spansFull = view.kind === "code" || view.kind === "message" || view.wide === true;
    const viewHeight = measureExportView(context, view, spansFull ? cellWidth * columns + gap * (columns - 1) : cellWidth);

    if (spansFull) {
      if (column !== 0) {
        height += rowHeight + gap;
        rowHeight = 0;
        column = 0;
      }
      height += viewHeight + gap;
      continue;
    }

    rowHeight = Math.max(rowHeight, viewHeight);
    column += 1;
    if (column >= columns) {
      height += rowHeight + gap;
      rowHeight = 0;
      column = 0;
    }
  }

  if (column !== 0) {
    height += rowHeight + gap;
  }

  return Math.max(0, height - gap);
}

function drawViewGrid(context, views, x, y, columns, cellWidth, gap) {
  let rowHeight = 0;
  let column = 0;

  for (const view of views) {
    const spansFull = view.kind === "code" || view.kind === "message" || view.wide === true;
    const drawWidth = spansFull ? cellWidth * columns + gap * (columns - 1) : cellWidth;
    const viewHeight = measureExportView(context, view, drawWidth);

    if (spansFull) {
      if (column !== 0) {
        y += rowHeight + gap;
        rowHeight = 0;
        column = 0;
      }
      drawExportView(context, view, x, y, drawWidth, viewHeight);
      y += viewHeight + gap;
      continue;
    }

    const drawX = x + column * (cellWidth + gap);
    drawExportView(context, view, drawX, y, drawWidth, viewHeight);
    rowHeight = Math.max(rowHeight, viewHeight);
    column += 1;
    if (column >= columns) {
      y += rowHeight + gap;
      rowHeight = 0;
      column = 0;
    }
  }

  if (column !== 0) {
    y += rowHeight + gap;
  }

  return y - gap;
}

function measureExportView(context, view, width) {
  if (view.kind === "rgba") {
    const imageHeight = Math.min(420, Math.max(140, Math.round((width - 28) * (view.height / view.width))));
    return imageHeight + 72;
  }
  if (view.kind === "swatches") {
    const colors = splitPalette(view.palette).length;
    const columns = Math.max(4, Math.min(8, Math.floor((width - 36) / 56)));
    const rows = Math.ceil(colors / columns);
    return 72 + rows * 56;
  }

  context.font = "500 20px Cascadia Code";
  const text = view.kind === "code" || view.kind === "message" ? view.text : "";
  const textHeight = wrapText(context, text, width - 36).length * 28;
  return 86 + textHeight;
}

function drawExportView(context, view, x, y, width, height) {
  drawRoundedRect(context, x, y, width, height, 22, "#ffffff", "rgba(29, 28, 26, 0.12)");

  context.font = "600 18px Cascadia Code";
  context.fillStyle = "#5d574c";
  context.fillText(view.label, x + 18, y + 30);

  if (view.kind === "rgba") {
    const boxX = x + 14;
    const boxY = y + 44;
    const boxWidth = width - 28;
    const boxHeight = height - 58;
    drawCheckerboard(context, boxX, boxY, boxWidth, boxHeight, 18);

    const temp = document.createElement("canvas");
    temp.width = view.width;
    temp.height = view.height;
    const tempContext = temp.getContext("2d");
    tempContext.putImageData(new ImageData(new Uint8ClampedArray(view.rgba), view.width, view.height), 0, 0);

    const scale = Math.min(boxWidth / view.width, boxHeight / view.height);
    const drawWidth = Math.max(1, Math.round(view.width * scale));
    const drawHeight = Math.max(1, Math.round(view.height * scale));
    const drawX = boxX + Math.floor((boxWidth - drawWidth) / 2);
    const drawY = boxY + Math.floor((boxHeight - drawHeight) / 2);
    context.imageSmoothingEnabled = false;
    context.drawImage(temp, drawX, drawY, drawWidth, drawHeight);
    context.imageSmoothingEnabled = true;
    return;
  }

  if (view.kind === "swatches") {
    const colors = splitPalette(view.palette);
    const columns = Math.max(4, Math.min(8, Math.floor((width - 36) / 56)));
    const size = Math.floor((width - 36 - (columns - 1) * 10) / columns);
    let colorX = x + 18;
    let colorY = y + 48;
    colors.forEach((color, index) => {
      context.fillStyle = `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${color[3] / 255})`;
      drawRoundedRect(context, colorX, colorY, size, size, 12, context.fillStyle, "rgba(29, 28, 26, 0.14)");
      colorX += size + 10;
      if ((index + 1) % columns === 0) {
        colorX = x + 18;
        colorY += size + 10;
      }
    });
    return;
  }

  context.font = "500 20px Cascadia Code";
  drawWrappedText(
    context,
    view.text,
    x + 18,
    y + 56,
    width - 36,
    28,
    "#1d1c1a",
  );
}

function drawCheckerboard(context, x, y, width, height, size) {
  context.fillStyle = "#ffffff";
  context.fillRect(x, y, width, height);
  for (let row = 0; row < Math.ceil(height / size); row += 1) {
    for (let column = 0; column < Math.ceil(width / size); column += 1) {
      if ((row + column) % 2 === 0) {
        context.fillStyle = "#ece6df";
        context.fillRect(x + column * size, y + row * size, size, size);
      }
    }
  }
  context.strokeStyle = "rgba(29, 28, 26, 0.12)";
  context.strokeRect(x, y, width, height);
}

function drawRoundedRect(context, x, y, width, height, radius, fill, stroke) {
  context.beginPath();
  context.moveTo(x + radius, y);
  context.lineTo(x + width - radius, y);
  context.quadraticCurveTo(x + width, y, x + width, y + radius);
  context.lineTo(x + width, y + height - radius);
  context.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  context.lineTo(x + radius, y + height);
  context.quadraticCurveTo(x, y + height, x, y + height - radius);
  context.lineTo(x, y + radius);
  context.quadraticCurveTo(x, y, x + radius, y);
  context.closePath();
  context.fillStyle = fill;
  context.fill();
  context.strokeStyle = stroke;
  context.stroke();
}

function createVerifier() {
  return {
    count: 0,
    equal(actual, expected, message) {
      this.count += 1;
      if (actual !== expected) {
        throw new Error(`${message}. Expected ${expected}, received ${actual}.`);
      }
    },
    ok(condition, message) {
      this.count += 1;
      if (!condition) {
        throw new Error(message);
      }
    },
  };
}

function createGlyphFixture() {
  const canvas = document.createElement("canvas");
  canvas.width = 96;
  canvas.height = 96;
  const context = canvas.getContext("2d");

  context.clearRect(0, 0, 96, 96);
  context.fillStyle = "#d15033";
  context.fillRect(8, 14, 28, 68);
  context.fillRect(8, 56, 52, 18);

  context.fillStyle = "#244fdd";
  context.beginPath();
  context.moveTo(50, 14);
  context.lineTo(86, 30);
  context.lineTo(50, 46);
  context.closePath();
  context.fill();

  context.fillStyle = "#f0bf3d";
  context.beginPath();
  context.arc(65, 67, 15, 0, Math.PI * 2);
  context.fill();

  context.strokeStyle = "#15120f";
  context.lineWidth = 4;
  context.beginPath();
  context.moveTo(18, 10);
  context.lineTo(82, 88);
  context.stroke();

  return fixtureFromCanvas(canvas);
}

function createBorderedFixture(innerFixture) {
  const canvas = document.createElement("canvas");
  canvas.width = innerFixture.width + 32;
  canvas.height = innerFixture.height + 32;
  const context = canvas.getContext("2d");
  context.putImageData(
    new ImageData(new Uint8ClampedArray(innerFixture.rgba), innerFixture.width, innerFixture.height),
    16,
    16,
  );
  return fixtureFromCanvas(canvas);
}

function createClipPatternFixture() {
  const canvas = document.createElement("canvas");
  canvas.width = 136;
  canvas.height = 136;
  const context = canvas.getContext("2d");
  context.clearRect(0, 0, canvas.width, canvas.height);

  context.fillStyle = "#c9492d";
  context.fillRect(14, 18, 34, 92);

  context.fillStyle = "#234fdd";
  context.beginPath();
  context.moveTo(70, 18);
  context.lineTo(120, 42);
  context.lineTo(70, 66);
  context.closePath();
  context.fill();

  context.fillStyle = "#f0bf3d";
  context.beginPath();
  context.arc(98, 96, 22, 0, Math.PI * 2);
  context.fill();

  context.strokeStyle = "#15120f";
  context.lineWidth = 7;
  context.beginPath();
  context.moveTo(28, 12);
  context.lineTo(116, 122);
  context.stroke();

  return fixtureFromCanvas(canvas);
}

function createFilterFixture() {
  const canvas = document.createElement("canvas");
  canvas.width = 108;
  canvas.height = 108;
  const context = canvas.getContext("2d");
  context.clearRect(0, 0, canvas.width, canvas.height);

  const ramp = context.createLinearGradient(0, 0, canvas.width, 0);
  ramp.addColorStop(0, "#0f1020");
  ramp.addColorStop(0.5, "#ffffff");
  ramp.addColorStop(1, "#f0bf3d");
  context.fillStyle = ramp;
  context.fillRect(0, 0, canvas.width, 22);

  context.fillStyle = "#c9492d";
  context.fillRect(10, 32, 26, 66);
  context.fillStyle = "#234fdd";
  context.fillRect(42, 32, 26, 66);
  context.fillStyle = "#157347";
  context.fillRect(74, 32, 24, 66);

  context.fillStyle = "#ffffff";
  context.fillRect(18, 42, 10, 46);
  context.fillRect(50, 42, 10, 46);
  context.fillRect(82, 42, 8, 46);

  context.fillStyle = "#f0bf3d";
  context.beginPath();
  context.arc(82, 76, 16, 0, Math.PI * 2);
  context.fill();

  context.strokeStyle = "#111111";
  context.lineWidth = 5;
  context.beginPath();
  context.moveTo(8, 98);
  context.lineTo(100, 30);
  context.stroke();

  return fixtureFromCanvas(canvas);
}

function createMaskFixture(width, height) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  context.fillStyle = "rgba(0,0,0,1)";
  context.fillRect(0, 0, width, height);
  context.fillStyle = "rgba(255,255,255,1)";
  context.fillRect(width * 0.08, height * 0.14, width * 0.28, height * 0.72);
  context.fillRect(width * 0.52, height * 0.58, width * 0.32, height * 0.2);
  context.beginPath();
  context.moveTo(width * 0.56, height * 0.16);
  context.lineTo(width * 0.88, height * 0.16);
  context.lineTo(width * 0.72, height * 0.46);
  context.closePath();
  context.fill();
  return fixtureFromCanvas(canvas);
}

function createFillFixture(mode) {
  const canvas = document.createElement("canvas");
  canvas.width = 120;
  canvas.height = 84;
  const context = canvas.getContext("2d");
  context.clearRect(0, 0, 120, 84);

  if (mode === "contiguous") {
    context.fillStyle = "rgba(255,255,255,1)";
    context.fillRect(0, 0, 120, 84);
    context.fillStyle = "rgba(45,45,45,1)";
    context.fillRect(8, 8, 104, 68);
    context.clearRect(18, 18, 42, 42);
    context.clearRect(68, 18, 22, 22);
  } else if (mode === "non-contiguous") {
    context.fillStyle = "rgba(255,255,255,1)";
    context.fillRect(0, 0, 120, 84);
    context.fillStyle = "rgba(80,80,80,1)";
    for (const [x, y] of [
      [12, 12],
      [68, 12],
      [28, 42],
      [84, 44],
    ]) {
      context.fillRect(x, y, 24, 20);
    }
  } else {
    for (let y = 0; y < 84; y += 1) {
      for (let x = 0; x < 120; x += 1) {
        const wobble = Math.sin((x + y) / 9) * 10;
        const value = Math.max(0, Math.min(255, 122 + wobble));
        context.fillStyle = `rgba(${value}, ${value}, ${value}, 1)`;
        context.fillRect(x, y, 1, 1);
      }
    }
  }

  return fixtureFromCanvas(canvas);
}

function createSpriteFixtures() {
  const palettes = [
    ["#c9492d", "#f2d27f", "#101010"],
    ["#234fdd", "#ffffff", "#0e1328"],
    ["#157347", "#c7f2d8", "#0f2518"],
    ["#8956c4", "#f3dbff", "#27123f"],
  ];
  return palettes.map((palette) => createSpriteFixture(palette));
}

function createSpriteFixture([primary, secondary, accent]) {
  const canvas = document.createElement("canvas");
  canvas.width = 8;
  canvas.height = 8;
  const context = canvas.getContext("2d");
  context.clearRect(0, 0, 8, 8);
  context.fillStyle = primary;
  context.fillRect(1, 1, 6, 6);
  context.fillStyle = secondary;
  context.fillRect(2, 2, 3, 3);
  context.fillStyle = accent;
  context.fillRect(5, 2, 1, 4);
  context.fillRect(2, 5, 4, 1);
  return fixtureFromCanvas(canvas);
}

function createUtilityTile() {
  const canvas = document.createElement("canvas");
  canvas.width = 2;
  canvas.height = 2;
  const context = canvas.getContext("2d");
  context.fillStyle = "#234fdd";
  context.fillRect(0, 0, 1, 1);
  context.fillStyle = "#c9492d";
  context.fillRect(1, 0, 1, 1);
  context.fillStyle = "#157347";
  context.fillRect(0, 1, 1, 1);
  context.fillStyle = "#f2d27f";
  context.fillRect(1, 1, 1, 1);
  return fixtureFromCanvas(canvas);
}

function tintFixture(fixture, [r, g, b]) {
  const output = new Uint8Array(fixture.rgba);
  for (let index = 0; index < output.length; index += 4) {
    const alpha = output[index + 3];
    if (alpha === 0) {
      continue;
    }
    output[index] = r;
    output[index + 1] = g;
    output[index + 2] = b;
  }
  return output;
}

function fixtureFromCanvas(canvas) {
  const context = canvas.getContext("2d");
  const imageData = context.getImageData(0, 0, canvas.width, canvas.height);
  return {
    height: canvas.height,
    rgba: new Uint8Array(imageData.data),
    width: canvas.width,
  };
}

async function loadBinaryFixture(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status}`);
  }

  return new Uint8Array(await response.arrayBuffer());
}

async function loadTeapotFixture(url, maxEdge) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status}`);
  }

  const sourceBytes = new Uint8Array(await response.arrayBuffer());
  const { width: originalWidth, height: originalHeight } = parsePngSize(sourceBytes);
  const scale = Math.min(1, maxEdge / Math.max(originalWidth, originalHeight));
  const width = Math.max(1, Math.round(originalWidth * scale));
  const height = Math.max(1, Math.round(originalHeight * scale));
  const decoded = await decodeImage(sourceBytes);

  const sourceComposition = await Composition.create({ width: originalWidth, height: originalHeight });

  try {
    const layerId = sourceComposition.addImageLayer({
      height: originalHeight,
      name: "teapot-source",
      rgba: decoded,
      width: originalWidth,
    });

    if (width !== originalWidth || height !== originalHeight) {
      sourceComposition.resizeLayerBilinear(layerId, { width, height });
    }

    const rgba = new Uint8Array(sourceComposition.getLayerRgba(layerId));
    const workingComposition = await Composition.create({ width, height });

    try {
      workingComposition.addImageLayer({
        height,
        name: "teapot-working",
        rgba,
        width,
      });

      return {
        originalHeight,
        originalWidth,
        pngBytes: workingComposition.exportPng(),
        rgba,
        sourceBytes,
        height,
        width,
      };
    } finally {
      workingComposition.free();
    }
  } finally {
    sourceComposition.free();
  }
}

function parsePngSize(bytes) {
  if (bytes.length < 24) {
    throw new Error("PNG fixture is too short to contain an IHDR chunk");
  }

  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  for (let index = 0; index < signature.length; index += 1) {
    if (bytes[index] !== signature[index]) {
      throw new Error("Teapot fixture is not a valid PNG file");
    }
  }

  const chunkType = String.fromCharCode(bytes[12], bytes[13], bytes[14], bytes[15]);
  if (chunkType !== "IHDR") {
    throw new Error("PNG fixture is missing the IHDR chunk");
  }

  const dataView = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return {
    height: dataView.getUint32(20),
    width: dataView.getUint32(16),
  };
}

function splitPalette(palette) {
  const colors = [];
  for (let index = 0; index + 3 < palette.length; index += 4) {
    colors.push(palette.slice(index, index + 4));
  }
  return colors;
}

function rgbaEquals(left, right) {
  if (left.length !== right.length) {
    return false;
  }
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
}

function decodeBase64(input) {
  const binary = atob(input);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function stringifyArgs(args) {
  return args
    .map((value) => {
      if (value instanceof Error) {
        return value.stack ?? value.message;
      }
      if (typeof value === "string") {
        return value;
      }
      try {
        return JSON.stringify(value);
      } catch {
        return String(value);
      }
    })
    .join(" ");
}

function toErrorMessage(error) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function escapeHtml(input) {
  return input
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

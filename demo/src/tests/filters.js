import { Composition } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView } from "../helpers/views.js";
import { rgbaEquals } from "../helpers/context.js";

async function buildScopedFilterScene(context, withFilter) {
  const composition = await Composition.create({ width: 292, height: 168 });
  composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
  composition.addShapeLayer({
    fill: [18, 18, 18, 18], height: 126, name: "control-frame",
    stroke: { color: [18, 18, 18, 72], width: 2 }, type: "rectangle", width: 118, x: 14, y: 20,
  });
  composition.addImageLayer({ height: context.filterFixture.height, name: "outside-control", rgba: context.filterFixture.rgba, width: context.filterFixture.width, x: 20, y: 26 });
  const groupId = composition.addGroupLayer({ name: "subject-group" });
  composition.addShapeLayer({
    fill: [35, 79, 221, 18], height: 126, name: "subject-frame",
    stroke: { color: [35, 79, 221, 84], width: 2 }, parentId: groupId, type: "rectangle", width: 118, x: 158, y: 20,
  });
  composition.addImageLayer({ height: context.filterFixture.height, name: "inside-subject", parentId: groupId, rgba: context.filterFixture.rgba, width: context.filterFixture.width, x: 164, y: 26 });
  const filterId = composition.addFilterLayer({ name: "group-filter", parentId: groupId });
  if (withFilter) {
    composition.setFilterLayerConfig(filterId, { brightness: 0.18, contrast: 0.46, hueDeg: 118, saturation: 0.28, temperature: -0.22, tint: 0.18 });
  }
  return {
    dispose() { composition.free(); },
    filter: composition.getLayer(filterId), height: composition.height, rgba: composition.renderRgba(), width: composition.width,
  };
}

async function mutateTeapotLayer(context, mutation, label) {
  const composition = await Composition.create({ width: context.fixture.width, height: context.fixture.height });
  const layerId = composition.addImageLayer({ height: context.fixture.height, name: label, rgba: context.fixture.rgba, width: context.fixture.width });
  mutation(composition, layerId);
  const result = { height: context.fixture.height, label, rgba: composition.getLayerRgba(layerId), width: context.fixture.width };
  composition.free();
  return result;
}

async function buildFilterVariants(context) {
  const operations = [
    ["Original", (c, id) => c.updateLayer(id, {})],
    ["boxBlur", (c, id) => c.boxBlurLayer(id, { radius: 4 })],
    ["gaussianBlur", (c, id) => c.gaussianBlurLayer(id, { radius: 4 })],
    ["sharpen", (c, id) => c.sharpenLayer(id)],
    ["edgeDetect", (c, id) => c.edgeDetectLayer(id)],
    ["emboss", (c, id) => c.embossLayer(id)],
    ["invert", (c, id) => c.invertLayer(id)],
    ["posterize", (c, id) => c.posterizeLayer(id, { levels: 3 })],
    ["threshold", (c, id) => c.thresholdLayer(id, { threshold: 150 })],
    ["levels", (c, id) => c.levelsLayer(id, { highlights: 0.88, midtones: 0.64, shadows: 0.18 })],
    ["gradientMap", (c, id) => c.gradientMapLayer(id, { stops: [{ color: [14, 25, 73, 255], position: 0 }, { color: [255, 224, 132, 255], position: 1 }] })],
  ];
  const variants = [];
  for (const [label, op] of operations) {
    variants.push(await mutateTeapotLayer(context, op, label));
  }
  return variants;
}

export const filterTests = [
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
          metrics: [["filter hue", filtered.filter?.filterConfig?.hueDeg?.toFixed(1) ?? "n/a"], ["filter contrast", filtered.filter?.filterConfig?.contrast ?? "n/a"]],
          views: [rgbaView("Without scoped filter", base.rgba, base.width, base.height), rgbaView("With scoped filter", filtered.rgba, filtered.width, filtered.height)],
        };
      } finally {
        base.dispose();
        filtered.dispose();
      }
    },
  },
  {
    expectation:
      "Each destructive filter should visibly alter the same hard-edged color chart in a distinct way.",
    featured: true,
    fullSpan: true,
    previewMin: 250,
    section: "filters",
    title: "Destructive Filter Strip",
    async run(context) {
      const verify = createVerifier();
      const variants = await buildFilterVariants(context);

      verify.ok(variants.length >= 10, "expected all destructive filter variants");
      verify.ok(variants.every((v) => v.rgba.length > 0), "each variant should render bytes");

      return {
        assertions: verify.count,
        metrics: [["Variants", variants.length], ["Source", `${context.fixture.width}x${context.fixture.height}`]],
        views: variants.map((v) => rgbaView(v.label, v.rgba, v.width, v.height, { maxDisplay: 240 })),
      };
    },
  },

  // ── New filter tests ────────────────────────────────────────────────────────

  {
    expectation:
      "Three levels presets — punch, fade, normalize — should each produce visibly different tonal results on the same teapot source.",
    section: "filters",
    title: "levelsLayer Presets",
    async run(context) {
      const verify = createVerifier();
      const presets = [
        { label: "Punch", params: { shadows: 0.1, midtones: 0.58, highlights: 0.92 } },
        { label: "Fade", params: { shadows: 0.28, midtones: 1.1, highlights: 0.76 } },
        { label: "Normalize", params: { shadows: 0, midtones: 1, highlights: 1 } },
      ];

      const results = [];
      for (const preset of presets) {
        const comp = await Composition.create({ width: context.fixture.width, height: context.fixture.height });
        const id = comp.addImageLayer({ height: context.fixture.height, name: preset.label, rgba: context.fixture.rgba, width: context.fixture.width });
        comp.levelsLayer(id, preset.params);
        results.push({ label: preset.label, rgba: comp.getLayerRgba(id), width: context.fixture.width, height: context.fixture.height });
        comp.free();
      }

      verify.equal(results.length, 3, "all three presets should produce output");
      verify.ok(!rgbaEquals(results[0].rgba, results[1].rgba), "punch and fade should differ");
      verify.ok(!rgbaEquals(results[1].rgba, results[2].rgba), "fade and normalize should differ");

      return {
        assertions: verify.count,
        metrics: [
          ["Punch", "shadows 0.10, mid 0.58, highlights 0.92"],
          ["Fade", "shadows 0.28, mid 1.10, highlights 0.76"],
          ["Normalize", "shadows 0, mid 1.0, highlights 1.0"],
        ],
        views: [
          rgbaView("Original", context.fixture.rgba, context.fixture.width, context.fixture.height, { maxDisplay: 240 }),
          ...results.map((r) => rgbaView(r.label, r.rgba, r.width, r.height, { maxDisplay: 240 })),
        ],
      };
    },
  },
  {
    expectation:
      "A 3-stop gradient map (black→cyan→white) should remap luminance so dark areas go black, mids go cyan, and highlights go white.",
    section: "filters",
    title: "gradientMapLayer 3-Stop",
    async run(context) {
      const verify = createVerifier();
      const comp = await Composition.create({ width: context.fixture.width, height: context.fixture.height });

      try {
        const id = comp.addImageLayer({ height: context.fixture.height, name: "teapot", rgba: context.fixture.rgba, width: context.fixture.width });
        const original = comp.getLayerRgba(id);
        comp.gradientMapLayer(id, {
          stops: [
            { color: [0, 0, 0, 255], position: 0 },
            { color: [0, 224, 212, 255], position: 0.5 },
            { color: [255, 255, 255, 255], position: 1 },
          ],
        });
        const mapped = comp.getLayerRgba(id);

        verify.ok(!rgbaEquals(original, mapped), "gradient map should alter pixel values");
        verify.ok(mapped.some((v, i) => i % 4 === 1 && v > 150), "cyan channel should be prominent in midtones");

        return {
          assertions: verify.count,
          metrics: [["Stop 0", "black (0,0,0)"], ["Stop 0.5", "cyan (0,224,212)"], ["Stop 1", "white (255,255,255)"]],
          views: [
            rgbaView("Original", original, context.fixture.width, context.fixture.height, { maxDisplay: 300 }),
            rgbaView("Gradient map", mapped, context.fixture.width, context.fixture.height, { maxDisplay: 300 }),
          ],
        };
      } finally {
        comp.free();
      }
    },
  },
  {
    expectation:
      "All 11 blend mode names (multiply, screen, overlay, darken, lighten, difference, exclusion, color-dodge, color-burn, soft-light, hard-light) should render distinctly on the same base + overlay pair.",
    featured: true,
    fullSpan: true,
    section: "filters",
    title: "Blend Modes Grid",
    async run() {
      const verify = createVerifier();
      const modes = ["normal", "multiply", "screen", "overlay", "darken", "lighten", "difference", "exclusion", "colorDodge", "colorBurn", "softLight", "hardLight"];
      const results = [];

      for (const mode of modes) {
        const comp = await Composition.create({ width: 96, height: 96 });
        comp.addSolidColorLayer({ color: [35, 79, 221, 255], name: "base" });
        const id = comp.addShapeLayer({ fill: [201, 73, 45, 200], height: 96, name: mode, type: "ellipse", width: 96, x: 0, y: 0 });
        comp.setLayerBlendMode(id, mode);
        results.push({ label: mode, rgba: comp.renderRgba(), width: 96, height: 96 });
        comp.free();
      }

      verify.equal(results.length, modes.length, "all blend modes should produce output");

      return {
        assertions: verify.count,
        metrics: [["Modes tested", modes.length]],
        views: results.map((r) => rgbaView(r.label, r.rgba, r.width, r.height, { maxDisplay: 120 })),
      };
    },
  },
];

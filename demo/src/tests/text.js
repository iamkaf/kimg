import { Composition, registerFont, clearRegisteredFonts, loadGoogleFont } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { rgbaView } from "../helpers/views.js";

export const textTests = [
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
          (await registerFont({ bytes: context.bungeeKimgWoff2, family: "Bungee", style: "normal", weight: 400 })) +
          (await registerFont({ bytes: context.bodoniModaRegularWoff2, family: "Bodoni Moda", style: "normal", weight: 400 })) +
          (await registerFont({ bytes: context.bodoniModaItalicWoff2, family: "Bodoni Moda", style: "italic", weight: 400 }));

        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        composition.addShapeLayer({ fill: [228, 113, 76, 28], height: 112, name: "backplate", stroke: { color: [228, 113, 76, 96], width: 3 }, type: "rectangle", width: 128, x: 14, y: 18 });

        const headlineId = composition.addTextLayer({ color: [201, 73, 45, 255], fontFamily: "Bungee", fontSize: 24, letterSpacing: 2, lineHeight: 28, name: "headline", text: "HELLO", x: 28, y: 32 });
        const badgeId = composition.addTextLayer({ color: [24, 77, 163, 255], fontFamily: "Bungee", fontSize: 16, letterSpacing: 1, lineHeight: 20, name: "badge", text: "KIMG\nTEXT", x: 236, y: 98 });

        composition.updateLayer(badgeId, { anchor: "center", rotation: -12, textConfig: { color: [35, 79, 221, 255], fontFamily: "Bungee", fontSize: 24, letterSpacing: 2, lineHeight: 28, text: "KIMG\nTEXT" } });

        const layers = composition.listLayers();
        const headline = composition.getLayer(headlineId);
        const badge = composition.getLayer(badgeId);
        const render = composition.renderRgba();

        styleComposition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        const cupcakeText = "Cupcake ipsum\ndolor sit amet\nfrosting";
        const panelSpecs = [
          { align: "left", color: [201, 73, 45, 255], fontStyle: "normal", fontWeight: 400, label: "left / 400", x: 16 },
          { align: "center", color: [35, 79, 221, 255], fontStyle: "normal", fontWeight: 900, label: "center / 900", x: 116 },
          { align: "right", color: [24, 119, 92, 255], fontStyle: "italic", fontWeight: 400, label: "right / italic", x: 216 },
        ];

        const styleLayerIds = [];
        for (const spec of panelSpecs) {
          styleComposition.addShapeLayer({ fill: [0, 0, 0, 0], height: 136, name: `${spec.label}-panel`, stroke: { color: [120, 112, 101, 90], width: 2 }, type: "rectangle", width: 88, x: spec.x, y: 16 });
          styleLayerIds.push(styleComposition.addTextLayer({ align: spec.align, boxWidth: 72, color: spec.color, fontFamily: "Bodoni Moda", fontSize: 17, fontStyle: spec.fontStyle, fontWeight: spec.fontWeight, lineHeight: 23, name: spec.label, text: cupcakeText, wrap: "word", x: spec.x + 8, y: 28 }));
        }

        const styleLayers = styleComposition.listLayers();
        const styleRender = styleComposition.renderRgba();
        const [leftText, centerText, rightText] = styleLayerIds.map((id) => styleComposition.getLayer(id));

        verify.equal(layers.filter((l) => l.kind === "text").length, 2, "expected two text layers in metadata");
        verify.ok(loadedFaces > 0, "registerFont should load at least one usable face");
        verify.equal(headline?.text, "HELLO", "headline text should stay readable in metadata");
        verify.equal(badge?.anchor, "center", "rotated text should switch to center anchor");
        verify.equal(badge?.letterSpacing, 2, "textConfig update should widen tracking");
        verify.equal(styleLayers.filter((l) => l.kind === "text").length, 3, "expected three styled text layers in the second composition");
        verify.equal(leftText?.align, "left", "left panel should keep left alignment");
        verify.equal(centerText?.fontWeight, 900, "center panel should use heavier weight");
        verify.equal(rightText?.fontStyle, "italic", "right panel should keep italic style");
        verify.ok(render.some((v) => v !== 0), "text composition should render non-empty pixels");
        verify.ok(styleRender.some((v) => v !== 0), "styled text composition should render non-empty pixels");

        return {
          assertions: verify.count,
          metrics: [
            ["Text layers", `${layers.filter((l) => l.kind === "text").length + styleLayers.filter((l) => l.kind === "text").length}`],
            ["Registered faces", loadedFaces],
            ["display families", "Bungee + Bodoni Moda"],
            ["headline size", headline?.fontSize ?? "n/a"],
            ["badge tracking", badge?.letterSpacing ?? "n/a"],
            ["badge rotation", badge?.rotation?.toFixed(1) ?? "n/a"],
            ["styled columns", `${leftText?.align} / ${centerText?.fontWeight} / ${rightText?.fontStyle}`],
          ],
          views: [
            rgbaView("Transform and tracking", render, composition.width, composition.height),
            rgbaView("Weight, italics, and alignment", styleRender, styleComposition.width, styleComposition.height),
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
        "/* latin */", "@font-face {", "  font-family: 'Bodoni Moda';", "  font-style: normal;", "  font-weight: 400;", "  font-display: swap;",
        `  src: url(${regularUrl}) format('woff2');`, "}",
        "/* latin */", "@font-face {", "  font-family: 'Bodoni Moda';", "  font-style: italic;", "  font-weight: 400;", "  font-display: swap;",
        `  src: url(${italicUrl}) format('woff2');`, "}",
      ].join("\n");

      try {
        await clearRegisteredFonts();
        globalThis.fetch = async (input) => {
          const url = typeof input === "string" ? input : input instanceof URL ? input.href : String(input);
          if (url.startsWith("https://fonts.googleapis.com/css2?")) return new Response(css, { headers: { "content-type": "text/css; charset=utf-8" }, status: 200 });
          if (url === regularUrl) return new Response(context.bodoniModaRegularWoff2, { headers: { "content-type": "font/woff2" }, status: 200 });
          if (url === italicUrl) return new Response(context.bodoniModaItalicWoff2, { headers: { "content-type": "font/woff2" }, status: 200 });
          return originalFetch(input);
        };

        const loaded = await loadGoogleFont({ family: "Bodoni Moda", ital: [0, 1], text: "Cupcakeipsumdolorsitamet", weights: [400] });
        composition.addSolidColorLayer({ color: [247, 241, 232, 255], name: "paper" });
        composition.addShapeLayer({ fill: [0, 0, 0, 0], height: 92, name: "regular-panel", stroke: { color: [120, 112, 101, 90], width: 2 }, type: "rectangle", width: 132, x: 16, y: 16 });
        composition.addShapeLayer({ fill: [0, 0, 0, 0], height: 92, name: "italic-panel", stroke: { color: [120, 112, 101, 90], width: 2 }, type: "rectangle", width: 132, x: 172, y: 16 });

        const regularId = composition.addTextLayer({ align: "left", boxWidth: 108, color: [201, 73, 45, 255], fontFamily: "Bodoni Moda", fontSize: 17, fontWeight: 400, lineHeight: 23, name: "google-regular", text: "Cupcake ipsum\ndolor sit amet", wrap: "word", x: 28, y: 28 });
        const italicId = composition.addTextLayer({ align: "left", boxWidth: 108, color: [35, 79, 221, 255], fontFamily: "Bodoni Moda", fontSize: 17, fontStyle: "italic", fontWeight: 400, lineHeight: 23, name: "google-italic", text: "Cupcake ipsum\ndolor sit amet", wrap: "word", x: 184, y: 28 });

        const render = composition.renderRgba();
        const regular = composition.getLayer(regularId);
        const italic = composition.getLayer(italicId);

        verify.equal(loaded.family, "Bodoni Moda", "loader should report the requested family");
        verify.equal(loaded.faces.length, 2, "mocked stylesheet should expose two faces");
        verify.ok(loaded.registeredFaces >= 2, "loadGoogleFont should register both faces");
        verify.equal(regular?.fontWeight, 400, "regular text should keep weight 400");
        verify.equal(italic?.fontStyle, "italic", "italic text should keep italic style");
        verify.ok(render.some((v) => v !== 0), "Google Fonts composition should render non-empty pixels");

        return {
          assertions: verify.count,
          metrics: [["Loader faces", loaded.faces.length], ["Registered faces", loaded.registeredFaces], ["Regular weight", regular?.fontWeight ?? "n/a"], ["Italic style", italic?.fontStyle ?? "n/a"]],
          views: [rgbaView("Mocked CSS2 font load", render, composition.width, composition.height)],
        };
      } finally {
        globalThis.fetch = originalFetch;
        await clearRegisteredFonts();
        composition.free();
      }
    },
  },
];

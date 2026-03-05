import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { Composition, registerFont } from "../dist/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "..");
const demoAssetsDir = path.join(rootDir, "demo", "assets");
const outputPath = path.join(demoAssetsDir, "kimg-hero.png");

const WIDTH = 1400;
const HEIGHT = 720;

function parsePngSize(bytes) {
  if (bytes.length < 24) {
    throw new Error("PNG is too short to contain an IHDR chunk.");
  }

  const signature = [137, 80, 78, 71, 13, 10, 26, 10];
  for (let index = 0; index < signature.length; index += 1) {
    if (bytes[index] !== signature[index]) {
      throw new Error("Expected a valid PNG file for the hero teapot.");
    }
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return {
    height: view.getUint32(20),
    width: view.getUint32(16),
  };
}

async function registerDemoFonts() {
  const bungee = await readFile(path.join(demoAssetsDir, "bungee-kimg.woff2"));

  await registerFont({ bytes: bungee, family: "Bungee", style: "normal", weight: 400 });

  const candidateSystemFonts = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
  ];
  for (const fontPath of candidateSystemFonts) {
    try {
      const bytes = await readFile(fontPath);
      await registerFont({ bytes, family: "Hero Sans", style: "normal", weight: 400 });
      return "Hero Sans";
    } catch {
      // try next candidate
    }
  }
  return "Bungee";
}

async function buildHero() {
  const uiFontFamily = await registerDemoFonts();

  const doc = await Composition.create({ width: WIDTH, height: HEIGHT });
  try {
    doc.addGradientLayer({
      direction: "diagonalDown",
      name: "bg-main",
      stops: [
        { color: [10, 14, 28, 255], position: 0 },
        { color: [22, 30, 64, 255], position: 0.52 },
        { color: [23, 70, 88, 255], position: 1 },
      ],
    });

    const glowTop = doc.addGradientLayer({
      direction: "vertical",
      name: "bg-glow-top",
      stops: [
        { color: [80, 149, 255, 110], position: 0 },
        { color: [80, 149, 255, 0], position: 0.55 },
        { color: [0, 0, 0, 0], position: 1 },
      ],
    });
    doc.updateLayer(glowTop, { blendMode: "screen", opacity: 0.9 });

    const glowRight = doc.addGradientLayer({
      direction: "horizontal",
      name: "bg-glow-right",
      stops: [
        { color: [0, 0, 0, 0], position: 0 },
        { color: [93, 240, 212, 0], position: 0.68 },
        { color: [93, 240, 212, 110], position: 1 },
      ],
    });
    doc.updateLayer(glowRight, { blendMode: "screen", opacity: 0.85 });

    doc.addTextLayer({
      align: "left",
      color: [157, 195, 252, 96],
      fontFamily: "Bungee",
      fontSize: 156,
      fontWeight: 700,
      letterSpacing: 0,
      lineHeight: 160,
      name: "title-shadow",
      text: "KIMG",
      x: 560,
      y: 214,
    });

    doc.addTextLayer({
      align: "left",
      color: [238, 247, 255, 255],
      fontFamily: "Bungee",
      fontSize: 156,
      fontWeight: 700,
      letterSpacing: 0,
      lineHeight: 160,
      name: "title",
      text: "KIMG",
      x: 550,
      y: 202,
    });

    doc.addShapeLayer({
      fill: [132, 188, 255, 42],
      height: 8,
      name: "separator",
      radius: 4,
      type: "rectangle",
      width: 520,
      x: 550,
      y: 358,
    });

    doc.addTextLayer({
      align: "left",
      color: [187, 211, 235, 255],
      fontFamily: uiFontFamily,
      fontSize: 40,
      fontWeight: 500,
      lineHeight: 44,
      name: "subtitle",
      text: "LAYERED COMPOSITING ENGINE",
      x: 550,
      y: 386,
    });

    doc.addTextLayer({
      align: "left",
      color: [151, 182, 214, 255],
      fontFamily: uiFontFamily,
      fontSize: 30,
      fontWeight: 500,
      lineHeight: 34,
      name: "support",
      text: "RUST CORE  ·  WASM  ·  NODE + BROWSER",
      x: 550,
      y: 452,
    });

    const teapotBytes = await readFile(path.join(demoAssetsDir, "teapot.png"));
    const { width: teapotWidth } = parsePngSize(teapotBytes);
    const teapotScale = 0.5;
    const teapotShiftX = Math.round(teapotWidth * teapotScale * 0.15);
    const teapotCenterX = 206 + teapotShiftX;
    const teapotCenterY = Math.round(HEIGHT / 2);
    const teapot = doc.addPngLayer({
      name: "teapot",
      png: teapotBytes,
      x: teapotCenterX,
      y: teapotCenterY,
    });
    doc.updateLayer(teapot, {
      anchor: "center",
      rotation: 0,
      scaleX: teapotScale,
      scaleY: teapotScale,
    });

    const png = doc.exportPng();
    await mkdir(demoAssetsDir, { recursive: true });
    await writeFile(outputPath, png);
    console.log(`Hero image written to ${outputPath}`);
  } finally {
    doc.free();
  }
}

buildHero().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

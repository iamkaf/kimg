import { Composition, decodeImage, simdSupported } from "#kimg/index.js";
import { VOLUME_RASTER_ASSETS, VOLUME_SVG_ASSETS } from "../constants.js";

export function resolveDemoPreloadInput() {
  const mode = new URLSearchParams(window.location.search).get("wasm");
  if (mode === "baseline") {
    return { module_or_path: new URL("../../dist/kimg_wasm_bg.wasm", import.meta.url) };
  }
  if (mode === "simd") {
    return { module_or_path: new URL("../../dist/kimg_wasm_simd_bg.wasm", import.meta.url) };
  }
  return undefined;
}

export async function buildContext() {
  const fixture = await loadTeapotFixture("./assets/teapot.png", 192);
  const [
    bodoniModaItalicWoff2,
    bodoniModaRegularWoff2,
    bungeeKimgWoff2,
    loadedRasterFixtures,
    loadedSvgFixtures,
  ] = await Promise.all([
    loadBinaryFixture("./assets/bodoni-moda-italic.woff2"),
    loadBinaryFixture("./assets/bodoni-moda-regular.woff2"),
    loadBinaryFixture("./assets/bungee-kimg.woff2"),
    loadVolumeRasterFixtures(),
    loadVolumeSvgFixtures(),
  ]);

  const volumeFixtures = {
    teapot: {
      key: "teapot",
      label: "Teapot PNG",
      path: "./assets/teapot.png",
      sourceBytes: fixture.sourceBytes,
      originalWidth: fixture.originalWidth,
      originalHeight: fixture.originalHeight,
      width: fixture.width,
      height: fixture.height,
      rgba: fixture.rgba,
    },
    ...loadedRasterFixtures,
  };
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
    runtime: { simd: simdSupported() },
    spriteFixtures: createSpriteFixtures(),
    utilityPalette: [
      await import("#kimg/index.js").then(m => m.hexToRgb("#234fdd")),
      await import("#kimg/index.js").then(m => m.hexToRgb("#c9492d")),
      await import("#kimg/index.js").then(m => m.hexToRgb("#157347")),
    ].flatMap((rgb) => [rgb[0], rgb[1], rgb[2], 255]),
    utilityTile,
    volumeFixtures,
    volumeSvgFixtures: loadedSvgFixtures,
  };
}

export function rgbaEquals(left, right) {
  if (left.length !== right.length) return false;
  for (let i = 0; i < left.length; i++) {
    if (left[i] !== right[i]) return false;
  }
  return true;
}

export function decodeBase64(input) {
  const binary = atob(input);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

export function tintFixture(fixture, [r, g, b]) {
  const output = new Uint8Array(fixture.rgba);
  for (let i = 0; i < output.length; i += 4) {
    if (output[i + 3] === 0) continue;
    output[i] = r; output[i + 1] = g; output[i + 2] = b;
  }
  return output;
}

export function stringifyArgs(args) {
  return args.map((v) => {
    if (v instanceof Error) return v.stack ?? v.message;
    if (typeof v === "string") return v;
    try { return JSON.stringify(v); } catch { return String(v); }
  }).join(" ");
}

export function toErrorMessage(error) {
  if (error instanceof Error) return error.message;
  return String(error);
}

// ── Fixture builders ─────────────────────────────────────────────────────────

export function createGlyphFixture() {
  const canvas = document.createElement("canvas");
  canvas.width = 96; canvas.height = 96;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, 96, 96);
  ctx.fillStyle = "#d15033";
  ctx.fillRect(8, 14, 28, 68);
  ctx.fillRect(8, 56, 52, 18);
  ctx.fillStyle = "#244fdd";
  ctx.beginPath(); ctx.moveTo(50, 14); ctx.lineTo(86, 30); ctx.lineTo(50, 46); ctx.closePath(); ctx.fill();
  ctx.fillStyle = "#f0bf3d";
  ctx.beginPath(); ctx.arc(65, 67, 15, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = "#15120f"; ctx.lineWidth = 4;
  ctx.beginPath(); ctx.moveTo(18, 10); ctx.lineTo(82, 88); ctx.stroke();
  return fixtureFromCanvas(canvas);
}

export function createBorderedFixture(innerFixture) {
  const canvas = document.createElement("canvas");
  canvas.width = innerFixture.width + 32;
  canvas.height = innerFixture.height + 32;
  const ctx = canvas.getContext("2d");
  ctx.putImageData(new ImageData(new Uint8ClampedArray(innerFixture.rgba), innerFixture.width, innerFixture.height), 16, 16);
  return fixtureFromCanvas(canvas);
}

export function createClipPatternFixture() {
  const canvas = document.createElement("canvas");
  canvas.width = 136; canvas.height = 136;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = "#c9492d"; ctx.fillRect(14, 18, 34, 92);
  ctx.fillStyle = "#234fdd";
  ctx.beginPath(); ctx.moveTo(70, 18); ctx.lineTo(120, 42); ctx.lineTo(70, 66); ctx.closePath(); ctx.fill();
  ctx.fillStyle = "#f0bf3d";
  ctx.beginPath(); ctx.arc(98, 96, 22, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = "#15120f"; ctx.lineWidth = 7;
  ctx.beginPath(); ctx.moveTo(28, 12); ctx.lineTo(116, 122); ctx.stroke();
  return fixtureFromCanvas(canvas);
}

export function createFilterFixture() {
  const canvas = document.createElement("canvas");
  canvas.width = 108; canvas.height = 108;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const ramp = ctx.createLinearGradient(0, 0, canvas.width, 0);
  ramp.addColorStop(0, "#0f1020");
  ramp.addColorStop(0.5, "#ffffff");
  ramp.addColorStop(1, "#f0bf3d");
  ctx.fillStyle = ramp; ctx.fillRect(0, 0, canvas.width, 22);
  ctx.fillStyle = "#c9492d"; ctx.fillRect(10, 32, 26, 66);
  ctx.fillStyle = "#234fdd"; ctx.fillRect(42, 32, 26, 66);
  ctx.fillStyle = "#157347"; ctx.fillRect(74, 32, 24, 66);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(18, 42, 10, 46); ctx.fillRect(50, 42, 10, 46); ctx.fillRect(82, 42, 8, 46);
  ctx.fillStyle = "#f0bf3d";
  ctx.beginPath(); ctx.arc(82, 76, 16, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = "#111111"; ctx.lineWidth = 5;
  ctx.beginPath(); ctx.moveTo(8, 98); ctx.lineTo(100, 30); ctx.stroke();
  return fixtureFromCanvas(canvas);
}

export function createMaskFixture(width, height) {
  const canvas = document.createElement("canvas");
  canvas.width = width; canvas.height = height;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "rgba(0,0,0,1)"; ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "rgba(255,255,255,1)";
  ctx.fillRect(width * 0.08, height * 0.14, width * 0.28, height * 0.72);
  ctx.fillRect(width * 0.52, height * 0.58, width * 0.32, height * 0.2);
  ctx.beginPath();
  ctx.moveTo(width * 0.56, height * 0.16);
  ctx.lineTo(width * 0.88, height * 0.16);
  ctx.lineTo(width * 0.72, height * 0.46);
  ctx.closePath(); ctx.fill();
  return fixtureFromCanvas(canvas);
}

export function createFillFixture(mode) {
  const canvas = document.createElement("canvas");
  canvas.width = 120; canvas.height = 84;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, 120, 84);
  if (mode === "contiguous") {
    ctx.fillStyle = "rgba(255,255,255,1)"; ctx.fillRect(0, 0, 120, 84);
    ctx.fillStyle = "rgba(45,45,45,1)"; ctx.fillRect(8, 8, 104, 68);
    ctx.clearRect(18, 18, 42, 42); ctx.clearRect(68, 18, 22, 22);
  } else if (mode === "non-contiguous") {
    ctx.fillStyle = "rgba(255,255,255,1)"; ctx.fillRect(0, 0, 120, 84);
    ctx.fillStyle = "rgba(80,80,80,1)";
    for (const [x, y] of [[12, 12], [68, 12], [28, 42], [84, 44]]) {
      ctx.fillRect(x, y, 24, 20);
    }
  } else {
    for (let y = 0; y < 84; y++) {
      for (let x = 0; x < 120; x++) {
        const wobble = Math.sin((x + y) / 9) * 10;
        const val = Math.max(0, Math.min(255, 122 + wobble));
        ctx.fillStyle = `rgba(${val},${val},${val},1)`;
        ctx.fillRect(x, y, 1, 1);
      }
    }
  }
  return fixtureFromCanvas(canvas);
}

export function createSpriteFixtures() {
  return [
    ["#c9492d", "#f2d27f", "#101010"],
    ["#234fdd", "#ffffff", "#0e1328"],
    ["#157347", "#c7f2d8", "#0f2518"],
    ["#8956c4", "#f3dbff", "#27123f"],
  ].map(createSpriteFixture);
}

export function createSpriteFixture([primary, secondary, accent]) {
  const canvas = document.createElement("canvas");
  canvas.width = 8; canvas.height = 8;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, 8, 8);
  ctx.fillStyle = primary; ctx.fillRect(1, 1, 6, 6);
  ctx.fillStyle = secondary; ctx.fillRect(2, 2, 3, 3);
  ctx.fillStyle = accent; ctx.fillRect(5, 2, 1, 4); ctx.fillRect(2, 5, 4, 1);
  return fixtureFromCanvas(canvas);
}

export function createUtilityTile() {
  const canvas = document.createElement("canvas");
  canvas.width = 2; canvas.height = 2;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#234fdd"; ctx.fillRect(0, 0, 1, 1);
  ctx.fillStyle = "#c9492d"; ctx.fillRect(1, 0, 1, 1);
  ctx.fillStyle = "#157347"; ctx.fillRect(0, 1, 1, 1);
  ctx.fillStyle = "#f2d27f"; ctx.fillRect(1, 1, 1, 1);
  return fixtureFromCanvas(canvas);
}

function fixtureFromCanvas(canvas) {
  const ctx = canvas.getContext("2d");
  const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
  return { height: canvas.height, rgba: new Uint8Array(imageData.data), width: canvas.width };
}

async function loadBinaryFixture(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Failed to load ${url}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

async function loadTextFixture(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Failed to load ${url}: ${response.status}`);
  return response.text();
}

async function loadVolumeRasterFixtures() {
  const entries = await Promise.all(
    VOLUME_RASTER_ASSETS.filter((asset) => asset.key !== "teapot").map(async (asset) => {
      const loaded = await loadRasterFixture(asset.path, asset.maxEdge);
      return [
        asset.key,
        {
          key: asset.key,
          label: asset.label,
          path: asset.path,
          ...loaded,
        },
      ];
    }),
  );
  return Object.fromEntries(entries);
}

async function loadVolumeSvgFixtures() {
  const entries = await Promise.all(
    VOLUME_SVG_ASSETS.map(async (asset) => [
      asset.key,
      {
        key: asset.key,
        label: asset.label,
        path: asset.path,
        svg: await loadTextFixture(asset.path),
      },
    ]),
  );
  return Object.fromEntries(entries);
}

async function loadRasterFixture(url, maxEdge) {
  const sourceBytes = await loadBinaryFixture(url);
  const blob = new Blob([sourceBytes]);
  const bitmap = await createImageBitmap(blob);

  try {
    const originalWidth = bitmap.width;
    const originalHeight = bitmap.height;
    const scale = Math.min(1, maxEdge / Math.max(originalWidth, originalHeight));
    const width = Math.max(1, Math.round(originalWidth * scale));
    const height = Math.max(1, Math.round(originalHeight * scale));
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(bitmap, 0, 0, width, height);
    const data = ctx.getImageData(0, 0, width, height);

    return {
      sourceBytes,
      originalWidth,
      originalHeight,
      width,
      height,
      rgba: new Uint8Array(data.data),
    };
  } finally {
    bitmap.close();
  }
}

async function loadTeapotFixture(url, maxEdge) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Failed to load ${url}: ${response.status}`);
  const sourceBytes = new Uint8Array(await response.arrayBuffer());
  const { width: originalWidth, height: originalHeight } = parsePngSize(sourceBytes);
  const scale = Math.min(1, maxEdge / Math.max(originalWidth, originalHeight));
  const width = Math.max(1, Math.round(originalWidth * scale));
  const height = Math.max(1, Math.round(originalHeight * scale));
  const decoded = await decodeImage(sourceBytes);
  const sourceComposition = await Composition.create({ width: originalWidth, height: originalHeight });

  try {
    const layerId = sourceComposition.addImageLayer({ height: originalHeight, name: "teapot-source", rgba: decoded, width: originalWidth });
    if (width !== originalWidth || height !== originalHeight) {
      sourceComposition.resizeLayerBilinear(layerId, { width, height });
    }
    const rgba = new Uint8Array(sourceComposition.getLayerRgba(layerId));
    const workingComposition = await Composition.create({ width, height });
    try {
      workingComposition.addImageLayer({ height, name: "teapot-working", rgba, width });
      return { originalHeight, originalWidth, pngBytes: workingComposition.exportPng(), rgba, sourceBytes, height, width };
    } finally {
      workingComposition.free();
    }
  } finally {
    sourceComposition.free();
  }
}

function parsePngSize(bytes) {
  if (bytes.length < 24) throw new Error("PNG fixture is too short to contain an IHDR chunk");
  const sig = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  for (let i = 0; i < sig.length; i++) {
    if (bytes[i] !== sig[i]) throw new Error("Teapot fixture is not a valid PNG file");
  }
  if (String.fromCharCode(bytes[12], bytes[13], bytes[14], bytes[15]) !== "IHDR") {
    throw new Error("PNG fixture is missing the IHDR chunk");
  }
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return { height: dv.getUint32(20), width: dv.getUint32(16) };
}

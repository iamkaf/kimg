import initRaw, {
  Composition as RawComposition,
  clear_registered_fonts,
  contrast_ratio,
  document_has_svg_layers,
  decode_image,
  detect_format,
  dominant_rgb_from_rgba,
  extract_palette_from_rgba,
  hex_to_rgb,
  histogram_rgba,
  initSvg as initSvgRaw,
  initSvgSync as initSvgSyncRaw,
  initText as initTextRaw,
  initTextSvg as initTextSvgRaw,
  initTextSvgSync as initTextSvgSyncRaw,
  preloadTextSvgBindings,
  quantize_rgba,
  register_font,
  registered_font_count,
  relative_luminance,
  rgb_to_hex,
  type RawCompositionInstance,
  simdSupported,
  svgBackendActive,
  textBackendActive,
} from "./raw.js";
import type { InitInput, InitOutput } from "./raw.js";

export type ByteInput = ArrayBuffer | ArrayBufferView | ArrayLike<number>;
export type Anchor = "topLeft" | "top_left" | "center" | 0 | 1;
export type TextFontStyle = "normal" | "italic" | "oblique";
export type TextAlign = "left" | "center" | "right";
export type TextWrap = "none" | "word";
export type GradientDirection =
  | "horizontal"
  | "vertical"
  | "diagonalDown"
  | "diagonalUp"
  | "diagonal_down"
  | "diagonal_up"
  | 0
  | 1
  | 2
  | 3;

export interface CompositionOptions {
  width: number;
  height: number;
}

export interface ImageLayerOptions {
  name: string;
  rgba: ByteInput;
  width: number;
  height: number;
  x?: number;
  y?: number;
  parentId?: number;
}

export interface PaintLayerOptions {
  name: string;
  width: number;
  height: number;
  parentId?: number;
}

export interface FilterLayerOptions {
  name: string;
  parentId?: number;
}

export interface GroupLayerOptions {
  name: string;
  parentId?: number;
}

export interface SolidColorLayerOptions {
  name: string;
  color: ByteInput;
  parentId?: number;
}

export interface GradientStop {
  position: number;
  color: ByteInput;
}

export interface GradientLayerOptions {
  name: string;
  stops: GradientStop[];
  direction?: GradientDirection;
  parentId?: number;
}

export type ShapeLayerType = "rectangle" | "ellipse" | "line" | "polygon";

export interface ShapePoint {
  x: number;
  y: number;
}

export interface ShapeStrokeOptions {
  color: ByteInput;
  width: number;
}

export interface ShapeLayerOptions {
  name: string;
  type: ShapeLayerType;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  radius?: number;
  fill?: ByteInput;
  stroke?: ShapeStrokeOptions;
  points?: ShapePoint[];
  parentId?: number;
}

export interface TextLayerOptions {
  name: string;
  text: string;
  color: ByteInput;
  fontFamily?: string;
  fontWeight?: number;
  fontStyle?: TextFontStyle;
  fontSize?: number;
  lineHeight?: number;
  letterSpacing?: number;
  align?: TextAlign;
  wrap?: TextWrap;
  boxWidth?: number | null;
  x?: number;
  y?: number;
  parentId?: number;
}

export interface SvgLayerOptions {
  name: string;
  svg: string | ByteInput;
  width: number;
  height: number;
  x?: number;
  y?: number;
  parentId?: number;
}

export interface PngLayerOptions {
  name: string;
  png: ByteInput;
  x?: number;
  y?: number;
  parentId?: number;
}

export interface ImportImageOptions {
  name: string;
  bytes: ByteInput;
  x?: number;
  y?: number;
}

export interface MaskOptions {
  rgba: ByteInput;
  width: number;
  height: number;
  inverted?: boolean;
}

export interface BucketFillOptions {
  x: number;
  y: number;
  color: ByteInput;
  contiguous?: boolean;
  tolerance?: number;
}

export type BrushTool = "paint" | "erase";

export interface BrushPoint {
  x: number;
  y: number;
  pressure?: number;
}

export interface BrushStrokeOptions {
  points: BrushPoint[];
  tool?: BrushTool;
  color?: ByteInput;
  size: number;
  opacity?: number;
  flow?: number;
  hardness?: number;
  spacing?: number;
  smoothing?: number;
  pressureSize?: number;
  pressureOpacity?: number;
}

export interface Position {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface FlipOptions {
  flipX?: boolean;
  flipY?: boolean;
}

export interface FilterConfig {
  hue?: number;
  hueDeg?: number;
  saturation?: number;
  lightness?: number;
  alpha?: number;
  brightness?: number;
  contrast?: number;
  temperature?: number;
  tint?: number;
  sharpen?: number;
}

export interface FilterConfigSnapshot {
  hueDeg: number;
  saturation: number;
  lightness: number;
  alpha: number;
  brightness: number;
  contrast: number;
  temperature: number;
  tint: number;
  sharpen: number;
}

export interface TextLayerConfig {
  text?: string;
  color?: ByteInput;
  fontFamily?: string;
  fontWeight?: number;
  fontStyle?: TextFontStyle;
  fontSize?: number;
  lineHeight?: number;
  letterSpacing?: number;
  align?: TextAlign;
  wrap?: TextWrap;
  boxWidth?: number | null;
}

export interface RegisterFontOptions {
  bytes: ByteInput;
  family?: string;
  weight?: number;
  style?: TextFontStyle;
}

export type GoogleFontDisplay = "auto" | "block" | "fallback" | "optional" | "swap";

export interface LoadGoogleFontOptions {
  family: string;
  weights?: ArrayLike<number>;
  ital?: ArrayLike<number>;
  display?: GoogleFontDisplay;
  text?: string;
}

export interface LoadedGoogleFontFace {
  family: string;
  style: TextFontStyle;
  weight: number;
  url: string;
  format: string | null;
}

export interface LoadedGoogleFontResult {
  family: string;
  registeredFaces: number;
  faces: LoadedGoogleFontFace[];
  stylesheetUrl: string;
}

export interface ExportJpegOptions {
  quality?: number;
}

export interface LayerUpdate {
  name?: string;
  visible?: boolean;
  opacity?: number;
  x?: number;
  y?: number;
  blendMode?: string;
  maskInverted?: boolean;
  clipToBelow?: boolean;
  anchor?: Anchor;
  flipX?: boolean;
  flipY?: boolean;
  rotation?: number;
  scaleX?: number;
  scaleY?: number;
  filterConfig?: FilterConfig;
  textConfig?: TextLayerConfig;
}

export interface MoveLayerTarget {
  parentId?: number | null;
  index?: number;
}

export interface ListLayersOptions {
  parentId?: number | null;
  recursive?: boolean;
}

export interface RadiusOptions {
  radius: number;
}

export interface PosterizeOptions {
  levels: number;
}

export interface ThresholdOptions {
  threshold: number;
}

export interface LevelsOptions {
  shadows: number;
  midtones: number;
  highlights: number;
}

export interface PixelScaleOptions {
  factor: number;
}

export interface PaletteOptions {
  maxColors: number;
}

export interface QuantizeOptions {
  palette: ByteInput;
}

export interface PackSpritesOptions {
  layerIds: ArrayLike<number>;
  maxWidth: number;
  padding?: number;
}

export interface ContactSheetOptions {
  layerIds: ArrayLike<number>;
  columns: number;
  padding?: number;
  background?: ByteInput;
}

export interface GeometryOptions {
  width: number;
  height: number;
}

export interface GeometryWithPaletteOptions extends GeometryOptions {
  palette: ByteInput;
}

export interface GeometryWithMaxColorsOptions extends GeometryOptions {
  maxColors: number;
}

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export type LayerKind =
  | "raster"
  | "filter"
  | "group"
  | "fill"
  | "shape"
  | "text"
  | "svg"
  | "unknown";

export interface LayerInfo {
  id: number;
  parentId: number | null;
  index: number;
  depth: number;
  kind: LayerKind;
  name: string;
  visible: boolean;
  opacity: number;
  x: number;
  y: number;
  blendMode: string;
  hasMask: boolean;
  maskInverted: boolean;
  clipToBelow: boolean;
  width?: number;
  height?: number;
  anchor?: "topLeft" | "center";
  flipX?: boolean;
  flipY?: boolean;
  rotation?: number;
  scaleX?: number;
  scaleY?: number;
  filterConfig?: FilterConfigSnapshot;
  childCount?: number;
  fillType?: "solid" | "gradient";
  color?: number[];
  direction?: Exclude<GradientDirection, 0 | 1 | 2 | 3>;
  stopCount?: number;
  shapeType?: ShapeLayerType;
  radius?: number;
  pointCount?: number;
  fill?: number[];
  strokeColor?: number[];
  strokeWidth?: number;
  text?: string;
  fontFamily?: string;
  fontWeight?: number;
  fontStyle?: TextFontStyle;
  fontSize?: number;
  lineHeight?: number;
  letterSpacing?: number;
  align?: TextAlign;
  wrap?: TextWrap;
  boxWidth?: number | null;
}

const PRIVATE_CONSTRUCTOR_TOKEN = Symbol("kimgComposition");

const ANCHOR_TO_RAW = {
  center: 1,
  topLeft: 0,
  top_left: 0,
};

const GRADIENT_DIRECTION_TO_RAW = {
  diagonalDown: 2,
  diagonalUp: 3,
  diagonal_down: 2,
  diagonal_up: 3,
  horizontal: 0,
  vertical: 1,
};

let preloadPromise: Promise<InitOutput> | null = null;
let textPreloadPromise: Promise<InitOutput> | null = null;
let svgPreloadPromise: Promise<InitOutput> | null = null;
let textSvgPreloadPromise: Promise<InitOutput> | null = null;
const liveCompositions = new Set<Composition>();
const googleFontRequestCache = new Map<string, Promise<LoadedGoogleFontResult>>();
const googleFontBinaryCache = new Map<string, Promise<number>>();

type BackendKind = "base" | "text" | "svg" | "textSvg";

function isNodeRuntime() {
  const runtime = globalThis as typeof globalThis & {
    process?: {
      versions?: {
        node?: string;
      };
    };
  };

  return (
    typeof runtime.process === "object" &&
    runtime.process !== null &&
    typeof runtime.process.versions === "object" &&
    runtime.process.versions !== null &&
    typeof runtime.process.versions.node === "string"
  );
}

function requireObject(value, what) {
  if (typeof value !== "object" || value === null) {
    throw new TypeError(`${what} must be an object.`);
  }

  return value;
}

function normalizeCreateArgs(widthOrOptions, height) {
  if (typeof widthOrOptions === "object" && widthOrOptions !== null) {
    return {
      height: widthOrOptions.height,
      width: widthOrOptions.width,
    };
  }

  return {
    height,
    width: widthOrOptions,
  };
}

function normalizeFiniteNumber(value, fieldName) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`${fieldName} must be a finite number.`);
  }

  return value;
}

function normalizeInteger(value, fieldName) {
  return Math.trunc(normalizeFiniteNumber(value, fieldName));
}

function normalizeNonNegativeInteger(value, fieldName) {
  const normalized = normalizeInteger(value, fieldName);
  if (normalized < 0) {
    throw new RangeError(`${fieldName} must be greater than or equal to 0.`);
  }
  return normalized;
}

function normalizePositiveNumber(value, fieldName) {
  const normalized = normalizeFiniteNumber(value, fieldName);
  if (normalized <= 0) {
    throw new RangeError(`${fieldName} must be greater than 0.`);
  }
  return normalized;
}

function normalizeUnitInterval(value, fieldName) {
  const normalized = normalizeFiniteNumber(value, fieldName);
  if (normalized < 0 || normalized > 1) {
    throw new RangeError(`${fieldName} must be between 0 and 1.`);
  }
  return normalized;
}

function normalizePositiveInteger(value, fieldName) {
  const normalized = normalizeInteger(value, fieldName);
  if (normalized <= 0) {
    throw new RangeError(`${fieldName} must be greater than 0.`);
  }
  return normalized;
}

function clamp01(value) {
  return Math.min(1, Math.max(0, value));
}

function normalizeString(value, fieldName) {
  if (typeof value !== "string") {
    throw new TypeError(`${fieldName} must be a string.`);
  }

  return value;
}

function normalizeByteInput(value, fieldName): Uint8Array {
  if (value instanceof Uint8Array) {
    return value;
  }

  if (value instanceof ArrayBuffer) {
    return new Uint8Array(value);
  }

  if (ArrayBuffer.isView(value)) {
    return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  }

  if (Array.isArray(value) || (typeof value === "object" && value !== null && "length" in value)) {
    return Uint8Array.from(value);
  }

  throw new TypeError(`${fieldName} must be a byte array, ArrayBuffer, or array-like object.`);
}

function normalizeSvgSource(value, fieldName): Uint8Array {
  if (typeof value === "string") {
    return new TextEncoder().encode(value);
  }

  return normalizeByteInput(value, fieldName);
}

function normalizeFloat64Input(value, fieldName) {
  if (value instanceof Float64Array) {
    return value;
  }

  if (Array.isArray(value) || (typeof value === "object" && value !== null && "length" in value)) {
    return Float64Array.from(value);
  }

  throw new TypeError(`${fieldName} must be a Float64Array or array-like object.`);
}

function normalizeRgbaColor(value, fieldName): Uint8Array {
  const rgba = normalizeByteInput(value, fieldName);
  if (rgba.length !== 4) {
    throw new TypeError(`${fieldName} must contain exactly 4 RGBA bytes.`);
  }
  return rgba;
}

function normalizeByte(value, fieldName) {
  const normalized = normalizeInteger(value, fieldName);
  if (normalized < 0 || normalized > 255) {
    throw new RangeError(`${fieldName} must be between 0 and 255.`);
  }
  return normalized;
}

function normalizeOptionalRgbaColor(value, fieldName): Uint8Array {
  if (value === undefined || value === null) {
    return new Uint8Array();
  }
  return normalizeRgbaColor(value, fieldName);
}

function normalizeImagePlacement(options, what) {
  const object = requireObject(options, what);
  return {
    height: normalizeInteger(object.height, `${what}.height`),
    name: object.name,
    parentId: object.parentId,
    width: normalizeInteger(object.width, `${what}.width`),
    x: normalizeInteger(object.x ?? 0, `${what}.x`),
    y: normalizeInteger(object.y ?? 0, `${what}.y`),
  };
}

function normalizeAnchor(anchor) {
  if (typeof anchor === "number") {
    if (anchor === 0 || anchor === 1) {
      return anchor;
    }
  }

  if (typeof anchor === "string" && Object.hasOwn(ANCHOR_TO_RAW, anchor)) {
    return ANCHOR_TO_RAW[anchor];
  }

  throw new TypeError('anchor must be "topLeft", "top_left", "center", 0, or 1.');
}

function normalizeTextFontStyle(value, fieldName): TextFontStyle {
  const normalized = normalizeString(value, fieldName);
  if (normalized === "normal" || normalized === "italic" || normalized === "oblique") {
    return normalized;
  }
  throw new TypeError(`${fieldName} must be "normal", "italic", or "oblique".`);
}

function normalizeTextAlign(value, fieldName): TextAlign {
  const normalized = normalizeString(value, fieldName);
  if (normalized === "left" || normalized === "center" || normalized === "right") {
    return normalized;
  }
  throw new TypeError(`${fieldName} must be "left", "center", or "right".`);
}

function normalizeTextWrap(value, fieldName): TextWrap {
  const normalized = normalizeString(value, fieldName);
  if (normalized === "none" || normalized === "word") {
    return normalized;
  }
  throw new TypeError(`${fieldName} must be "none" or "word".`);
}

function normalizeFontWeight(value, fieldName) {
  const normalized = normalizePositiveInteger(value, fieldName);
  if (normalized > 1000) {
    throw new RangeError(`${fieldName} must be less than or equal to 1000.`);
  }
  return normalized;
}

function normalizeGoogleFontDisplay(value, fieldName): GoogleFontDisplay {
  const normalized = normalizeString(value, fieldName);
  switch (normalized) {
    case "auto":
    case "block":
    case "fallback":
    case "optional":
    case "swap":
      return normalized;
    default:
      throw new TypeError(
        `${fieldName} must be "auto", "block", "fallback", "optional", or "swap".`,
      );
  }
}

function normalizeGoogleFontFlagArray(
  value: ArrayLike<number> | undefined,
  fieldName: string,
  defaults: number[],
  allowed: number[],
): number[] {
  if (value === undefined) {
    return defaults;
  }

  if (
    !(Array.isArray(value) || (typeof value === "object" && value !== null && "length" in value))
  ) {
    throw new TypeError(`${fieldName} must be an array-like collection of numbers.`);
  }

  const normalized = Array.from(
    new Set(
      Array.from(value, (entry, index) => {
        const parsed = normalizeInteger(entry, `${fieldName}[${index}]`);
        if (!allowed.includes(parsed)) {
          throw new RangeError(`${fieldName}[${index}] must be one of ${allowed.join(", ")}.`);
        }
        return parsed;
      }),
    ),
  ).sort((a, b) => a - b);

  if (normalized.length === 0) {
    throw new TypeError(`${fieldName} must contain at least one entry.`);
  }

  return normalized;
}

function normalizeGoogleFontWeights(
  value: ArrayLike<number> | undefined,
  fieldName: string,
): number[] {
  if (value === undefined) {
    return [400];
  }

  if (
    !(Array.isArray(value) || (typeof value === "object" && value !== null && "length" in value))
  ) {
    throw new TypeError(`${fieldName} must be an array-like collection of weights.`);
  }

  const normalized = Array.from(
    new Set(
      Array.from(value, (entry, index) => {
        const parsed = normalizeFontWeight(entry, `${fieldName}[${index}]`);
        return parsed;
      }),
    ),
  ).sort((a, b) => a - b);

  if (normalized.length === 0) {
    throw new TypeError(`${fieldName} must contain at least one weight.`);
  }

  return normalized;
}

function normalizeRegisterFontOptions(options): RegisterFontOptions & { bytes: Uint8Array } {
  const object = requireObject(options, "registerFont");
  const normalized: RegisterFontOptions & { bytes: Uint8Array } = {
    bytes: normalizeByteInput(object.bytes, "registerFont.bytes"),
  };

  if ("family" in object && object.family !== undefined) {
    normalized.family = normalizeString(object.family, "registerFont.family");
  }
  if ("weight" in object && object.weight !== undefined) {
    normalized.weight = normalizeFontWeight(object.weight, "registerFont.weight");
  }
  if ("style" in object && object.style !== undefined) {
    normalized.style = normalizeTextFontStyle(object.style, "registerFont.style");
  }

  return normalized;
}

function normalizeLoadGoogleFontOptions(options): Required<LoadGoogleFontOptions> & {
  ital: number[];
  weights: number[];
} {
  const object = requireObject(options, "loadGoogleFont");

  return {
    display: normalizeGoogleFontDisplay(object.display ?? "swap", "loadGoogleFont.display"),
    family: normalizeString(object.family, "loadGoogleFont.family"),
    ital: normalizeGoogleFontFlagArray(object.ital, "loadGoogleFont.ital", [0], [0, 1]),
    text: object.text === undefined ? "" : normalizeString(object.text, "loadGoogleFont.text"),
    weights: normalizeGoogleFontWeights(object.weights, "loadGoogleFont.weights"),
  };
}

function buildGoogleFontsStylesheetUrl(
  options: Required<LoadGoogleFontOptions> & {
    ital: number[];
    weights: number[];
  },
): string {
  const params = new URLSearchParams();
  const familyName = options.family.trim().replace(/\s+/g, " ");
  const pairTokens: string[] = [];

  if (options.ital.length === 1 && options.ital[0] === 0) {
    params.set("family", `${familyName}:wght@${options.weights.join(";")}`);
  } else {
    for (const ital of options.ital) {
      for (const weight of options.weights) {
        pairTokens.push(`${ital},${weight}`);
      }
    }
    params.set("family", `${familyName}:ital,wght@${pairTokens.join(";")}`);
  }

  params.set("display", options.display);
  if (options.text.length > 0) {
    params.set("text", options.text);
  }

  return `https://fonts.googleapis.com/css2?${params.toString()}`;
}

function parseCssFontString(value: string): string {
  const normalized = value.trim();
  if (
    (normalized.startsWith('"') && normalized.endsWith('"')) ||
    (normalized.startsWith("'") && normalized.endsWith("'"))
  ) {
    return normalized.slice(1, -1);
  }
  return normalized;
}

function extractCssProperty(block: string, name: string): string | null {
  const pattern = new RegExp(`${name}\\s*:\\s*([^;]+);`, "i");
  const match = block.match(pattern);
  return match ? match[1].trim() : null;
}

function parseGoogleFontsStylesheet(css: string): LoadedGoogleFontFace[] {
  const blocks = css.match(/@font-face\s*\{[^}]*\}/g) ?? [];
  const faces: LoadedGoogleFontFace[] = [];

  for (const block of blocks) {
    const family = extractCssProperty(block, "font-family");
    const style = extractCssProperty(block, "font-style");
    const weight = extractCssProperty(block, "font-weight");
    const src = extractCssProperty(block, "src");

    if (family === null || style === null || weight === null || src === null) {
      continue;
    }

    const urlMatch = src.match(/url\(([^)]+)\)/i);
    if (urlMatch === null) {
      continue;
    }

    const formatMatch = src.match(/format\(([^)]+)\)/i);
    faces.push({
      family: parseCssFontString(family),
      format: formatMatch ? parseCssFontString(formatMatch[1]) : null,
      style: normalizeTextFontStyle(parseCssFontString(style), "google font style"),
      url: parseCssFontString(urlMatch[1]),
      weight: normalizeFontWeight(Number.parseInt(weight, 10), "google font weight"),
    });
  }

  return faces;
}

function normalizeGradientDirection(direction) {
  if (direction === undefined) {
    return 0;
  }

  if (
    typeof direction === "number" &&
    Number.isInteger(direction) &&
    direction >= 0 &&
    direction <= 3
  ) {
    return direction;
  }

  if (typeof direction === "string" && Object.hasOwn(GRADIENT_DIRECTION_TO_RAW, direction)) {
    return GRADIENT_DIRECTION_TO_RAW[direction];
  }

  throw new TypeError(
    'direction must be "horizontal", "vertical", "diagonalDown", "diagonalUp", "diagonal_down", "diagonal_up", or 0-3.',
  );
}

function normalizeGradientStops(stops) {
  if (!Array.isArray(stops) || stops.length === 0) {
    throw new TypeError("stops must be a non-empty array.");
  }

  const colors = new Uint8Array(stops.length * 4);
  const positions = new Float64Array(stops.length);

  for (let index = 0; index < stops.length; index += 1) {
    const stop = requireObject(stops[index], `stops[${index}]`);
    const color = normalizeRgbaColor(stop.color, `stops[${index}].color`);

    colors.set(color, index * 4);
    positions[index] = normalizeFiniteNumber(stop.position, `stops[${index}].position`);
  }

  return { colors, positions };
}

function normalizeShapeType(shapeType) {
  switch (shapeType) {
    case "rectangle":
    case "ellipse":
    case "line":
    case "polygon":
      return shapeType;
    case "roundedRect":
      return "rectangle";
    default:
      throw new TypeError('type must be "rectangle", "ellipse", "line", or "polygon".');
  }
}

function normalizeShapePoints(points, fieldName) {
  if (!Array.isArray(points) || points.length < 3) {
    throw new TypeError(`${fieldName} must be an array with at least 3 points.`);
  }

  const flat = new Int32Array(points.length * 2);
  let maxX = 0;
  let maxY = 0;

  for (let index = 0; index < points.length; index += 1) {
    const point = requireObject(points[index], `${fieldName}[${index}]`);
    const x = normalizeInteger(point.x, `${fieldName}[${index}].x`);
    const y = normalizeInteger(point.y, `${fieldName}[${index}].y`);
    if (x < 0 || y < 0) {
      throw new RangeError(`${fieldName}[${index}] coordinates must be >= 0.`);
    }
    flat[index * 2] = x;
    flat[index * 2 + 1] = y;
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  }

  return {
    height: maxY + 1,
    points: flat,
    width: maxX + 1,
  };
}

function normalizeShapeLayerOptions(options) {
  const layer = requireObject(options, "addShapeLayer");
  const type = normalizeShapeType(layer.type);
  const position = normalizePositionArg(layer.x, layer.y, "addShapeLayer");
  const fill = normalizeOptionalRgbaColor(layer.fill, "addShapeLayer.fill");
  let strokeColor: Uint8Array<ArrayBufferLike> = new Uint8Array();
  let strokeWidth = 0;

  if (layer.stroke !== undefined && layer.stroke !== null) {
    const stroke = requireObject(layer.stroke, "addShapeLayer.stroke");
    strokeColor = normalizeRgbaColor(stroke.color, "addShapeLayer.stroke.color");
    strokeWidth = normalizePositiveInteger(stroke.width, "addShapeLayer.stroke.width");
  }

  if (fill.length === 0 && strokeWidth === 0) {
    throw new TypeError("addShapeLayer requires a fill color, a stroke, or both.");
  }

  let width;
  let height;
  let points = new Int32Array();

  if (type === "polygon") {
    const polygon = normalizeShapePoints(layer.points, "addShapeLayer.points");
    points = polygon.points;
    width =
      layer.width === undefined
        ? polygon.width
        : normalizePositiveInteger(layer.width, "addShapeLayer.width");
    height =
      layer.height === undefined
        ? polygon.height
        : normalizePositiveInteger(layer.height, "addShapeLayer.height");
  } else {
    width = normalizePositiveInteger(layer.width, "addShapeLayer.width");
    height = normalizePositiveInteger(layer.height, "addShapeLayer.height");
  }

  const radius = normalizeInteger(layer.radius ?? 0, "addShapeLayer.radius");

  return {
    fill,
    height,
    name: normalizeString(layer.name, "addShapeLayer.name"),
    parentId: layer.parentId,
    points,
    radius: Math.max(radius, 0),
    strokeColor,
    strokeWidth,
    type,
    width,
    x: position.x,
    y: position.y,
  };
}

function normalizeTextLayerOptions(options) {
  const layer = requireObject(options, "addTextLayer");
  const position = normalizePositionArg(layer.x, layer.y, "addTextLayer");
  const fontSize = normalizePositiveInteger(layer.fontSize ?? 16, "addTextLayer.fontSize");
  const lineHeight = normalizePositiveInteger(
    layer.lineHeight ?? Math.max(fontSize + 2, fontSize),
    "addTextLayer.lineHeight",
  );

  return {
    align: normalizeTextAlign(layer.align ?? "left", "addTextLayer.align"),
    boxWidth:
      layer.boxWidth === undefined || layer.boxWidth === null
        ? null
        : normalizePositiveInteger(layer.boxWidth, "addTextLayer.boxWidth"),
    color: normalizeRgbaColor(layer.color, "addTextLayer.color"),
    fontFamily: normalizeString(layer.fontFamily ?? "sans-serif", "addTextLayer.fontFamily"),
    fontStyle: normalizeTextFontStyle(layer.fontStyle ?? "normal", "addTextLayer.fontStyle"),
    fontWeight: normalizeFontWeight(layer.fontWeight ?? 400, "addTextLayer.fontWeight"),
    fontSize,
    letterSpacing: normalizeNonNegativeInteger(
      layer.letterSpacing ?? 0,
      "addTextLayer.letterSpacing",
    ),
    lineHeight,
    name: normalizeString(layer.name, "addTextLayer.name"),
    parentId: layer.parentId,
    text: normalizeString(layer.text, "addTextLayer.text"),
    wrap: normalizeTextWrap(layer.wrap ?? "none", "addTextLayer.wrap"),
    x: position.x,
    y: position.y,
  };
}

function normalizeSvgLayerOptions(options) {
  const layer = requireObject(options, "addSvgLayer");
  const position = normalizePositionArg(layer.x, layer.y, "addSvgLayer");

  return {
    height: normalizePositiveInteger(layer.height, "addSvgLayer.height"),
    name: normalizeString(layer.name, "addSvgLayer.name"),
    parentId: layer.parentId,
    svg: normalizeSvgSource(layer.svg, "addSvgLayer.svg"),
    width: normalizePositiveInteger(layer.width, "addSvgLayer.width"),
    x: position.x,
    y: position.y,
  };
}

function normalizeLayerId(id, fieldName = "id") {
  return normalizeInteger(id, fieldName);
}

function normalizeSizeArg(widthOrOptions, height, what) {
  if (typeof widthOrOptions === "object" && widthOrOptions !== null) {
    return {
      height: normalizeInteger(widthOrOptions.height, `${what}.height`),
      width: normalizeInteger(widthOrOptions.width, `${what}.width`),
    };
  }

  return {
    height: normalizeInteger(height, `${what}.height`),
    width: normalizeInteger(widthOrOptions, `${what}.width`),
  };
}

function normalizePositionArg(xOrOptions, y, what) {
  if (typeof xOrOptions === "object" && xOrOptions !== null) {
    return {
      x: normalizeInteger(xOrOptions.x ?? 0, `${what}.x`),
      y: normalizeInteger(xOrOptions.y ?? 0, `${what}.y`),
    };
  }

  return {
    x: normalizeInteger(xOrOptions ?? 0, `${what}.x`),
    y: normalizeInteger(y ?? 0, `${what}.y`),
  };
}

function normalizeFlipArg(flipXOrOptions, flipY) {
  if (typeof flipXOrOptions === "object" && flipXOrOptions !== null) {
    return {
      flipX: Boolean(flipXOrOptions.flipX),
      flipY: Boolean(flipXOrOptions.flipY),
    };
  }

  return {
    flipX: Boolean(flipXOrOptions),
    flipY: Boolean(flipY),
  };
}

function normalizeBucketFillOptions(options) {
  const object = requireObject(options, "bucketFillLayer");
  return {
    color: normalizeRgbaColor(object.color, "bucketFillLayer.color"),
    contiguous: object.contiguous ?? true,
    tolerance: normalizeByte(object.tolerance ?? 0, "bucketFillLayer.tolerance"),
    x: normalizeNonNegativeInteger(object.x, "bucketFillLayer.x"),
    y: normalizeNonNegativeInteger(object.y, "bucketFillLayer.y"),
  };
}

function normalizeBrushStrokeOptions(options) {
  const object = requireObject(options, "paintStrokeLayer");
  if (!Array.isArray(object.points) || object.points.length === 0) {
    throw new TypeError("paintStrokeLayer.points must be a non-empty array.");
  }

  const tool =
    object.tool === undefined ? "paint" : normalizeString(object.tool, "paintStrokeLayer.tool");
  if (tool !== "paint" && tool !== "erase") {
    throw new TypeError('paintStrokeLayer.tool must be "paint" or "erase".');
  }

  const points = new Float32Array(object.points.length * 3);
  for (let index = 0; index < object.points.length; index += 1) {
    const point = requireObject(object.points[index], `paintStrokeLayer.points[${index}]`);
    points[index * 3] = normalizeFiniteNumber(point.x, `paintStrokeLayer.points[${index}].x`);
    points[index * 3 + 1] = normalizeFiniteNumber(point.y, `paintStrokeLayer.points[${index}].y`);
    points[index * 3 + 2] = normalizeUnitInterval(
      point.pressure ?? 1,
      `paintStrokeLayer.points[${index}].pressure`,
    );
  }

  let color: Uint8Array<ArrayBufferLike> = new Uint8Array([0, 0, 0, 255]);
  if (tool === "paint") {
    color = normalizeRgbaColor(object.color ?? [0, 0, 0, 255], "paintStrokeLayer.color");
  } else if (object.color !== undefined) {
    color = normalizeRgbaColor(object.color, "paintStrokeLayer.color");
  }

  return {
    color,
    flow: normalizeUnitInterval(object.flow ?? 1, "paintStrokeLayer.flow"),
    hardness: normalizeUnitInterval(object.hardness ?? 1, "paintStrokeLayer.hardness"),
    opacity: normalizeUnitInterval(object.opacity ?? 1, "paintStrokeLayer.opacity"),
    points,
    pressureOpacity: normalizeUnitInterval(
      object.pressureOpacity ?? 0,
      "paintStrokeLayer.pressureOpacity",
    ),
    pressureSize: normalizeUnitInterval(object.pressureSize ?? 1, "paintStrokeLayer.pressureSize"),
    size: normalizePositiveNumber(object.size, "paintStrokeLayer.size"),
    smoothing: normalizeUnitInterval(object.smoothing ?? 0, "paintStrokeLayer.smoothing"),
    spacing: normalizePositiveNumber(object.spacing ?? 0.25, "paintStrokeLayer.spacing"),
    tool,
  };
}

function normalizeListLayersOptions(options) {
  if (options === undefined) {
    return {
      parentId: -1,
      recursive: true,
    };
  }

  const object = requireObject(options, "listLayers");
  return {
    parentId:
      object.parentId == null ? -1 : normalizeLayerId(object.parentId, "listLayers.parentId"),
    recursive: object.recursive ?? true,
  };
}

function normalizeMoveLayerTarget(target) {
  const object = requireObject(target, "moveLayer");
  return {
    index: object.index == null ? -1 : normalizeInteger(object.index, "moveLayer.index"),
    parentId:
      object.parentId == null ? -1 : normalizeLayerId(object.parentId, "moveLayer.parentId"),
  };
}

function normalizeExportJpegArg(qualityOrOptions) {
  if (typeof qualityOrOptions === "object" && qualityOrOptions !== null) {
    return normalizeInteger(qualityOrOptions.quality ?? 85, "exportJpeg.quality");
  }

  return normalizeInteger(qualityOrOptions ?? 85, "exportJpeg.quality");
}

function normalizeFilterConfigPatch(config, what) {
  const object = requireObject(config, what);
  const normalized: FilterConfig = {};

  if ("hue" in object && object.hue !== undefined) {
    normalized.hue = normalizeFiniteNumber(object.hue, `${what}.hue`);
  }
  if ("hueDeg" in object && object.hueDeg !== undefined) {
    normalized.hueDeg = normalizeFiniteNumber(object.hueDeg, `${what}.hueDeg`);
  }
  if ("saturation" in object && object.saturation !== undefined) {
    normalized.saturation = normalizeFiniteNumber(object.saturation, `${what}.saturation`);
  }
  if ("lightness" in object && object.lightness !== undefined) {
    normalized.lightness = normalizeFiniteNumber(object.lightness, `${what}.lightness`);
  }
  if ("alpha" in object && object.alpha !== undefined) {
    normalized.alpha = normalizeFiniteNumber(object.alpha, `${what}.alpha`);
  }
  if ("brightness" in object && object.brightness !== undefined) {
    normalized.brightness = normalizeFiniteNumber(object.brightness, `${what}.brightness`);
  }
  if ("contrast" in object && object.contrast !== undefined) {
    normalized.contrast = normalizeFiniteNumber(object.contrast, `${what}.contrast`);
  }
  if ("temperature" in object && object.temperature !== undefined) {
    normalized.temperature = normalizeFiniteNumber(object.temperature, `${what}.temperature`);
  }
  if ("tint" in object && object.tint !== undefined) {
    normalized.tint = normalizeFiniteNumber(object.tint, `${what}.tint`);
  }
  if ("sharpen" in object && object.sharpen !== undefined) {
    normalized.sharpen = normalizeFiniteNumber(object.sharpen, `${what}.sharpen`);
  }

  return normalized;
}

function normalizeTextConfigPatch(config, what): TextLayerConfig {
  const object = requireObject(config, what);
  const normalized: TextLayerConfig = {};

  if ("text" in object && object.text !== undefined) {
    normalized.text = normalizeString(object.text, `${what}.text`);
  }
  if ("color" in object && object.color !== undefined) {
    normalized.color = normalizeRgbaColor(object.color, `${what}.color`);
  }
  if ("fontFamily" in object && object.fontFamily !== undefined) {
    normalized.fontFamily = normalizeString(object.fontFamily, `${what}.fontFamily`);
  }
  if ("fontWeight" in object && object.fontWeight !== undefined) {
    normalized.fontWeight = normalizeFontWeight(object.fontWeight, `${what}.fontWeight`);
  }
  if ("fontStyle" in object && object.fontStyle !== undefined) {
    normalized.fontStyle = normalizeTextFontStyle(object.fontStyle, `${what}.fontStyle`);
  }
  if ("fontSize" in object && object.fontSize !== undefined) {
    normalized.fontSize = normalizePositiveInteger(object.fontSize, `${what}.fontSize`);
  }
  if ("lineHeight" in object && object.lineHeight !== undefined) {
    normalized.lineHeight = normalizePositiveInteger(object.lineHeight, `${what}.lineHeight`);
  }
  if ("letterSpacing" in object && object.letterSpacing !== undefined) {
    normalized.letterSpacing = normalizeNonNegativeInteger(
      object.letterSpacing,
      `${what}.letterSpacing`,
    );
  }
  if ("align" in object && object.align !== undefined) {
    normalized.align = normalizeTextAlign(object.align, `${what}.align`);
  }
  if ("wrap" in object && object.wrap !== undefined) {
    normalized.wrap = normalizeTextWrap(object.wrap, `${what}.wrap`);
  }
  if ("boxWidth" in object && object.boxWidth !== undefined) {
    normalized.boxWidth =
      object.boxWidth === null
        ? null
        : normalizePositiveInteger(object.boxWidth, `${what}.boxWidth`);
  }

  return normalized;
}

function normalizeLayerUpdatePatch(patch) {
  const object = requireObject(patch, "updateLayer");
  const normalized: LayerUpdate = {};

  if ("name" in object && object.name !== undefined) {
    normalized.name = normalizeString(object.name, "updateLayer.name");
  }
  if ("visible" in object && object.visible !== undefined) {
    normalized.visible = Boolean(object.visible);
  }
  if ("opacity" in object && object.opacity !== undefined) {
    normalized.opacity = normalizeFiniteNumber(object.opacity, "updateLayer.opacity");
  }
  if ("x" in object && object.x !== undefined) {
    normalized.x = normalizeInteger(object.x, "updateLayer.x");
  }
  if ("y" in object && object.y !== undefined) {
    normalized.y = normalizeInteger(object.y, "updateLayer.y");
  }
  if ("blendMode" in object && object.blendMode !== undefined) {
    normalized.blendMode = normalizeString(object.blendMode, "updateLayer.blendMode");
  }
  if ("maskInverted" in object && object.maskInverted !== undefined) {
    normalized.maskInverted = Boolean(object.maskInverted);
  }
  if ("clipToBelow" in object && object.clipToBelow !== undefined) {
    normalized.clipToBelow = Boolean(object.clipToBelow);
  }
  if ("anchor" in object && object.anchor !== undefined) {
    normalized.anchor = normalizeAnchor(object.anchor) === 1 ? "center" : "topLeft";
  }
  if ("flipX" in object && object.flipX !== undefined) {
    normalized.flipX = Boolean(object.flipX);
  }
  if ("flipY" in object && object.flipY !== undefined) {
    normalized.flipY = Boolean(object.flipY);
  }
  if ("rotation" in object && object.rotation !== undefined) {
    normalized.rotation = normalizeFiniteNumber(object.rotation, "updateLayer.rotation");
  }
  if ("scaleX" in object && object.scaleX !== undefined) {
    normalized.scaleX = normalizePositiveNumber(object.scaleX, "updateLayer.scaleX");
  }
  if ("scaleY" in object && object.scaleY !== undefined) {
    normalized.scaleY = normalizePositiveNumber(object.scaleY, "updateLayer.scaleY");
  }

  const filterConfig = object.filterConfig ?? object.filter;
  if (filterConfig !== undefined) {
    normalized.filterConfig = normalizeFilterConfigPatch(filterConfig, "updateLayer.filterConfig");
  }
  if ("textConfig" in object && object.textConfig !== undefined) {
    normalized.textConfig = normalizeTextConfigPatch(object.textConfig, "updateLayer.textConfig");
  }

  return normalized;
}

function normalizeRgbArgs(rOrOptions, g, b) {
  if (typeof rOrOptions === "object" && rOrOptions !== null) {
    return {
      b: normalizeInteger(rOrOptions.b, "rgb.b"),
      g: normalizeInteger(rOrOptions.g, "rgb.g"),
      r: normalizeInteger(rOrOptions.r, "rgb.r"),
    };
  }

  return {
    b: normalizeInteger(b, "rgb.b"),
    g: normalizeInteger(g, "rgb.g"),
    r: normalizeInteger(rOrOptions, "rgb.r"),
  };
}

function normalizeImageGeometryArg(widthOrOptions, height, what) {
  return normalizeSizeArg(widthOrOptions, height, what);
}

function normalizeLayerIdArray(ids, fieldName) {
  if (ids instanceof Uint32Array) {
    return ids;
  }

  if (Array.isArray(ids) || (typeof ids === "object" && ids !== null && "length" in ids)) {
    return Uint32Array.from(ids);
  }

  throw new TypeError(`${fieldName} must be a Uint32Array or array-like collection of layer ids.`);
}

function backendKindFromFlags(textBackend: boolean, svgBackend: boolean): BackendKind {
  if (textBackend && svgBackend) {
    return "textSvg";
  }
  if (textBackend) {
    return "text";
  }
  if (svgBackend) {
    return "svg";
  }
  return "base";
}

function currentBackendKind(): BackendKind {
  return backendKindFromFlags(textBackendActive(), svgBackendActive());
}

function backendSatisfies(current: BackendKind, required: BackendKind): boolean {
  switch (required) {
    case "base":
      return true;
    case "text":
      return current === "text" || current === "textSvg";
    case "svg":
      return current === "svg" || current === "textSvg";
    case "textSvg":
      return current === "textSvg";
  }
}

function wasmFileNameForBackend(kind: BackendKind): string {
  switch (kind) {
    case "textSvg":
      return simdSupported() ? "kimg_wasm_text_svg_simd_bg.wasm" : "kimg_wasm_text_svg_bg.wasm";
    case "text":
      return simdSupported() ? "kimg_wasm_text_simd_bg.wasm" : "kimg_wasm_text_bg.wasm";
    case "svg":
      return simdSupported() ? "kimg_wasm_svg_simd_bg.wasm" : "kimg_wasm_svg_bg.wasm";
    case "base":
    default:
      return simdSupported() ? "kimg_wasm_simd_bg.wasm" : "kimg_wasm_bg.wasm";
  }
}

async function getDefaultInitInput(kind: BackendKind): Promise<Uint8Array | undefined> {
  if (!isNodeRuntime()) {
    return undefined;
  }

  // @ts-ignore Node built-in typing is only available in Node environments.
  const { readFile } = await import("node:fs/promises");
  const wasmName = wasmFileNameForBackend(kind);
  return readFile(new URL(`./${wasmName}`, import.meta.url));
}

function getBrowserInitInputSync(kind: BackendKind): Uint8Array {
  const request = new XMLHttpRequest();
  request.open("GET", new URL(`./${wasmFileNameForBackend(kind)}`, import.meta.url), false);
  request.overrideMimeType("text/plain; charset=x-user-defined");
  request.send();

  if (request.status !== 0 && (request.status < 200 || request.status >= 300)) {
    throw new Error(`Failed to load ${wasmFileNameForBackend(kind)}: HTTP ${request.status}`);
  }

  if (typeof request.responseText !== "string" || request.responseText.length === 0) {
    throw new Error(`Failed to load ${wasmFileNameForBackend(kind)}.`);
  }

  const bytes = new Uint8Array(request.responseText.length);
  for (let index = 0; index < request.responseText.length; index += 1) {
    bytes[index] = request.responseText.charCodeAt(index) & 0xff;
  }

  return bytes;
}

async function withPreload<T>(fn: () => T): Promise<T> {
  await preload();
  return fn();
}

function upgradeCompositionBackend(composition: Composition): void {
  const serialized = composition._inner.serialize();
  composition._inner.free();
  composition._inner = RawComposition.deserialize(serialized) as RawCompositionInstance;
  composition._backendKind = currentBackendKind();
}

function refreshLiveCompositions(): void {
  for (const composition of liveCompositions) {
    upgradeCompositionBackend(composition);
  }
}

function preloadPromiseForKind(kind: BackendKind): Promise<InitOutput> | null {
  switch (kind) {
    case "base":
      return preloadPromise;
    case "text":
      return textPreloadPromise;
    case "svg":
      return svgPreloadPromise;
    case "textSvg":
      return textSvgPreloadPromise;
  }
}

function setPreloadPromiseForKind(kind: BackendKind, promise: Promise<InitOutput> | null): void {
  switch (kind) {
    case "base":
      preloadPromise = promise;
      break;
    case "text":
      textPreloadPromise = promise;
      break;
    case "svg":
      svgPreloadPromise = promise;
      break;
    case "textSvg":
      textSvgPreloadPromise = promise;
      break;
  }
}

async function preloadBackendKind(
  kind: BackendKind,
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  const existing = preloadPromiseForKind(kind);
  if (existing !== null && backendSatisfies(currentBackendKind(), kind)) {
    return existing;
  }

  const promise = (async () => {
    if (module_or_path !== undefined) {
      switch (kind) {
        case "text":
          return initTextRaw(module_or_path);
        case "svg":
          return initSvgRaw(module_or_path);
        case "textSvg":
          return initTextSvgRaw(module_or_path);
        case "base":
        default:
          return initRaw(module_or_path);
      }
    }

    const defaultInput = await getDefaultInitInput(kind);
    if (defaultInput !== undefined) {
      const input = { module_or_path: defaultInput };
      switch (kind) {
        case "text":
          return initTextRaw(input);
        case "svg":
          return initSvgRaw(input);
        case "textSvg":
          return initTextSvgRaw(input);
        case "base":
        default:
          return initRaw(input);
      }
    }

    switch (kind) {
      case "text":
        return initTextRaw();
      case "svg":
        return initSvgRaw();
      case "textSvg":
        return initTextSvgRaw();
      case "base":
      default:
        return initRaw();
    }
  })().catch((error) => {
    setPreloadPromiseForKind(kind, null);
    throw error;
  });

  setPreloadPromiseForKind(kind, promise);
  if (kind === "text" || kind === "textSvg" || (kind === "svg" && isNodeRuntime())) {
    preloadPromise = promise;
  }
  return promise;
}

function ensureBackendReadySync(kind: BackendKind): void {
  const current = currentBackendKind();
  if (backendSatisfies(current, kind)) {
    return;
  }

  if (isNodeRuntime()) {
    return;
  }

  const input = { module: getBrowserInitInputSync(kind) };
  switch (kind) {
    case "svg":
      initSvgSyncRaw(input);
      break;
    case "textSvg":
      initTextSvgSyncRaw(input);
      break;
    case "base":
    case "text":
    default:
      break;
  }
  refreshLiveCompositions();
}

async function preloadText(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  return preloadBackendKind(
    svgBackendActive() || isNodeRuntime() ? "textSvg" : "text",
    module_or_path,
  );
}

export async function preloadSvg(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  return preloadBackendKind(
    textBackendActive() || isNodeRuntime() ? "textSvg" : "svg",
    module_or_path,
  );
}

async function ensureTextBackendReady(): Promise<void> {
  if (!textBackendActive()) {
    await preloadText();
  }
  refreshLiveCompositions();
}

async function ensureSvgBackendReady(): Promise<void> {
  if (!svgBackendActive()) {
    await preloadSvg();
  }
  refreshLiveCompositions();
}

async function prepareBrowserSvgUpgradePath(): Promise<void> {
  if (isNodeRuntime()) {
    return;
  }

  await preloadTextSvgBindings();
}

export function preload(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput>;
export async function preload(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
) {
  if (preloadPromise !== null) {
    return preloadPromise;
  }

  preloadPromise = (async () => {
    if (isNodeRuntime() && module_or_path === undefined) {
      return preloadBackendKind("textSvg");
    }

    if (module_or_path !== undefined) {
      return preloadBackendKind("base", module_or_path);
    }

    const defaultInput = await getDefaultInitInput("base");
    if (defaultInput !== undefined) {
      return initRaw({ module_or_path: defaultInput });
    }

    return initRaw();
  })().catch((error) => {
    preloadPromise = null;
    throw error;
  });

  return preloadPromise;
}

export async function registerFont(options: RegisterFontOptions): Promise<number> {
  const normalized = normalizeRegisterFontOptions(options);
  await ensureTextBackendReady();
  const loadedFaces = register_font(normalized.bytes);

  if (loadedFaces === 0) {
    const familyHint = normalized.family ? ` for ${normalized.family}` : "";
    throw new Error(`registerFont could not parse any usable font faces${familyHint}.`);
  }

  refreshLiveCompositions();
  return loadedFaces;
}

export async function clearRegisteredFonts(): Promise<void> {
  googleFontRequestCache.clear();
  googleFontBinaryCache.clear();

  if (
    !textBackendActive() &&
    !svgBackendActive() &&
    textPreloadPromise === null &&
    textSvgPreloadPromise === null &&
    !isNodeRuntime()
  ) {
    return;
  }

  await ensureTextBackendReady();
  clear_registered_fonts();
  refreshLiveCompositions();
}

export async function registeredFontCount(): Promise<number> {
  if (
    !textBackendActive() &&
    !svgBackendActive() &&
    textPreloadPromise === null &&
    textSvgPreloadPromise === null &&
    !isNodeRuntime()
  ) {
    return 0;
  }

  await ensureTextBackendReady();
  return registered_font_count();
}

export async function loadGoogleFont(
  options: LoadGoogleFontOptions,
): Promise<LoadedGoogleFontResult> {
  if (isNodeRuntime()) {
    throw new Error("loadGoogleFont() is browser-only. Use registerFont() with raw bytes on Node.");
  }

  const normalized = normalizeLoadGoogleFontOptions(options);
  const stylesheetUrl = buildGoogleFontsStylesheetUrl(normalized);

  const cacheKey = JSON.stringify({
    display: normalized.display,
    family: normalized.family,
    ital: normalized.ital,
    text: normalized.text,
    weights: normalized.weights,
  });
  const cached = googleFontRequestCache.get(cacheKey);
  if (cached !== undefined) {
    return cached;
  }

  const request = (async () => {
    await ensureTextBackendReady();

    const stylesheetResponse = await fetch(stylesheetUrl);
    if (!stylesheetResponse.ok) {
      throw new Error(`Google Fonts stylesheet request failed: ${stylesheetResponse.status}`);
    }

    const css = await stylesheetResponse.text();
    const faces = parseGoogleFontsStylesheet(css);
    if (faces.length === 0) {
      throw new Error("Google Fonts stylesheet did not contain any usable @font-face rules.");
    }

    let registeredFaces = 0;
    for (const face of faces) {
      let fontLoad = googleFontBinaryCache.get(face.url);
      if (fontLoad === undefined) {
        fontLoad = (async () => {
          const fontResponse = await fetch(face.url);
          if (!fontResponse.ok) {
            throw new Error(
              `Google Fonts font request failed: ${fontResponse.status} for ${face.url}`,
            );
          }

          return registerFont({
            bytes: new Uint8Array(await fontResponse.arrayBuffer()),
            family: face.family,
            style: face.style,
            weight: face.weight,
          });
        })();
        googleFontBinaryCache.set(face.url, fontLoad);
      }

      registeredFaces += await fontLoad;
    }

    return {
      faces,
      family: normalized.family,
      registeredFaces,
      stylesheetUrl,
    };
  })().catch((error) => {
    googleFontRequestCache.delete(cacheKey);
    throw error;
  });

  googleFontRequestCache.set(cacheKey, request);
  return request;
}

export interface Composition {
  free(): void;
  [Symbol.dispose](): void;
  addImageLayer(options: ImageLayerOptions): number;
  addPaintLayer(options: PaintLayerOptions): number;
  addFilterLayer(options: FilterLayerOptions): number;
  addGroupLayer(options: GroupLayerOptions): number;
  addSolidColorLayer(options: SolidColorLayerOptions): number;
  addGradientLayer(options: GradientLayerOptions): number;
  addShapeLayer(options: ShapeLayerOptions): number;
  addTextLayer(options: TextLayerOptions): number;
  addSvgLayer(options: SvgLayerOptions): number;
  addPngLayer(options: PngLayerOptions): number;
  importImage(options: ImportImageOptions): number;
  importJpeg(options: ImportImageOptions): number;
  importWebp(options: ImportImageOptions): number;
  importGifFrames(options: { bytes: ByteInput }): Uint32Array;
  importPsd(options: { bytes: ByteInput }): Uint32Array;
  setLayerOpacity(id: number, opacity: number): void;
  setLayerVisibility(id: number, visible: boolean): void;
  setLayerPosition(id: number, xOrPosition: number | Position, y?: number): void;
  setLayerBlendMode(id: number, blendMode: string): void;
  setLayerMask(id: number, options: MaskOptions): void;
  bucketFillLayer(id: number, options: BucketFillOptions): boolean;
  clearLayerMask(id: number): void;
  setLayerMaskInverted(id: number, inverted: boolean): void;
  setLayerClipToBelow(id: number, clipToBelow: boolean): void;
  setLayerFlip(id: number, flipXOrOptions: boolean | FlipOptions, flipY?: boolean): void;
  setLayerRotation(id: number, rotation: number): void;
  setLayerAnchor(id: number, anchor: Anchor): void;
  setFilterLayerConfig(id: number, config: FilterConfig): void;
  updateLayer(id: number, patch: LayerUpdate): boolean;
  getLayer(id: number): LayerInfo | null;
  listLayers(options?: ListLayersOptions): LayerInfo[];
  removeLayer(id: number): boolean;
  moveLayer(id: number, target: MoveLayerTarget): boolean;
  resizeCanvas(widthOrSize: number | Size, height?: number): void;
  flattenGroup(groupId: number): boolean;
  removeFromGroup(groupId: number, childId: number): boolean;
  renderRgba(): Uint8Array;
  exportPng(): Uint8Array;
  exportJpeg(quality?: number | ExportJpegOptions): Uint8Array;
  exportWebp(): Uint8Array;
  serialize(): Uint8Array;
  getLayerRgba(id: number): Uint8Array;
  rasterizeSvgLayer(id: number): boolean;
  layerCount(): number;
  resizeLayerNearest(id: number, widthOrSize: number | Size, height?: number): void;
  resizeLayerBilinear(id: number, widthOrSize: number | Size, height?: number): void;
  resizeLayerLanczos3(id: number, widthOrSize: number | Size, height?: number): void;
  cropLayer(
    id: number,
    xOrRect: number | (Position & Size),
    y?: number,
    width?: number,
    height?: number,
  ): void;
  trimLayerAlpha(id: number): void;
  rotateLayer(id: number, angleDeg: number): void;
  boxBlurLayer(id: number, radius: number | RadiusOptions): void;
  gaussianBlurLayer(id: number, radius: number | RadiusOptions): void;
  sharpenLayer(id: number): void;
  edgeDetectLayer(id: number): void;
  embossLayer(id: number): void;
  invertLayer(id: number): void;
  posterizeLayer(id: number, levels: number | PosterizeOptions): void;
  thresholdLayer(id: number, threshold: number | ThresholdOptions): void;
  levelsLayer(
    id: number,
    shadowsOrOptions: number | LevelsOptions,
    midtones?: number,
    highlights?: number,
  ): void;
  gradientMapLayer(id: number, stops: GradientStop[] | { stops: GradientStop[] }): void;
  pixelScaleLayer(id: number, factor: number | PixelScaleOptions): void;
  extractPalette(id: number, maxColors: number | PaletteOptions): Uint8Array;
  quantizeLayer(id: number, palette: ByteInput | QuantizeOptions): void;
  packSprites(
    layerIdsOrOptions: ArrayLike<number> | PackSpritesOptions,
    maxWidth?: number,
    padding?: number,
  ): Uint8Array;
  packSpritesJson(
    layerIdsOrOptions: ArrayLike<number> | PackSpritesOptions,
    maxWidth?: number,
    padding?: number,
  ): string;
  contactSheet(
    layerIdsOrOptions: ArrayLike<number> | ContactSheetOptions,
    columns?: number,
    padding?: number,
    background?: ByteInput,
  ): Uint8Array;
}

export class Composition {
  _inner!: RawCompositionInstance;
  _backendKind!: BackendKind;

  static #fromInner(inner: RawCompositionInstance) {
    return new Composition(inner, PRIVATE_CONSTRUCTOR_TOKEN, currentBackendKind());
  }

  private constructor(inner: RawCompositionInstance, token: symbol, backendKind: BackendKind) {
    if (token !== PRIVATE_CONSTRUCTOR_TOKEN) {
      throw new TypeError("Use await Composition.create(...) instead of new Composition(...).");
    }

    Object.defineProperty(this, "_inner", {
      configurable: false,
      enumerable: false,
      value: inner,
      writable: true,
    });
    Object.defineProperty(this, "_backendKind", {
      configurable: false,
      enumerable: false,
      value: backendKind,
      writable: true,
    });

    liveCompositions.add(this);
  }

  static async create(width: number, height: number): Promise<Composition>;
  static async create(options: CompositionOptions): Promise<Composition>;
  static async create(
    widthOrOptions: number | CompositionOptions,
    height?: number,
  ): Promise<Composition> {
    const size = normalizeCreateArgs(widthOrOptions, height);
    await preloadText();
    await prepareBrowserSvgUpgradePath();
    return Composition.#fromInner(
      new RawComposition(size.width, size.height) as RawCompositionInstance,
    );
  }

  static async deserialize(data: ByteInput): Promise<Composition> {
    const bytes = normalizeByteInput(data, "data");
    if (document_has_svg_layers(bytes)) {
      await preloadBackendKind("textSvg");
    } else {
      await preloadText();
      await prepareBrowserSvgUpgradePath();
    }
    return Composition.#fromInner(RawComposition.deserialize(bytes) as RawCompositionInstance);
  }

  get width(): number {
    return this._inner.width;
  }

  get height(): number {
    return this._inner.height;
  }

  free(): void {
    liveCompositions.delete(this);
    return this._inner.free();
  }

  addImageLayer(options) {
    const layer = normalizeImagePlacement(options, "addImageLayer");
    const rgba = normalizeByteInput(options.rgba, "addImageLayer.rgba");

    if (layer.parentId !== undefined) {
      return this._inner.add_image_to_group(
        normalizeLayerId(layer.parentId, "addImageLayer.parentId"),
        layer.name,
        rgba,
        layer.width,
        layer.height,
        layer.x,
        layer.y,
      );
    }

    return this._inner.add_image_layer(
      layer.name,
      rgba,
      layer.width,
      layer.height,
      layer.x,
      layer.y,
    );
  }

  addPaintLayer(options) {
    const layer = normalizeImagePlacement(options, "addPaintLayer");
    if (layer.parentId !== undefined) {
      throw new Error("addPaintLayer does not support parentId with the current bindings.");
    }

    return this._inner.add_paint_layer(layer.name, layer.width, layer.height);
  }

  addFilterLayer(options) {
    const layer = requireObject(options, "addFilterLayer");
    if (layer.parentId !== undefined) {
      return this._inner.add_filter_to_group(
        normalizeLayerId(layer.parentId, "addFilterLayer.parentId"),
        layer.name,
      );
    }

    return this._inner.add_filter_layer(layer.name);
  }

  addGroupLayer(options) {
    const layer = requireObject(options, "addGroupLayer");
    if (layer.parentId !== undefined) {
      return this._inner.add_group_to_group(
        normalizeLayerId(layer.parentId, "addGroupLayer.parentId"),
        layer.name,
      );
    }

    return this._inner.add_group_layer(layer.name);
  }

  addSolidColorLayer(options) {
    const layer = requireObject(options, "addSolidColorLayer");
    if (layer.parentId !== undefined) {
      throw new Error("addSolidColorLayer does not support parentId with the current bindings.");
    }

    const color = normalizeRgbaColor(layer.color, "addSolidColorLayer.color");
    return this._inner.add_solid_color_layer(layer.name, color[0], color[1], color[2], color[3]);
  }

  addGradientLayer(options) {
    const layer = requireObject(options, "addGradientLayer");
    if (layer.parentId !== undefined) {
      throw new Error("addGradientLayer does not support parentId with the current bindings.");
    }

    const { colors, positions } = normalizeGradientStops(layer.stops);
    const direction = normalizeGradientDirection(layer.direction);
    return this._inner.add_gradient_layer(layer.name, colors, positions, direction);
  }

  addShapeLayer(options) {
    const layer = normalizeShapeLayerOptions(options);

    if (layer.parentId !== undefined) {
      return this._inner.add_shape_to_group(
        normalizeLayerId(layer.parentId, "addShapeLayer.parentId"),
        layer.name,
        layer.type,
        layer.width,
        layer.height,
        layer.radius,
        layer.fill,
        layer.strokeColor,
        layer.strokeWidth,
        layer.points,
        layer.x,
        layer.y,
      );
    }

    return this._inner.add_shape_layer(
      layer.name,
      layer.type,
      layer.width,
      layer.height,
      layer.radius,
      layer.fill,
      layer.strokeColor,
      layer.strokeWidth,
      layer.points,
      layer.x,
      layer.y,
    );
  }

  addTextLayer(options) {
    const layer = normalizeTextLayerOptions(options);
    const textId =
      layer.parentId !== undefined
        ? this._inner.add_text_to_group(
            normalizeLayerId(layer.parentId, "addTextLayer.parentId"),
            layer.name,
            layer.text,
            layer.fontSize,
            layer.lineHeight,
            layer.letterSpacing,
            layer.color,
            layer.x,
            layer.y,
          )
        : this._inner.add_text_layer(
            layer.name,
            layer.text,
            layer.fontSize,
            layer.lineHeight,
            layer.letterSpacing,
            layer.color,
            layer.x,
            layer.y,
          );

    if (
      layer.fontFamily !== "sans-serif" ||
      layer.fontWeight !== 400 ||
      layer.fontStyle !== "normal" ||
      layer.align !== "left" ||
      layer.wrap !== "none" ||
      layer.boxWidth !== null
    ) {
      this._inner.update_layer(textId, {
        textConfig: {
          align: layer.align,
          boxWidth: layer.boxWidth,
          fontFamily: layer.fontFamily,
          fontStyle: layer.fontStyle,
          fontWeight: layer.fontWeight,
          wrap: layer.wrap,
        },
      });
    }

    return textId;
  }

  addSvgLayer(options) {
    if (!svgBackendActive()) {
      ensureBackendReadySync(textBackendActive() ? "textSvg" : "svg");
    }
    const layer = normalizeSvgLayerOptions(options);

    if (layer.parentId !== undefined) {
      return this._inner.add_svg_to_group(
        normalizeLayerId(layer.parentId, "addSvgLayer.parentId"),
        layer.name,
        layer.svg,
        layer.width,
        layer.height,
        layer.x,
        layer.y,
      );
    }

    return this._inner.add_svg_layer(
      layer.name,
      layer.svg,
      layer.width,
      layer.height,
      layer.x,
      layer.y,
    );
  }

  addPngLayer(options) {
    const layer = requireObject(options, "addPngLayer");
    if (layer.parentId !== undefined) {
      throw new Error("addPngLayer does not support parentId with the current bindings.");
    }

    const position = normalizePositionArg(layer.x, layer.y, "addPngLayer");
    return this._inner.add_png_layer(
      layer.name,
      normalizeByteInput(layer.png, "addPngLayer.png"),
      position.x,
      position.y,
    );
  }

  importImage(options) {
    const layer = requireObject(options, "importImage");
    const position = normalizePositionArg(layer.x, layer.y, "importImage");
    return this._inner.import_auto(
      layer.name,
      normalizeByteInput(layer.bytes, "importImage.bytes"),
      position.x,
      position.y,
    );
  }

  importJpeg(options) {
    const layer = requireObject(options, "importJpeg");
    const position = normalizePositionArg(layer.x, layer.y, "importJpeg");
    return this._inner.import_jpeg(
      layer.name,
      normalizeByteInput(layer.bytes, "importJpeg.bytes"),
      position.x,
      position.y,
    );
  }

  importWebp(options) {
    const layer = requireObject(options, "importWebp");
    const position = normalizePositionArg(layer.x, layer.y, "importWebp");
    return this._inner.import_webp(
      layer.name,
      normalizeByteInput(layer.bytes, "importWebp.bytes"),
      position.x,
      position.y,
    );
  }

  importGifFrames(options) {
    const input = requireObject(options, "importGifFrames");
    return this._inner.import_gif_frames(normalizeByteInput(input.bytes, "importGifFrames.bytes"));
  }

  importPsd(options) {
    const input = requireObject(options, "importPsd");
    return this._inner.import_psd(normalizeByteInput(input.bytes, "importPsd.bytes"));
  }

  setLayerOpacity(id, opacity) {
    return this._inner.set_opacity(normalizeLayerId(id), normalizeFiniteNumber(opacity, "opacity"));
  }

  setLayerVisibility(id, visible) {
    return this._inner.set_visible(normalizeLayerId(id), Boolean(visible));
  }

  setLayerPosition(id, xOrOptions, y) {
    const position = normalizePositionArg(xOrOptions, y, "setLayerPosition");
    return this._inner.set_position(normalizeLayerId(id), position.x, position.y);
  }

  setLayerBlendMode(id, blendMode) {
    return this._inner.set_blend_mode(normalizeLayerId(id), blendMode);
  }

  setLayerMask(id, options) {
    const mask = requireObject(options, "setLayerMask");
    this._inner.set_layer_mask(
      normalizeLayerId(id),
      normalizeByteInput(mask.rgba, "setLayerMask.rgba"),
      normalizeInteger(mask.width, "setLayerMask.width"),
      normalizeInteger(mask.height, "setLayerMask.height"),
    );

    if (mask.inverted !== undefined) {
      this._inner.set_mask_inverted(normalizeLayerId(id), Boolean(mask.inverted));
    }
  }

  bucketFillLayer(id, options) {
    const fill = normalizeBucketFillOptions(options);
    return this._inner.bucket_fill_layer(
      normalizeLayerId(id),
      fill.x,
      fill.y,
      fill.color[0],
      fill.color[1],
      fill.color[2],
      fill.color[3],
      fill.contiguous,
      fill.tolerance,
    );
  }

  paintStrokeLayer(id, options) {
    const stroke = normalizeBrushStrokeOptions(options);
    return this._inner.paint_stroke_layer(
      normalizeLayerId(id),
      stroke.points,
      stroke.color[0],
      stroke.color[1],
      stroke.color[2],
      stroke.color[3],
      stroke.size,
      stroke.opacity,
      stroke.flow,
      stroke.hardness,
      stroke.spacing,
      stroke.smoothing,
      stroke.pressureSize,
      stroke.pressureOpacity,
      stroke.tool === "erase",
    );
  }

  clearLayerMask(id) {
    return this._inner.remove_layer_mask(normalizeLayerId(id));
  }

  setLayerMaskInverted(id, inverted) {
    return this._inner.set_mask_inverted(normalizeLayerId(id), Boolean(inverted));
  }

  setLayerClipToBelow(id, clipToBelow) {
    return this._inner.set_clip_to_below(normalizeLayerId(id), Boolean(clipToBelow));
  }

  setLayerFlip(id, flipXOrOptions, flipY) {
    const flip = normalizeFlipArg(flipXOrOptions, flipY);
    return this._inner.set_flip(normalizeLayerId(id), flip.flipX, flip.flipY);
  }

  setLayerRotation(id, rotation) {
    return this._inner.set_rotation(
      normalizeLayerId(id),
      normalizeFiniteNumber(rotation, "rotation"),
    );
  }

  setLayerAnchor(id, anchor) {
    return this._inner.set_anchor(normalizeLayerId(id), normalizeAnchor(anchor));
  }

  setFilterLayerConfig(id, config) {
    const options = requireObject(config, "setFilterLayerConfig");
    return this._inner.set_filter_config(
      normalizeLayerId(id),
      normalizeFiniteNumber(options.hueDeg ?? options.hue ?? 0, "setFilterLayerConfig.hueDeg"),
      normalizeFiniteNumber(options.saturation ?? 0, "setFilterLayerConfig.saturation"),
      normalizeFiniteNumber(options.lightness ?? 0, "setFilterLayerConfig.lightness"),
      normalizeFiniteNumber(options.alpha ?? 0, "setFilterLayerConfig.alpha"),
      normalizeFiniteNumber(options.brightness ?? 0, "setFilterLayerConfig.brightness"),
      normalizeFiniteNumber(options.contrast ?? 0, "setFilterLayerConfig.contrast"),
      normalizeFiniteNumber(options.temperature ?? 0, "setFilterLayerConfig.temperature"),
      normalizeFiniteNumber(options.tint ?? 0, "setFilterLayerConfig.tint"),
      normalizeFiniteNumber(options.sharpen ?? 0, "setFilterLayerConfig.sharpen"),
    );
  }

  updateLayer(id, patch) {
    return this._inner.update_layer(normalizeLayerId(id), normalizeLayerUpdatePatch(patch));
  }

  getLayer(id) {
    return this._inner.get_layer(normalizeLayerId(id)) ?? null;
  }

  listLayers(options) {
    const normalized = normalizeListLayersOptions(options);
    return this._inner.list_layers(normalized.parentId, normalized.recursive);
  }

  removeLayer(id) {
    return this._inner.remove_layer(normalizeLayerId(id));
  }

  moveLayer(id, target) {
    const normalized = normalizeMoveLayerTarget(target);
    return this._inner.move_layer(normalizeLayerId(id), normalized.parentId, normalized.index);
  }

  resizeCanvas(widthOrOptions, height) {
    const size = normalizeSizeArg(widthOrOptions, height, "resizeCanvas");
    return this._inner.resize_canvas(size.width, size.height);
  }

  flattenGroup(groupId) {
    return this._inner.flatten_group(normalizeLayerId(groupId, "groupId"));
  }

  removeFromGroup(groupId, childId) {
    return this._inner.remove_from_group(
      normalizeLayerId(groupId, "groupId"),
      normalizeLayerId(childId, "childId"),
    );
  }

  renderRgba() {
    return this._inner.render();
  }

  exportPng() {
    return this._inner.export_png();
  }

  exportJpeg(qualityOrOptions) {
    return this._inner.export_jpeg(normalizeExportJpegArg(qualityOrOptions));
  }

  exportWebp() {
    return this._inner.export_webp();
  }

  serialize() {
    return this._inner.serialize();
  }

  getLayerRgba(id) {
    return this._inner.get_layer_rgba(normalizeLayerId(id));
  }

  rasterizeSvgLayer(id) {
    if (!svgBackendActive()) {
      ensureBackendReadySync(textBackendActive() ? "textSvg" : "svg");
    }
    return this._inner.rasterize_svg_layer(normalizeLayerId(id));
  }

  layerCount() {
    return this._inner.layer_count();
  }

  resizeLayerNearest(id, widthOrOptions, height) {
    const size = normalizeSizeArg(widthOrOptions, height, "resizeLayerNearest");
    return this._inner.resize_layer_nearest(normalizeLayerId(id), size.width, size.height);
  }

  resizeLayerBilinear(id, widthOrOptions, height) {
    const size = normalizeSizeArg(widthOrOptions, height, "resizeLayerBilinear");
    return this._inner.resize_layer_bilinear(normalizeLayerId(id), size.width, size.height);
  }

  resizeLayerLanczos3(id, widthOrOptions, height) {
    const size = normalizeSizeArg(widthOrOptions, height, "resizeLayerLanczos3");
    return this._inner.resize_layer_lanczos3(normalizeLayerId(id), size.width, size.height);
  }

  cropLayer(id, xOrOptions, y, width, height) {
    const crop =
      typeof xOrOptions === "object" && xOrOptions !== null
        ? {
            height: normalizeInteger(xOrOptions.height, "cropLayer.height"),
            width: normalizeInteger(xOrOptions.width, "cropLayer.width"),
            x: normalizeInteger(xOrOptions.x, "cropLayer.x"),
            y: normalizeInteger(xOrOptions.y, "cropLayer.y"),
          }
        : {
            height: normalizeInteger(height, "cropLayer.height"),
            width: normalizeInteger(width, "cropLayer.width"),
            x: normalizeInteger(xOrOptions, "cropLayer.x"),
            y: normalizeInteger(y, "cropLayer.y"),
          };

    return this._inner.crop_layer(normalizeLayerId(id), crop.x, crop.y, crop.width, crop.height);
  }

  trimLayerAlpha(id) {
    return this._inner.trim_layer_alpha(normalizeLayerId(id));
  }

  rotateLayer(id, angleDeg) {
    return this._inner.rotate_layer(
      normalizeLayerId(id),
      normalizeFiniteNumber(angleDeg, "angleDeg"),
    );
  }

  boxBlurLayer(id, radiusOrOptions) {
    const radius =
      typeof radiusOrOptions === "object" && radiusOrOptions !== null
        ? radiusOrOptions.radius
        : radiusOrOptions;
    return this._inner.box_blur_layer(
      normalizeLayerId(id),
      normalizeInteger(radius, "boxBlurLayer.radius"),
    );
  }

  gaussianBlurLayer(id, radiusOrOptions) {
    const radius =
      typeof radiusOrOptions === "object" && radiusOrOptions !== null
        ? radiusOrOptions.radius
        : radiusOrOptions;
    return this._inner.gaussian_blur_layer(
      normalizeLayerId(id),
      normalizeInteger(radius, "gaussianBlurLayer.radius"),
    );
  }

  sharpenLayer(id) {
    return this._inner.sharpen_layer(normalizeLayerId(id));
  }

  edgeDetectLayer(id) {
    return this._inner.edge_detect_layer(normalizeLayerId(id));
  }

  embossLayer(id) {
    return this._inner.emboss_layer(normalizeLayerId(id));
  }

  invertLayer(id) {
    return this._inner.invert_layer(normalizeLayerId(id));
  }

  posterizeLayer(id, levelsOrOptions) {
    const levels =
      typeof levelsOrOptions === "object" && levelsOrOptions !== null
        ? levelsOrOptions.levels
        : levelsOrOptions;
    return this._inner.posterize_layer(
      normalizeLayerId(id),
      normalizeInteger(levels, "posterizeLayer.levels"),
    );
  }

  thresholdLayer(id, thresholdOrOptions) {
    const threshold =
      typeof thresholdOrOptions === "object" && thresholdOrOptions !== null
        ? thresholdOrOptions.threshold
        : thresholdOrOptions;
    return this._inner.threshold_layer(
      normalizeLayerId(id),
      normalizeFiniteNumber(threshold, "thresholdLayer.threshold"),
    );
  }

  levelsLayer(id, shadowsOrOptions, midtones, highlights) {
    const levels =
      typeof shadowsOrOptions === "object" && shadowsOrOptions !== null
        ? {
            highlights: shadowsOrOptions.highlights,
            midtones: shadowsOrOptions.midtones,
            shadows: shadowsOrOptions.shadows,
          }
        : {
            highlights,
            midtones,
            shadows: shadowsOrOptions,
          };

    const shadowsValue = normalizeFiniteNumber(levels.shadows, "levelsLayer.shadows");
    const midtonesValue = normalizeFiniteNumber(levels.midtones, "levelsLayer.midtones");
    const highlightsValue = normalizeFiniteNumber(levels.highlights, "levelsLayer.highlights");

    const inBlack = Math.round(clamp01(shadowsValue) * 255);
    const inWhite = Math.max(inBlack + 1, Math.round(clamp01(highlightsValue) * 255));
    const gamma = Math.max(midtonesValue, Number.EPSILON);

    return this._inner.levels_layer(normalizeLayerId(id), inBlack, inWhite, gamma, 0, 255);
  }

  gradientMapLayer(id, stopsOrOptions) {
    const { colors, positions } = normalizeGradientStops(
      Array.isArray(stopsOrOptions) ? stopsOrOptions : stopsOrOptions.stops,
    );
    return this._inner.gradient_map_layer(normalizeLayerId(id), colors, positions);
  }

  pixelScaleLayer(id, factorOrOptions) {
    const factor =
      typeof factorOrOptions === "object" && factorOrOptions !== null
        ? factorOrOptions.factor
        : factorOrOptions;
    return this._inner.pixel_scale_layer(
      normalizeLayerId(id),
      normalizeInteger(factor, "pixelScaleLayer.factor"),
    );
  }

  extractPalette(id, maxColorsOrOptions) {
    const maxColors =
      typeof maxColorsOrOptions === "object" && maxColorsOrOptions !== null
        ? maxColorsOrOptions.maxColors
        : maxColorsOrOptions;
    return this._inner.extract_palette(
      normalizeLayerId(id),
      normalizeInteger(maxColors, "extractPalette.maxColors"),
    );
  }

  quantizeLayer(id, paletteOrOptions) {
    const palette =
      paletteOrOptions && typeof paletteOrOptions === "object" && "palette" in paletteOrOptions
        ? paletteOrOptions.palette
        : paletteOrOptions;
    return this._inner.quantize_layer(
      normalizeLayerId(id),
      normalizeByteInput(palette, "quantizeLayer.palette"),
    );
  }

  packSprites(layerIdsOrOptions, maxWidth, padding) {
    const options =
      typeof layerIdsOrOptions === "object" &&
      layerIdsOrOptions !== null &&
      !ArrayBuffer.isView(layerIdsOrOptions) &&
      !Array.isArray(layerIdsOrOptions)
        ? layerIdsOrOptions
        : { layerIds: layerIdsOrOptions, maxWidth, padding };

    return this._inner.pack_sprites(
      normalizeLayerIdArray(options.layerIds, "packSprites.layerIds"),
      normalizeInteger(options.maxWidth, "packSprites.maxWidth"),
      normalizeInteger(options.padding ?? 0, "packSprites.padding"),
    );
  }

  packSpritesJson(layerIdsOrOptions, maxWidth, padding) {
    const options =
      typeof layerIdsOrOptions === "object" &&
      layerIdsOrOptions !== null &&
      !ArrayBuffer.isView(layerIdsOrOptions) &&
      !Array.isArray(layerIdsOrOptions)
        ? layerIdsOrOptions
        : { layerIds: layerIdsOrOptions, maxWidth, padding };

    return this._inner.pack_sprites_json(
      normalizeLayerIdArray(options.layerIds, "packSpritesJson.layerIds"),
      normalizeInteger(options.maxWidth, "packSpritesJson.maxWidth"),
      normalizeInteger(options.padding ?? 0, "packSpritesJson.padding"),
    );
  }

  contactSheet(layerIdsOrOptions, columns, padding, background) {
    const options =
      typeof layerIdsOrOptions === "object" &&
      layerIdsOrOptions !== null &&
      !ArrayBuffer.isView(layerIdsOrOptions) &&
      !Array.isArray(layerIdsOrOptions)
        ? layerIdsOrOptions
        : {
            background,
            columns,
            layerIds: layerIdsOrOptions,
            padding,
          };
    const bg = normalizeRgbaColor(options.background ?? [0, 0, 0, 0], "contactSheet.background");

    return this._inner.contact_sheet(
      normalizeLayerIdArray(options.layerIds, "contactSheet.layerIds"),
      normalizeInteger(options.columns, "contactSheet.columns"),
      normalizeInteger(options.padding ?? 0, "contactSheet.padding"),
      bg[0],
      bg[1],
      bg[2],
      bg[3],
    );
  }
}

if (typeof Symbol.dispose === "symbol") {
  Composition.prototype[Symbol.dispose] = function (this: Composition): void {
    return this._inner.free();
  };
}

export { simdSupported };

export async function contrastRatio(a: string, b: string): Promise<number> {
  return withPreload(() => contrast_ratio(a, b));
}

export async function decodeImage(data: ByteInput): Promise<Uint8Array> {
  return withPreload(() => {
    const decoded = decode_image(normalizeByteInput(data, "decodeImage.data"));
    if (decoded.length < 8) {
      throw new Error("decodeImage returned malformed output");
    }
    return decoded.slice(8);
  });
}

export async function detectFormat(data: ByteInput): Promise<string> {
  return withPreload(() => detect_format(normalizeByteInput(data, "detectFormat.data")));
}

export async function dominantRgbFromRgba(
  data: ByteInput,
  widthOrOptions: number | GeometryOptions,
  height?: number,
): Promise<Uint8Array> {
  const size = normalizeImageGeometryArg(widthOrOptions, height, "dominantRgbFromRgba");
  return withPreload(() =>
    dominant_rgb_from_rgba(
      normalizeByteInput(data, "dominantRgbFromRgba.data"),
      size.width,
      size.height,
    ),
  );
}

export async function extractPaletteFromRgba(
  data: ByteInput,
  widthOrOptions: number | GeometryWithMaxColorsOptions,
  height?: number,
  maxColors?: number,
): Promise<Uint8Array> {
  const size = normalizeImageGeometryArg(widthOrOptions, height, "extractPaletteFromRgba");
  const paletteSize =
    typeof widthOrOptions === "object" && widthOrOptions !== null
      ? widthOrOptions.maxColors
      : maxColors;

  return withPreload(() =>
    extract_palette_from_rgba(
      normalizeByteInput(data, "extractPaletteFromRgba.data"),
      size.width,
      size.height,
      normalizeInteger(paletteSize, "extractPaletteFromRgba.maxColors"),
    ),
  );
}

export async function hexToRgb(hex: string): Promise<Uint8Array> {
  return withPreload(() => hex_to_rgb(hex));
}

export async function histogramRgba(
  data: ByteInput,
  widthOrOptions: number | GeometryOptions,
  height?: number,
): Promise<Uint32Array> {
  const size = normalizeImageGeometryArg(widthOrOptions, height, "histogramRgba");
  return withPreload(() =>
    histogram_rgba(normalizeByteInput(data, "histogramRgba.data"), size.width, size.height),
  );
}

export async function quantizeRgba(
  data: ByteInput,
  widthOrOptions: number | GeometryWithPaletteOptions,
  height?: number,
  palette?: ByteInput,
): Promise<Uint8Array> {
  const size = normalizeImageGeometryArg(widthOrOptions, height, "quantizeRgba");
  const paletteInput =
    typeof widthOrOptions === "object" && widthOrOptions !== null
      ? widthOrOptions.palette
      : palette;

  return withPreload(() =>
    quantize_rgba(
      normalizeByteInput(data, "quantizeRgba.data"),
      size.width,
      size.height,
      normalizeByteInput(paletteInput, "quantizeRgba.palette"),
    ),
  );
}

export async function relativeLuminance(hex: string): Promise<number> {
  return withPreload(() => relative_luminance(hex));
}

export async function rgbToHex(
  rOrOptions: number | RgbColor,
  g?: number,
  b?: number,
): Promise<string> {
  const rgb = normalizeRgbArgs(rOrOptions, g, b);
  return withPreload(() => rgb_to_hex(rgb.r, rgb.g, rgb.b));
}

export default preload;

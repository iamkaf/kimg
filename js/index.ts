import initRaw, {
  Composition as RawComposition,
  contrast_ratio,
  decode_image,
  detect_format,
  dominant_rgb_from_rgba,
  extract_palette_from_rgba,
  hex_to_rgb,
  histogram_rgba,
  quantize_rgba,
  relative_luminance,
  rgb_to_hex,
  simdSupported,
} from "./raw.js";
import type { InitInput, InitOutput } from "./raw.js";

export type ByteInput = ArrayBuffer | ArrayBufferView | ArrayLike<number>;
export type Anchor = "topLeft" | "top_left" | "center" | 0 | 1;
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

export type ShapeLayerType = "rectangle" | "roundedRect" | "ellipse" | "line" | "polygon";

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
  filterConfig?: FilterConfig;
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
  | "image"
  | "paint"
  | "filter"
  | "group"
  | "solidColor"
  | "gradient"
  | "shape"
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
  filterConfig?: FilterConfigSnapshot;
  childCount?: number;
  color?: number[];
  direction?: Exclude<GradientDirection, 0 | 1 | 2 | 3>;
  stopCount?: number;
  shapeType?: ShapeLayerType;
  radius?: number;
  pointCount?: number;
  fill?: number[];
  strokeColor?: number[];
  strokeWidth?: number;
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

function normalizePositiveInteger(value, fieldName) {
  const normalized = normalizeInteger(value, fieldName);
  if (normalized <= 0) {
    throw new RangeError(`${fieldName} must be greater than 0.`);
  }
  return normalized;
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
    case "roundedRect":
    case "ellipse":
    case "line":
    case "polygon":
      return shapeType;
    default:
      throw new TypeError(
        'type must be "rectangle", "roundedRect", "ellipse", "line", or "polygon".',
      );
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

  const radius =
    type === "roundedRect" ? normalizeInteger(layer.radius ?? 0, "addShapeLayer.radius") : 0;

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

  const filterConfig = object.filterConfig ?? object.filter;
  if (filterConfig !== undefined) {
    normalized.filterConfig = normalizeFilterConfigPatch(filterConfig, "updateLayer.filterConfig");
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

async function getDefaultInitInput(): Promise<Uint8Array | undefined> {
  if (!isNodeRuntime()) {
    return undefined;
  }

  // @ts-ignore Node built-in typing is only available in Node environments.
  const { readFile } = await import("node:fs/promises");
  const wasmName = simdSupported() ? "kimg_wasm_simd_bg.wasm" : "kimg_wasm_bg.wasm";
  return readFile(new URL(`./${wasmName}`, import.meta.url));
}

async function withPreload<T>(fn: () => T): Promise<T> {
  await preload();
  return fn();
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
    if (module_or_path !== undefined) {
      return initRaw(module_or_path);
    }

    const defaultInput = await getDefaultInitInput();
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
  readonly _inner!: RawComposition;

  static #fromInner(inner: RawComposition) {
    return new Composition(inner, PRIVATE_CONSTRUCTOR_TOKEN);
  }

  private constructor(inner: RawComposition, token: symbol) {
    if (token !== PRIVATE_CONSTRUCTOR_TOKEN) {
      throw new TypeError("Use await Composition.create(...) instead of new Composition(...).");
    }

    Object.defineProperty(this, "_inner", {
      configurable: false,
      enumerable: false,
      value: inner,
      writable: false,
    });
  }

  static async create(width: number, height: number): Promise<Composition>;
  static async create(options: CompositionOptions): Promise<Composition>;
  static async create(
    widthOrOptions: number | CompositionOptions,
    height?: number,
  ): Promise<Composition> {
    const size = normalizeCreateArgs(widthOrOptions, height);
    await preload();
    return Composition.#fromInner(new RawComposition(size.width, size.height));
  }

  static async deserialize(data: ByteInput): Promise<Composition> {
    await preload();
    return Composition.#fromInner(RawComposition.deserialize(normalizeByteInput(data, "data")));
  }

  get width(): number {
    return this._inner.width;
  }

  get height(): number {
    return this._inner.height;
  }

  free(): void {
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

    return this._inner.levels_layer(
      normalizeLayerId(id),
      normalizeFiniteNumber(levels.shadows, "levelsLayer.shadows"),
      normalizeFiniteNumber(levels.midtones, "levelsLayer.midtones"),
      normalizeFiniteNumber(levels.highlights, "levelsLayer.highlights"),
    );
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
  return withPreload(() => decode_image(normalizeByteInput(data, "decodeImage.data")));
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

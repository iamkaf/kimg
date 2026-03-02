export interface RawFilterConfig {
  hueDeg?: number;
  hue?: number;
  saturation?: number;
  lightness?: number;
  alpha?: number;
  brightness?: number;
  contrast?: number;
  temperature?: number;
  tint?: number;
  sharpen?: number;
}

export interface RawLayerPatch {
  name?: string;
  visible?: boolean;
  opacity?: number;
  x?: number;
  y?: number;
  blendMode?: string;
  maskInverted?: boolean;
  clipToBelow?: boolean;
  anchor?: "topLeft" | "top_left" | "center" | 0 | 1;
  flipX?: boolean;
  flipY?: boolean;
  rotation?: number;
  scaleX?: number;
  scaleY?: number;
  filterConfig?: RawFilterConfig;
}

export interface RawLayerSnapshot {
  id: number;
  parentId: number | null;
  index: number;
  depth: number;
  kind: "image" | "paint" | "filter" | "group" | "solidColor" | "gradient" | "shape" | "unknown";
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
  filterConfig?: {
    hueDeg: number;
    saturation: number;
    lightness: number;
    alpha: number;
    brightness: number;
    contrast: number;
    temperature: number;
    tint: number;
    sharpen: number;
  };
  childCount?: number;
  color?: number[];
  direction?: "horizontal" | "vertical" | "diagonalDown" | "diagonalUp";
  stopCount?: number;
  shapeType?: "rectangle" | "roundedRect" | "ellipse" | "line" | "polygon";
  radius?: number;
  pointCount?: number;
  fill?: number[];
  strokeColor?: number[];
  strokeWidth?: number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
export type SyncInitInput = BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly [key: string]: unknown;
}

interface RawCompositionMethods {
  free(): void;
  [Symbol.dispose](): void;
  add_image_layer(
    name: string,
    rgba_data: Uint8Array,
    img_width: number,
    img_height: number,
    x: number,
    y: number,
  ): number;
  add_paint_layer(name: string, width: number, height: number): number;
  add_filter_layer(name: string): number;
  add_group_layer(name: string): number;
  add_solid_color_layer(name: string, r: number, g: number, b: number, a: number): number;
  add_gradient_layer(
    name: string,
    stops_colors: Uint8Array,
    stops_positions: Float64Array,
    direction: number,
  ): number;
  add_shape_layer(
    name: string,
    shape_type: string,
    width: number,
    height: number,
    radius: number,
    fill: Uint8Array,
    stroke_color: Uint8Array,
    stroke_width: number,
    points_xy: Int32Array,
    x: number,
    y: number,
  ): number;
  set_opacity(id: number, opacity: number): void;
  set_visible(id: number, visible: boolean): void;
  set_position(id: number, x: number, y: number): void;
  set_blend_mode(id: number, mode: string): void;
  set_layer_mask(id: number, mask_data: Uint8Array, mask_width: number, mask_height: number): void;
  remove_layer_mask(id: number): void;
  set_mask_inverted(id: number, inverted: boolean): void;
  set_clip_to_below(id: number, clip: boolean): void;
  set_flip(id: number, flip_x: boolean, flip_y: boolean): void;
  set_rotation(id: number, degrees: number): void;
  set_anchor(id: number, anchor: number): void;
  set_filter_config(
    id: number,
    hue_deg: number,
    saturation: number,
    lightness: number,
    alpha: number,
    brightness: number,
    contrast: number,
    temperature: number,
    tint: number,
    sharpen: number,
  ): void;
  add_image_to_group(
    group_id: number,
    name: string,
    rgba_data: Uint8Array,
    img_width: number,
    img_height: number,
    x: number,
    y: number,
  ): number;
  add_filter_to_group(group_id: number, name: string): number;
  add_group_to_group(group_id: number, name: string): number;
  add_shape_to_group(
    group_id: number,
    name: string,
    shape_type: string,
    width: number,
    height: number,
    radius: number,
    fill: Uint8Array,
    stroke_color: Uint8Array,
    stroke_width: number,
    points_xy: Int32Array,
    x: number,
    y: number,
  ): number;
  remove_from_group(group_id: number, child_id: number): boolean;
  flatten_group(group_id: number): boolean;
  remove_layer(id: number): boolean;
  move_layer(id: number, parent_id: number, index: number): boolean;
  resize_canvas(width: number, height: number): void;
  get_layer(id: number): RawLayerSnapshot | undefined;
  list_layers(parent_id: number, recursive: boolean): RawLayerSnapshot[];
  update_layer(id: number, patch: RawLayerPatch): boolean;
  add_png_layer(name: string, png_bytes: Uint8Array, x: number, y: number): number;
  export_png(): Uint8Array;
  get_layer_rgba(id: number): Uint8Array;
  bucket_fill_layer(
    id: number,
    x: number,
    y: number,
    r: number,
    g: number,
    b: number,
    a: number,
    contiguous: boolean,
    tolerance: number,
  ): boolean;
  render(): Uint8Array;
  layer_count(): number;
  resize_layer_nearest(id: number, new_width: number, new_height: number): void;
  resize_layer_bilinear(id: number, new_width: number, new_height: number): void;
  resize_layer_lanczos3(id: number, new_width: number, new_height: number): void;
  crop_layer(id: number, x: number, y: number, width: number, height: number): void;
  trim_layer_alpha(id: number): void;
  rotate_layer(id: number, angle_deg: number): void;
  box_blur_layer(id: number, radius: number): void;
  gaussian_blur_layer(id: number, radius: number): void;
  sharpen_layer(id: number): void;
  edge_detect_layer(id: number): void;
  emboss_layer(id: number): void;
  invert_layer(id: number): void;
  posterize_layer(id: number, levels: number): void;
  threshold_layer(id: number, thresh: number): void;
  levels_layer(id: number, shadows: number, midtones: number, highlights: number): void;
  gradient_map_layer(id: number, stops_colors: Uint8Array, stops_positions: Float64Array): void;
  pack_sprites(layer_ids: Uint32Array, max_width: number, padding: number): Uint8Array;
  pack_sprites_json(layer_ids: Uint32Array, max_width: number, padding: number): string;
  contact_sheet(
    layer_ids: Uint32Array,
    columns: number,
    padding: number,
    bg_r: number,
    bg_g: number,
    bg_b: number,
    bg_a: number,
  ): Uint8Array;
  pixel_scale_layer(id: number, factor: number): void;
  extract_palette(id: number, max_colors: number): Uint8Array;
  quantize_layer(id: number, palette_colors: Uint8Array): void;
  import_jpeg(name: string, jpeg_bytes: Uint8Array, x: number, y: number): number;
  import_webp(name: string, webp_bytes: Uint8Array, x: number, y: number): number;
  import_gif_frames(gif_bytes: Uint8Array): Uint32Array;
  import_psd(psd_bytes: Uint8Array): Uint32Array;
  import_auto(name: string, bytes: Uint8Array, x: number, y: number): number;
  export_jpeg(quality: number): Uint8Array;
  export_webp(): Uint8Array;
  serialize(): Uint8Array;
  readonly width: number;
  readonly height: number;
}

interface GeneratedBindingsModule {
  Composition: {
    new (width: number, height: number): RawCompositionMethods;
    deserialize(data: Uint8Array): RawCompositionMethods;
  };
  contrast_ratio(a: string, b: string): number;
  decode_image(data: Uint8Array): Uint8Array;
  detect_format(data: Uint8Array): string;
  dominant_rgb_from_rgba(data: Uint8Array, w: number, h: number): Uint8Array;
  extract_palette_from_rgba(data: Uint8Array, w: number, h: number, max_colors: number): Uint8Array;
  hex_to_rgb(hex: string): Uint8Array;
  histogram_rgba(data: Uint8Array, w: number, h: number): Uint32Array;
  relative_luminance(hex: string): number;
  quantize_rgba(data: Uint8Array, w: number, h: number, palette: Uint8Array): Uint8Array;
  rgb_to_hex(r: number, g: number, b: number): string;
  initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;
  default(
    module_or_path?:
      | { module_or_path: InitInput | Promise<InitInput> }
      | InitInput
      | Promise<InitInput>,
  ): Promise<InitOutput>;
}

// @ts-ignore Generated by wasm-bindgen during build.
import * as baselineBindingsModule from "./kimg_wasm_bg.js";
// @ts-ignore Generated by wasm-bindgen during build.
import * as simdBindingsModule from "./kimg_wasm_simd.js";

const baselineBindings = baselineBindingsModule as GeneratedBindingsModule;
const simdBindings = simdBindingsModule as GeneratedBindingsModule;

const SIMD_DETECT_BYTES = new Uint8Array([
  0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15,
  253, 98, 11,
]);

const COMPOSITION_PROXY_HANDLER: ProxyHandler<Composition> = {
  get(target, prop, receiver) {
    if (Reflect.has(target, prop)) {
      return Reflect.get(target, prop, receiver);
    }

    const value = (target as Composition & { _inner: RawCompositionMethods })._inner[
      prop as keyof RawCompositionMethods
    ];
    return typeof value === "function"
      ? value.bind((target as Composition & { _inner: RawCompositionMethods })._inner)
      : value;
  },
  set(target, prop, value, receiver) {
    if (Reflect.has(target, prop)) {
      return Reflect.set(target, prop, value, receiver);
    }

    (target as Composition & { _inner: Record<PropertyKey, unknown> })._inner[prop] = value;
    return true;
  },
};

let activeBindings: GeneratedBindingsModule | null = null;
let cachedSimdSupport: boolean | undefined;

function requireBindings(): GeneratedBindingsModule {
  if (activeBindings === null) {
    throw new Error("kimg WASM is not initialized. Call init() or initSync() first.");
  }

  return activeBindings;
}

function normalizeAsyncInitInput(
  module_or_path:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): { module_or_path: InitInput | Promise<InitInput> } {
  if (
    typeof module_or_path === "object" &&
    module_or_path !== null &&
    "module_or_path" in module_or_path
  ) {
    return module_or_path;
  }

  return { module_or_path };
}

function normalizeSyncInitInput(module: { module: SyncInitInput } | SyncInitInput): {
  module: SyncInitInput;
} {
  if (typeof module === "object" && module !== null && "module" in module) {
    return module;
  }

  return { module };
}

export function simdSupported(): boolean {
  if (cachedSimdSupport !== undefined) {
    return cachedSimdSupport;
  }

  try {
    cachedSimdSupport =
      typeof WebAssembly === "object" &&
      typeof WebAssembly.validate === "function" &&
      WebAssembly.validate(SIMD_DETECT_BYTES);
  } catch {
    cachedSimdSupport = false;
  }

  return cachedSimdSupport;
}

export interface Composition extends RawCompositionMethods {}

export class Composition {
  readonly _inner!: RawCompositionMethods;

  static #fromInner(inner: RawCompositionMethods): Composition {
    const composition = Object.create(Composition.prototype) as Composition;
    Object.defineProperty(composition, "_inner", {
      configurable: false,
      enumerable: false,
      value: inner,
      writable: false,
    });
    return new Proxy(composition, COMPOSITION_PROXY_HANDLER);
  }

  constructor(width: number, height: number) {
    const bindings = requireBindings();
    return Composition.#fromInner(new bindings.Composition(width, height));
  }

  static deserialize(data: Uint8Array): Composition {
    return Composition.#fromInner(requireBindings().Composition.deserialize(data));
  }

  free(): void {
    return this._inner.free();
  }
}

if (typeof Symbol.dispose === "symbol") {
  Composition.prototype[Symbol.dispose] = function (): void {
    if (typeof this._inner[Symbol.dispose] === "function") {
      return this._inner[Symbol.dispose]();
    }

    return this._inner.free();
  };
}

export function contrast_ratio(a: string, b: string): number {
  return requireBindings().contrast_ratio(a, b);
}

export function decode_image(data: Uint8Array): Uint8Array {
  return requireBindings().decode_image(data);
}

export function detect_format(data: Uint8Array): string {
  return requireBindings().detect_format(data);
}

export function dominant_rgb_from_rgba(data: Uint8Array, w: number, h: number): Uint8Array {
  return requireBindings().dominant_rgb_from_rgba(data, w, h);
}

export function extract_palette_from_rgba(
  data: Uint8Array,
  w: number,
  h: number,
  max_colors: number,
): Uint8Array {
  return requireBindings().extract_palette_from_rgba(data, w, h, max_colors);
}

export function hex_to_rgb(hex: string): Uint8Array {
  return requireBindings().hex_to_rgb(hex);
}

export function histogram_rgba(data: Uint8Array, w: number, h: number): Uint32Array {
  return requireBindings().histogram_rgba(data, w, h);
}

export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput {
  const input = normalizeSyncInitInput(module);

  if (simdSupported()) {
    try {
      const output = simdBindings.initSync(input);
      activeBindings = simdBindings;
      return output;
    } catch {
      // Fall through to the baseline artifact if the explicit module is not SIMD.
    }
  }

  const output = baselineBindings.initSync(input);
  activeBindings = baselineBindings;
  return output;
}

export function relative_luminance(hex: string): number {
  return requireBindings().relative_luminance(hex);
}

export function quantize_rgba(
  data: Uint8Array,
  w: number,
  h: number,
  palette: Uint8Array,
): Uint8Array {
  return requireBindings().quantize_rgba(data, w, h, palette);
}

export function rgb_to_hex(r: number, g: number, b: number): string {
  return requireBindings().rgb_to_hex(r, g, b);
}

export default async function init(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  if (module_or_path !== undefined) {
    const input = normalizeAsyncInitInput(module_or_path);

    if (simdSupported()) {
      try {
        const output = await simdBindings.default(input);
        activeBindings = simdBindings;
        return output;
      } catch {
        // Fall through to the baseline artifact if the explicit module is not SIMD.
      }
    }

    const output = await baselineBindings.default(input);
    activeBindings = baselineBindings;
    return output;
  }

  if (simdSupported()) {
    try {
      const output = await simdBindings.default();
      activeBindings = simdBindings;
      return output;
    } catch {
      // Fall through to the baseline artifact if the SIMD module is unavailable.
    }
  }

  const output = await baselineBindings.default();
  activeBindings = baselineBindings;
  return output;
}

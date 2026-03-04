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
  kind: "raster" | "filter" | "group" | "fill" | "shape" | "text" | "svg" | "unknown";
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
  fillType?: "solid" | "gradient";
  color?: number[];
  direction?: "horizontal" | "vertical" | "diagonalDown" | "diagonalUp";
  stopCount?: number;
  shapeType?: "rectangle" | "ellipse" | "line" | "polygon";
  radius?: number;
  pointCount?: number;
  fill?: number[];
  strokeColor?: number[];
  strokeWidth?: number;
}

export type InitInput = import("./kimg_wasm_bg.js").InitInput;
export type InitOutput = import("./kimg_wasm_bg.js").InitOutput;
export type SyncInitInput = import("./kimg_wasm_bg.js").SyncInitInput;

type GeneratedBindingsModule = typeof import("./kimg_wasm_bg.js");
type GeneratedRawCompositionMethods = InstanceType<GeneratedBindingsModule["Composition"]>;
export type RawCompositionInstance = GeneratedRawCompositionMethods;

type RawCompositionMethods = GeneratedRawCompositionMethods & {
  get_layer(id: number): RawLayerSnapshot | undefined;
  list_layers(parent_id: number, recursive: boolean): RawLayerSnapshot[];
  update_layer(id: number, patch: RawLayerPatch): boolean;
};

type VariantModuleKey =
  | "base"
  | "baseSimd"
  | "svg"
  | "svgSimd"
  | "text"
  | "textSimd"
  | "textSvg"
  | "textSvgSimd";

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
let activeTextBackend = false;
let activeSvgBackend = false;
let cachedSimdSupport: boolean | undefined;
const loadedBindingsModules = new Map<VariantModuleKey, GeneratedBindingsModule>();
const loadingBindingsModules = new Map<VariantModuleKey, Promise<GeneratedBindingsModule>>();

function isNodeRuntimeLike(): boolean {
  const maybeProcess = (globalThis as typeof globalThis & {
    process?: { versions?: { node?: string } };
  }).process;
  return (
    typeof maybeProcess === "object" &&
    maybeProcess !== null &&
    typeof maybeProcess.versions === "object" &&
    maybeProcess.versions !== null &&
    typeof maybeProcess.versions.node === "string"
  );
}

function moduleKeyForVariant(
  textBackend: boolean,
  svgBackend: boolean,
  useSimd: boolean,
): VariantModuleKey {
  if (textBackend && svgBackend) {
    return useSimd ? "textSvgSimd" : "textSvg";
  }

  if (textBackend) {
    return useSimd ? "textSimd" : "text";
  }

  if (svgBackend) {
    return useSimd ? "svgSimd" : "svg";
  }

  return useSimd ? "baseSimd" : "base";
}

async function importBindingsModule(key: VariantModuleKey): Promise<GeneratedBindingsModule> {
  switch (key) {
    case "base":
      return import("./kimg_wasm_bg.js");
    case "baseSimd":
      return import("./kimg_wasm_simd.js");
    case "svg":
      return import("./kimg_wasm_svg_bg.js");
    case "svgSimd":
      return import("./kimg_wasm_svg_simd.js");
    case "text":
      return import("./kimg_wasm_text_bg.js");
    case "textSimd":
      return import("./kimg_wasm_text_simd.js");
    case "textSvg":
      return import("./kimg_wasm_text_svg_bg.js");
    case "textSvgSimd":
      return import("./kimg_wasm_text_svg_simd.js");
  }
}

async function loadBindingsModule(key: VariantModuleKey): Promise<GeneratedBindingsModule> {
  const loaded = loadedBindingsModules.get(key);
  if (loaded !== undefined) {
    return loaded;
  }

  const loading = loadingBindingsModules.get(key);
  if (loading !== undefined) {
    return loading;
  }

  const promise = importBindingsModule(key).then((module) => {
    loadedBindingsModules.set(key, module);
    loadingBindingsModules.delete(key);
    return module;
  });
  loadingBindingsModules.set(key, promise);
  return promise;
}

function requireLoadedBindingsModule(key: VariantModuleKey): GeneratedBindingsModule {
  const loaded = loadedBindingsModules.get(key);
  if (loaded !== undefined) {
    return loaded;
  }

  throw new Error(
    `The ${key} WASM bindings are not loaded. Use the async init/preload path before calling the synchronous initializer for this backend.`,
  );
}

const nodeBindingsBootstrapPromise = isNodeRuntimeLike()
  ? Promise.all([
      loadBindingsModule("base"),
      loadBindingsModule("baseSimd"),
      loadBindingsModule("svg"),
      loadBindingsModule("svgSimd"),
      loadBindingsModule("text"),
      loadBindingsModule("textSimd"),
      loadBindingsModule("textSvg"),
      loadBindingsModule("textSvgSimd"),
    ])
  : null;

if (nodeBindingsBootstrapPromise !== null) {
  await nodeBindingsBootstrapPromise;
}

function requireBindings(): GeneratedBindingsModule {
  if (activeBindings === null) {
    throw new Error("kimg WASM is not initialized. Call init() or initSync() first.");
  }

  return activeBindings;
}

function setActiveBindings(
  bindings: GeneratedBindingsModule,
  textBackend: boolean,
  svgBackend: boolean,
): void {
  activeBindings = bindings;
  activeTextBackend = textBackend;
  activeSvgBackend = svgBackend;
}

function preferredBindingsModuleKey(textBackend: boolean, svgBackend: boolean): VariantModuleKey {
  return moduleKeyForVariant(textBackend, svgBackend, simdSupported());
}

function fallbackBindingsModuleKey(textBackend: boolean, svgBackend: boolean): VariantModuleKey {
  return moduleKeyForVariant(textBackend, svgBackend, false);
}

async function loadPreferredBindingsModule(
  textBackend: boolean,
  svgBackend: boolean,
): Promise<GeneratedBindingsModule> {
  return loadBindingsModule(preferredBindingsModuleKey(textBackend, svgBackend));
}

async function loadFallbackBindingsModule(
  textBackend: boolean,
  svgBackend: boolean,
): Promise<GeneratedBindingsModule> {
  return loadBindingsModule(fallbackBindingsModuleKey(textBackend, svgBackend));
}

function requirePreferredBindingsModule(
  textBackend: boolean,
  svgBackend: boolean,
): GeneratedBindingsModule {
  return requireLoadedBindingsModule(preferredBindingsModuleKey(textBackend, svgBackend));
}

function requireFallbackBindingsModule(
  textBackend: boolean,
  svgBackend: boolean,
): GeneratedBindingsModule {
  return requireLoadedBindingsModule(fallbackBindingsModuleKey(textBackend, svgBackend));
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

export function document_has_svg_layers(data: Uint8Array): boolean {
  return requireBindings().document_has_svg_layers(data);
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
  return initVariantSync(false, false, module);
}

export function initSvgSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput {
  return initVariantSync(false, true, module);
}

export function initTextSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput {
  return initVariantSync(true, false, module);
}

export function initTextSvgSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput {
  return initVariantSync(true, true, module);
}

function initVariantSync(
  textBackend: boolean,
  svgBackend: boolean,
  module: { module: SyncInitInput } | SyncInitInput,
): InitOutput {
  const input = normalizeSyncInitInput(module);

  if (simdSupported()) {
    try {
      const preferredBindings = requirePreferredBindingsModule(textBackend, svgBackend);
      const output = preferredBindings.initSync(input);
      setActiveBindings(preferredBindings, textBackend, svgBackend);
      return output;
    } catch {
      // Fall through to the baseline artifact if the explicit module is not SIMD.
    }
  }

  const fallbackBindings = requireFallbackBindingsModule(textBackend, svgBackend);
  const output = fallbackBindings.initSync(input);
  setActiveBindings(fallbackBindings, textBackend, svgBackend);
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
  return initVariant(false, false, module_or_path);
}

export async function initSvg(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  return initVariant(false, true, module_or_path);
}

export async function initText(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  return initVariant(true, false, module_or_path);
}

export async function initTextSvg(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  return initVariant(true, true, module_or_path);
}

export async function preloadBindings(): Promise<void> {
  await loadBindingsModule(moduleKeyForVariant(false, false, simdSupported()));
}

export async function preloadSvgBindings(): Promise<void> {
  await loadBindingsModule(moduleKeyForVariant(false, true, simdSupported()));
}

export async function preloadTextBindings(): Promise<void> {
  await loadBindingsModule(moduleKeyForVariant(true, false, simdSupported()));
}

export async function preloadTextSvgBindings(): Promise<void> {
  await loadBindingsModule(moduleKeyForVariant(true, true, simdSupported()));
}

async function initVariant(
  textBackend: boolean,
  svgBackend: boolean,
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput> {
  if (module_or_path !== undefined) {
    const input = normalizeAsyncInitInput(module_or_path);

    if (simdSupported()) {
      try {
        const preferredBindings = await loadPreferredBindingsModule(textBackend, svgBackend);
        const output = await preferredBindings.default(input);
        setActiveBindings(preferredBindings, textBackend, svgBackend);
        return output;
      } catch {
        // Fall through to the baseline artifact if the explicit module is not SIMD.
      }
    }

    const fallbackBindings = await loadFallbackBindingsModule(textBackend, svgBackend);
    const output = await fallbackBindings.default(input);
    setActiveBindings(fallbackBindings, textBackend, svgBackend);
    return output;
  }

  if (simdSupported()) {
    try {
      const preferredBindings = await loadPreferredBindingsModule(textBackend, svgBackend);
      const output = await preferredBindings.default();
      setActiveBindings(preferredBindings, textBackend, svgBackend);
      return output;
    } catch {
      // Fall through to the baseline artifact if the SIMD module is unavailable.
    }
  }

  const fallbackBindings = await loadFallbackBindingsModule(textBackend, svgBackend);
  const output = await fallbackBindings.default();
  setActiveBindings(fallbackBindings, textBackend, svgBackend);
  return output;
}

export function textBackendActive(): boolean {
  return activeTextBackend;
}

export function svgBackendActive(): boolean {
  return activeSvgBackend;
}

export function register_font(bytes: Uint8Array): number {
  return requireBindings().register_font(bytes);
}

export function clear_registered_fonts(): void {
  return requireBindings().clear_registered_fonts();
}

export function registered_font_count(): number {
  return requireBindings().registered_font_count();
}

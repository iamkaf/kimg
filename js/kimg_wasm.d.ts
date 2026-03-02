/* tslint:disable */
/* eslint-disable */

/**
 * WASM-exposed Document for image compositing.
 */
export class Document {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add an HSL filter layer. Returns the layer ID.
     */
    add_filter_layer(name: string): number;
    /**
     * Add a filter layer as a child of a group. Returns the child layer ID.
     */
    add_filter_to_group(group_id: number, name: string): number;
    /**
     * Add a gradient layer. `stops` is a flat array of [pos_f32, r, g, b, a, ...].
     * Each stop is 5 values: position (0-1 as f32 bits in a u8 pair? No — we use f64).
     * Actually: stops_data is [r, g, b, a, r, g, b, a, ...] and stops_positions is [f64, f64, ...].
     * Direction: 0=horizontal, 1=vertical, 2=diagonal-down, 3=diagonal-up.
     */
    add_gradient_layer(name: string, stops_colors: Uint8Array, stops_positions: Float64Array, direction: number): number;
    /**
     * Add a group layer. Returns the layer ID.
     */
    add_group_layer(name: string): number;
    /**
     * Add a nested group as a child of a group. Returns the child layer ID.
     */
    add_group_to_group(group_id: number, name: string): number;
    /**
     * Add an image layer from raw RGBA data. Returns the layer ID.
     */
    add_image_layer(name: string, rgba_data: Uint8Array, img_width: number, img_height: number, x: number, y: number): number;
    /**
     * Add an image layer as a child of a group. Returns the child layer ID.
     */
    add_image_to_group(group_id: number, name: string, rgba_data: Uint8Array, img_width: number, img_height: number, x: number, y: number): number;
    /**
     * Add a paint layer (empty editable RGBA buffer). Returns the layer ID.
     */
    add_paint_layer(name: string, width: number, height: number): number;
    /**
     * Decode a PNG and add it as a top-level image layer. Returns the layer ID.
     */
    add_png_layer(name: string, png_bytes: Uint8Array, x: number, y: number): number;
    /**
     * Add a solid color fill layer. Returns the layer ID.
     */
    add_solid_color_layer(name: string, r: number, g: number, b: number, a: number): number;
    /**
     * Render the document and encode as PNG.
     */
    export_png(): Uint8Array;
    /**
     * Flatten a group layer into a single image layer. Returns true on success.
     */
    flatten_group(group_id: number): boolean;
    /**
     * Get a layer's raw RGBA pixel buffer. Returns empty vec if not an image/paint layer.
     */
    get_layer_rgba(id: number): Uint8Array;
    /**
     * Get the number of top-level layers.
     */
    layer_count(): number;
    /**
     * Create a new document with the given canvas dimensions.
     */
    constructor(width: number, height: number);
    /**
     * Remove a child from a group. Returns true if found and removed.
     */
    remove_from_group(group_id: number, child_id: number): boolean;
    /**
     * Remove a layer's mask.
     */
    remove_layer_mask(id: number): void;
    /**
     * Render the document and return the flat RGBA buffer.
     */
    render(): Uint8Array;
    /**
     * Set anchor on an image layer. 0 = TopLeft, 1 = Center.
     */
    set_anchor(id: number, anchor: number): void;
    /**
     * Set blend mode by name (e.g. "multiply", "screen", "color-dodge").
     */
    set_blend_mode(id: number, mode: string): void;
    /**
     * Set whether a layer clips to the layer below it.
     */
    set_clip_to_below(id: number, clip: boolean): void;
    /**
     * Bulk-set all 9 filter config fields on a filter layer.
     */
    set_filter_config(id: number, hue_deg: number, saturation: number, lightness: number, alpha: number, brightness: number, contrast: number, temperature: number, tint: number, sharpen: number): void;
    /**
     * Set flip on an image layer.
     */
    set_flip(id: number, flip_x: boolean, flip_y: boolean): void;
    /**
     * Set a grayscale layer mask from RGBA data. Uses the red channel as mask value.
     */
    set_layer_mask(id: number, mask_data: Uint8Array, mask_width: number, mask_height: number): void;
    /**
     * Set layer opacity (0.0 to 1.0).
     */
    set_opacity(id: number, opacity: number): void;
    /**
     * Set layer position.
     */
    set_position(id: number, x: number, y: number): void;
    /**
     * Set rotation on an image layer (snaps to nearest 90 degrees).
     */
    set_rotation(id: number, degrees: number): void;
    /**
     * Set layer visibility.
     */
    set_visible(id: number, visible: boolean): void;
    readonly height: number;
    readonly width: number;
}

/**
 * WCAG 2.x contrast ratio. Returns -1.0 on failure.
 */
export function contrast_ratio(a: string, b: string): number;

/**
 * Decode an image via auto-detected format and return flat RGBA bytes.
 */
export function decode_image(data: Uint8Array): Uint8Array;

/**
 * Detect the image format from magic bytes.
 */
export function detect_format(data: Uint8Array): string;

/**
 * Dominant color from RGBA pixel data. Returns 3 bytes [r, g, b] or empty on failure.
 */
export function dominant_rgb_from_rgba(data: Uint8Array, w: number, h: number): Uint8Array;

/**
 * Extract a palette as flat RGB bytes.
 */
export function extract_palette_from_rgba(data: Uint8Array, w: number, h: number, max_colors: number): Uint8Array;

/**
 * Parse hex color to RGB. Returns 3 bytes [r, g, b] or empty vec on failure.
 */
export function hex_to_rgb(hex: string): Uint8Array;

/**
 * Compute a flat RGBA histogram with 256 bins per channel.
 */
export function histogram_rgba(data: Uint8Array, w: number, h: number): Uint32Array;

/**
 * WCAG 2.x relative luminance. Returns -1.0 on failure.
 */
export function relative_luminance(hex: string): number;

/**
 * Quantize RGBA data to a provided RGB palette.
 */
export function quantize_rgba(data: Uint8Array, w: number, h: number, palette: Uint8Array): Uint8Array;

/**
 * Format RGB as hex string.
 */
export function rgb_to_hex(r: number, g: number, b: number): string;

/**
 * Returns true when the current JS runtime can instantiate the SIMD wasm build.
 */
export function simdSupported(): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_document_free: (a: number, b: number) => void;
    readonly contrast_ratio: (a: number, b: number, c: number, d: number) => number;
    readonly document_add_filter_layer: (a: number, b: number, c: number) => number;
    readonly document_add_filter_to_group: (a: number, b: number, c: number, d: number) => number;
    readonly document_add_gradient_layer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
    readonly document_add_group_layer: (a: number, b: number, c: number) => number;
    readonly document_add_group_to_group: (a: number, b: number, c: number, d: number) => number;
    readonly document_add_image_layer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => number;
    readonly document_add_image_to_group: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
    readonly document_add_paint_layer: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly document_add_png_layer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly document_add_solid_color_layer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly document_export_png: (a: number) => [number, number];
    readonly document_flatten_group: (a: number, b: number) => number;
    readonly document_get_layer_rgba: (a: number, b: number) => [number, number];
    readonly document_height: (a: number) => number;
    readonly document_layer_count: (a: number) => number;
    readonly document_new: (a: number, b: number) => number;
    readonly document_remove_from_group: (a: number, b: number, c: number) => number;
    readonly document_remove_layer_mask: (a: number, b: number) => void;
    readonly document_render: (a: number) => [number, number];
    readonly document_set_anchor: (a: number, b: number, c: number) => void;
    readonly document_set_blend_mode: (a: number, b: number, c: number, d: number) => void;
    readonly document_set_clip_to_below: (a: number, b: number, c: number) => void;
    readonly document_set_filter_config: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
    readonly document_set_flip: (a: number, b: number, c: number, d: number) => void;
    readonly document_set_layer_mask: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly document_set_opacity: (a: number, b: number, c: number) => void;
    readonly document_set_position: (a: number, b: number, c: number, d: number) => void;
    readonly document_set_rotation: (a: number, b: number, c: number) => void;
    readonly document_set_visible: (a: number, b: number, c: number) => void;
    readonly document_width: (a: number) => number;
    readonly dominant_rgb_from_rgba: (a: number, b: number, c: number, d: number) => [number, number];
    readonly hex_to_rgb: (a: number, b: number) => [number, number];
    readonly relative_luminance: (a: number, b: number) => number;
    readonly rgb_to_hex: (a: number, b: number, c: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

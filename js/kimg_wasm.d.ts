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
    filterConfig?: RawFilterConfig;
}

export interface RawLayerSnapshot {
    id: number;
    parentId: number | null;
    index: number;
    depth: number;
    kind: "image" | "paint" | "filter" | "group" | "solidColor" | "gradient";
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
}

export class Composition {
    constructor(width: number, height: number);
    static deserialize(data: Uint8Array): Composition;
    free(): void;
    [Symbol.dispose](): void;
    add_image_layer(name: string, rgba_data: Uint8Array, img_width: number, img_height: number, x: number, y: number): number;
    add_paint_layer(name: string, width: number, height: number): number;
    add_filter_layer(name: string): number;
    add_group_layer(name: string): number;
    add_solid_color_layer(name: string, r: number, g: number, b: number, a: number): number;
    add_gradient_layer(name: string, stops_colors: Uint8Array, stops_positions: Float64Array, direction: number): number;
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
    add_image_to_group(group_id: number, name: string, rgba_data: Uint8Array, img_width: number, img_height: number, x: number, y: number): number;
    add_filter_to_group(group_id: number, name: string): number;
    add_group_to_group(group_id: number, name: string): number;
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
    contact_sheet(layer_ids: Uint32Array, columns: number, padding: number, bg_r: number, bg_g: number, bg_b: number, bg_a: number): Uint8Array;
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

export function contrast_ratio(a: string, b: string): number;
export function decode_image(data: Uint8Array): Uint8Array;
export function detect_format(data: Uint8Array): string;
export function dominant_rgb_from_rgba(data: Uint8Array, w: number, h: number): Uint8Array;
export function extract_palette_from_rgba(data: Uint8Array, w: number, h: number, max_colors: number): Uint8Array;
export function hex_to_rgb(hex: string): Uint8Array;
export function histogram_rgba(data: Uint8Array, w: number, h: number): Uint32Array;
export function relative_luminance(hex: string): number;
export function quantize_rgba(data: Uint8Array, w: number, h: number, palette: Uint8Array): Uint8Array;
export function rgb_to_hex(r: number, g: number, b: number): string;
export function simdSupported(): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
export type SyncInitInput = BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly [key: string]: unknown;
}

export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

export default function init(
    module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>,
): Promise<InitOutput>;

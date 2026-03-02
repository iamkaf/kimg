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
    | "gradient";

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
}

export class Composition {
    private constructor();

    static create(width: number, height: number): Promise<Composition>;
    static create(options: CompositionOptions): Promise<Composition>;
    static deserialize(data: ByteInput): Promise<Composition>;

    readonly width: number;
    readonly height: number;

    free(): void;
    [Symbol.dispose](): void;

    addImageLayer(options: ImageLayerOptions): number;
    addPaintLayer(options: PaintLayerOptions): number;
    addFilterLayer(options: FilterLayerOptions): number;
    addGroupLayer(options: GroupLayerOptions): number;
    addSolidColorLayer(options: SolidColorLayerOptions): number;
    addGradientLayer(options: GradientLayerOptions): number;
    addPngLayer(options: PngLayerOptions): number;

    importImage(options: ImportImageOptions): number;
    importJpeg(options: ImportImageOptions): number;
    importWebp(options: ImportImageOptions): number;
    importGifFrames(options: { bytes: ByteInput }): Uint32Array;
    importPsd(options: { bytes: ByteInput }): Uint32Array;

    setLayerOpacity(id: number, opacity: number): void;
    setLayerVisibility(id: number, visible: boolean): void;
    setLayerPosition(id: number, x: number, y: number): void;
    setLayerPosition(id: number, position: Position): void;
    setLayerBlendMode(id: number, blendMode: string): void;
    setLayerMask(id: number, options: MaskOptions): void;
    clearLayerMask(id: number): void;
    setLayerMaskInverted(id: number, inverted: boolean): void;
    setLayerClipToBelow(id: number, clipToBelow: boolean): void;
    setLayerFlip(id: number, flipX: boolean, flipY: boolean): void;
    setLayerFlip(id: number, options: FlipOptions): void;
    setLayerRotation(id: number, rotation: number): void;
    setLayerAnchor(id: number, anchor: Anchor): void;
    setFilterLayerConfig(id: number, config: FilterConfig): void;
    updateLayer(id: number, patch: LayerUpdate): boolean;
    getLayer(id: number): LayerInfo | null;
    listLayers(options?: ListLayersOptions): LayerInfo[];
    removeLayer(id: number): boolean;
    moveLayer(id: number, target: MoveLayerTarget): boolean;
    resizeCanvas(width: number, height: number): void;
    resizeCanvas(size: Size): void;

    flattenGroup(groupId: number): boolean;
    removeFromGroup(groupId: number, childId: number): boolean;

    renderRgba(): Uint8Array;
    exportPng(): Uint8Array;
    exportJpeg(quality?: number): Uint8Array;
    exportJpeg(options: ExportJpegOptions): Uint8Array;
    exportWebp(): Uint8Array;
    serialize(): Uint8Array;
    getLayerRgba(id: number): Uint8Array;
    layerCount(): number;

    resizeLayerNearest(id: number, width: number, height: number): void;
    resizeLayerNearest(id: number, size: Size): void;
    resizeLayerBilinear(id: number, width: number, height: number): void;
    resizeLayerBilinear(id: number, size: Size): void;
    resizeLayerLanczos3(id: number, width: number, height: number): void;
    resizeLayerLanczos3(id: number, size: Size): void;
    cropLayer(id: number, x: number, y: number, width: number, height: number): void;
    cropLayer(id: number, rect: Position & Size): void;
    trimLayerAlpha(id: number): void;
    rotateLayer(id: number, angleDeg: number): void;
    boxBlurLayer(id: number, radius: number): void;
    boxBlurLayer(id: number, options: RadiusOptions): void;
    gaussianBlurLayer(id: number, radius: number): void;
    gaussianBlurLayer(id: number, options: RadiusOptions): void;
    sharpenLayer(id: number): void;
    edgeDetectLayer(id: number): void;
    embossLayer(id: number): void;
    invertLayer(id: number): void;
    posterizeLayer(id: number, levels: number): void;
    posterizeLayer(id: number, options: PosterizeOptions): void;
    thresholdLayer(id: number, threshold: number): void;
    thresholdLayer(id: number, options: ThresholdOptions): void;
    levelsLayer(id: number, shadows: number, midtones: number, highlights: number): void;
    levelsLayer(id: number, options: LevelsOptions): void;
    gradientMapLayer(id: number, stops: GradientStop[]): void;
    gradientMapLayer(id: number, options: { stops: GradientStop[] }): void;
    pixelScaleLayer(id: number, factor: number): void;
    pixelScaleLayer(id: number, options: PixelScaleOptions): void;
    extractPalette(id: number, maxColors: number): Uint8Array;
    extractPalette(id: number, options: PaletteOptions): Uint8Array;
    quantizeLayer(id: number, palette: ByteInput): void;
    quantizeLayer(id: number, options: QuantizeOptions): void;
    packSprites(layerIds: ArrayLike<number>, maxWidth: number, padding?: number): Uint8Array;
    packSprites(options: PackSpritesOptions): Uint8Array;
    packSpritesJson(layerIds: ArrayLike<number>, maxWidth: number, padding?: number): string;
    packSpritesJson(options: PackSpritesOptions): string;
    contactSheet(layerIds: ArrayLike<number>, columns: number, padding?: number, background?: ByteInput): Uint8Array;
    contactSheet(options: ContactSheetOptions): Uint8Array;
}

export function preload(
    module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>,
): Promise<InitOutput>;

export function simdSupported(): boolean;

export function contrastRatio(a: string, b: string): Promise<number>;
export function decodeImage(data: ByteInput): Promise<Uint8Array>;
export function detectFormat(data: ByteInput): Promise<string>;
export function dominantRgbFromRgba(data: ByteInput, width: number, height: number): Promise<Uint8Array>;
export function dominantRgbFromRgba(data: ByteInput, geometry: GeometryOptions): Promise<Uint8Array>;
export function extractPaletteFromRgba(
    data: ByteInput,
    width: number,
    height: number,
    maxColors: number,
): Promise<Uint8Array>;
export function extractPaletteFromRgba(data: ByteInput, options: GeometryWithMaxColorsOptions): Promise<Uint8Array>;
export function hexToRgb(hex: string): Promise<Uint8Array>;
export function histogramRgba(data: ByteInput, width: number, height: number): Promise<Uint32Array>;
export function histogramRgba(data: ByteInput, geometry: GeometryOptions): Promise<Uint32Array>;
export function quantizeRgba(data: ByteInput, width: number, height: number, palette: ByteInput): Promise<Uint8Array>;
export function quantizeRgba(data: ByteInput, options: GeometryWithPaletteOptions): Promise<Uint8Array>;
export function relativeLuminance(hex: string): Promise<number>;
export function rgbToHex(r: number, g: number, b: number): Promise<string>;
export function rgbToHex(rgb: RgbColor): Promise<string>;

export default preload;

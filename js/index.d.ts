import type {
    Composition as RawComposition,
    InitInput,
    InitOutput,
} from "./raw.js";

export interface CompositionOptions {
    width: number;
    height: number;
}

export class Composition {
    private constructor();
    static create(width: number, height: number): Promise<Composition>;
    static create(options: CompositionOptions): Promise<Composition>;
    static deserialize(data: Uint8Array): Promise<Composition>;
    free(): void;
    [Symbol.dispose](): void;
}

export interface Composition extends RawComposition {}

export function preload(
    module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>,
): Promise<InitOutput>;

export function simdSupported(): boolean;

export function contrastRatio(a: string, b: string): Promise<number>;
export function decodeImage(data: Uint8Array): Promise<Uint8Array>;
export function detectFormat(data: Uint8Array): Promise<string>;
export function dominantRgbFromRgba(data: Uint8Array, w: number, h: number): Promise<Uint8Array>;
export function extractPaletteFromRgba(data: Uint8Array, w: number, h: number, maxColors: number): Promise<Uint8Array>;
export function hexToRgb(hex: string): Promise<Uint8Array>;
export function histogramRgba(data: Uint8Array, w: number, h: number): Promise<Uint32Array>;
export function quantizeRgba(data: Uint8Array, w: number, h: number, palette: Uint8Array): Promise<Uint8Array>;
export function relativeLuminance(hex: string): Promise<number>;
export function rgbToHex(r: number, g: number, b: number): Promise<string>;

export default preload;

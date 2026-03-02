/* @ts-self-types="./index.d.ts" */

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

const PRIVATE_CONSTRUCTOR_TOKEN = Symbol("kimgComposition");

const COMPOSITION_PROXY_HANDLER = {
    get(target, prop, receiver) {
        if (Reflect.has(target, prop)) {
            return Reflect.get(target, prop, receiver);
        }

        const value = target._inner[prop];
        return typeof value === "function" ? value.bind(target._inner) : value;
    },
    set(target, prop, value, receiver) {
        if (Reflect.has(target, prop)) {
            return Reflect.set(target, prop, value, receiver);
        }

        target._inner[prop] = value;
        return true;
    },
};

let preloadPromise = null;

function isNodeRuntime() {
    return (
        typeof process === "object" &&
        process !== null &&
        typeof process.versions === "object" &&
        process.versions !== null &&
        typeof process.versions.node === "string"
    );
}

function normalizeCreateArgs(widthOrOptions, height) {
    if (typeof widthOrOptions === "object" && widthOrOptions !== null) {
        return {
            width: widthOrOptions.width,
            height: widthOrOptions.height,
        };
    }

    return {
        width: widthOrOptions,
        height,
    };
}

async function getDefaultInitInput() {
    if (!isNodeRuntime()) {
        return undefined;
    }

    const { readFile } = await import("node:fs/promises");
    const wasmName = simdSupported() ? "kimg_wasm_simd_bg.wasm" : "kimg_wasm_bg.wasm";
    return readFile(new URL(`./${wasmName}`, import.meta.url));
}

async function withPreload(fn) {
    await preload();
    return fn();
}

export async function preload(module_or_path) {
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

export class Composition {
    static #fromInner(inner) {
        return new Composition(inner, PRIVATE_CONSTRUCTOR_TOKEN);
    }

    constructor(inner, token) {
        if (token !== PRIVATE_CONSTRUCTOR_TOKEN) {
            throw new TypeError("Use await Composition.create(...) instead of new Composition(...).");
        }

        Object.defineProperty(this, "_inner", {
            configurable: false,
            enumerable: false,
            value: inner,
            writable: false,
        });
        return new Proxy(this, COMPOSITION_PROXY_HANDLER);
    }

    static async create(widthOrOptions, height) {
        const size = normalizeCreateArgs(widthOrOptions, height);
        await preload();
        return Composition.#fromInner(new RawComposition(size.width, size.height));
    }

    static async deserialize(data) {
        await preload();
        return Composition.#fromInner(RawComposition.deserialize(data));
    }

    free() {
        return this._inner.free();
    }
}

if (typeof Symbol.dispose === "symbol") {
    Composition.prototype[Symbol.dispose] = function () {
        if (typeof this._inner[Symbol.dispose] === "function") {
            return this._inner[Symbol.dispose]();
        }

        return this._inner.free();
    };
}

export { simdSupported };

export async function contrastRatio(a, b) {
    return withPreload(() => contrast_ratio(a, b));
}

export async function decodeImage(data) {
    return withPreload(() => decode_image(data));
}

export async function detectFormat(data) {
    return withPreload(() => detect_format(data));
}

export async function dominantRgbFromRgba(data, w, h) {
    return withPreload(() => dominant_rgb_from_rgba(data, w, h));
}

export async function extractPaletteFromRgba(data, w, h, maxColors) {
    return withPreload(() => extract_palette_from_rgba(data, w, h, maxColors));
}

export async function hexToRgb(hex) {
    return withPreload(() => hex_to_rgb(hex));
}

export async function histogramRgba(data, w, h) {
    return withPreload(() => histogram_rgba(data, w, h));
}

export async function quantizeRgba(data, w, h, palette) {
    return withPreload(() => quantize_rgba(data, w, h, palette));
}

export async function relativeLuminance(hex) {
    return withPreload(() => relative_luminance(hex));
}

export async function rgbToHex(r, g, b) {
    return withPreload(() => rgb_to_hex(r, g, b));
}

export default preload;

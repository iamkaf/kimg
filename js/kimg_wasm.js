/* @ts-self-types="./kimg_wasm.d.ts" */

import * as baselineBindings from "./kimg_wasm_bg.js";
import * as simdBindings from "./kimg_wasm_simd.js";

const SIMD_DETECT_BYTES = new Uint8Array([
    0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10,
    1, 8, 0, 65, 0, 253, 15, 253, 98, 11,
]);

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

let activeBindings = null;
let cachedSimdSupport;

function requireBindings() {
    if (activeBindings === null) {
        throw new Error("kimg WASM is not initialized. Call init() or initSync() first.");
    }

    return activeBindings;
}

function normalizeAsyncInitInput(module_or_path) {
    if (
        typeof module_or_path === "object" &&
        module_or_path !== null &&
        "module_or_path" in module_or_path
    ) {
        return module_or_path;
    }

    return { module_or_path };
}

function normalizeSyncInitInput(module) {
    if (typeof module === "object" && module !== null && "module" in module) {
        return module;
    }

    return { module };
}

export function simdSupported() {
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

export class Composition {
    static #fromInner(inner) {
        const composition = Object.create(Composition.prototype);
        Object.defineProperty(composition, "_inner", {
            configurable: false,
            enumerable: false,
            value: inner,
            writable: false,
        });
        return new Proxy(composition, COMPOSITION_PROXY_HANDLER);
    }

    constructor(width, height) {
        const bindings = requireBindings();
        return Composition.#fromInner(new bindings.Composition(width, height));
    }

    static deserialize(data) {
        return Composition.#fromInner(requireBindings().Composition.deserialize(data));
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

export function contrast_ratio(a, b) {
    return requireBindings().contrast_ratio(a, b);
}

export function decode_image(data) {
    return requireBindings().decode_image(data);
}

export function detect_format(data) {
    return requireBindings().detect_format(data);
}

export function dominant_rgb_from_rgba(data, w, h) {
    return requireBindings().dominant_rgb_from_rgba(data, w, h);
}

export function extract_palette_from_rgba(data, w, h, max_colors) {
    return requireBindings().extract_palette_from_rgba(data, w, h, max_colors);
}

export function hex_to_rgb(hex) {
    return requireBindings().hex_to_rgb(hex);
}

export function histogram_rgba(data, w, h) {
    return requireBindings().histogram_rgba(data, w, h);
}

export function initSync(module) {
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

export function relative_luminance(hex) {
    return requireBindings().relative_luminance(hex);
}

export function quantize_rgba(data, w, h, palette) {
    return requireBindings().quantize_rgba(data, w, h, palette);
}

export function rgb_to_hex(r, g, b) {
    return requireBindings().rgb_to_hex(r, g, b);
}

export default async function init(module_or_path) {
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

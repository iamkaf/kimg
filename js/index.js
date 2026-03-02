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

function normalizeByteInput(value, fieldName) {
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

function normalizeRgbaColor(value, fieldName) {
    const rgba = normalizeByteInput(value, fieldName);
    if (rgba.length !== 4) {
        throw new TypeError(`${fieldName} must contain exactly 4 RGBA bytes.`);
    }
    return rgba;
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

    if (typeof direction === "number" && Number.isInteger(direction) && direction >= 0 && direction <= 3) {
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

function normalizeExportJpegArg(qualityOrOptions) {
    if (typeof qualityOrOptions === "object" && qualityOrOptions !== null) {
        return normalizeInteger(qualityOrOptions.quality ?? 85, "exportJpeg.quality");
    }

    return normalizeInteger(qualityOrOptions ?? 85, "exportJpeg.quality");
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
    }

    static async create(widthOrOptions, height) {
        const size = normalizeCreateArgs(widthOrOptions, height);
        await preload();
        return Composition.#fromInner(new RawComposition(size.width, size.height));
    }

    static async deserialize(data) {
        await preload();
        return Composition.#fromInner(RawComposition.deserialize(normalizeByteInput(data, "data")));
    }

    get width() {
        return this._inner.width;
    }

    get height() {
        return this._inner.height;
    }

    free() {
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

        return this._inner.add_image_layer(layer.name, rgba, layer.width, layer.height, layer.x, layer.y);
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
        return this._inner.set_rotation(normalizeLayerId(id), normalizeFiniteNumber(rotation, "rotation"));
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
        const crop = typeof xOrOptions === "object" && xOrOptions !== null
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
        return this._inner.rotate_layer(normalizeLayerId(id), normalizeFiniteNumber(angleDeg, "angleDeg"));
    }

    boxBlurLayer(id, radiusOrOptions) {
        const radius = typeof radiusOrOptions === "object" && radiusOrOptions !== null
            ? radiusOrOptions.radius
            : radiusOrOptions;
        return this._inner.box_blur_layer(normalizeLayerId(id), normalizeInteger(radius, "boxBlurLayer.radius"));
    }

    gaussianBlurLayer(id, radiusOrOptions) {
        const radius = typeof radiusOrOptions === "object" && radiusOrOptions !== null
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
        const levels = typeof levelsOrOptions === "object" && levelsOrOptions !== null
            ? levelsOrOptions.levels
            : levelsOrOptions;
        return this._inner.posterize_layer(normalizeLayerId(id), normalizeInteger(levels, "posterizeLayer.levels"));
    }

    thresholdLayer(id, thresholdOrOptions) {
        const threshold = typeof thresholdOrOptions === "object" && thresholdOrOptions !== null
            ? thresholdOrOptions.threshold
            : thresholdOrOptions;
        return this._inner.threshold_layer(
            normalizeLayerId(id),
            normalizeFiniteNumber(threshold, "thresholdLayer.threshold"),
        );
    }

    levelsLayer(id, shadowsOrOptions, midtones, highlights) {
        const levels = typeof shadowsOrOptions === "object" && shadowsOrOptions !== null
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
        const factor = typeof factorOrOptions === "object" && factorOrOptions !== null
            ? factorOrOptions.factor
            : factorOrOptions;
        return this._inner.pixel_scale_layer(normalizeLayerId(id), normalizeInteger(factor, "pixelScaleLayer.factor"));
    }

    extractPalette(id, maxColorsOrOptions) {
        const maxColors = typeof maxColorsOrOptions === "object" && maxColorsOrOptions !== null
            ? maxColorsOrOptions.maxColors
            : maxColorsOrOptions;
        return this._inner.extract_palette(
            normalizeLayerId(id),
            normalizeInteger(maxColors, "extractPalette.maxColors"),
        );
    }

    quantizeLayer(id, paletteOrOptions) {
        const palette = paletteOrOptions && typeof paletteOrOptions === "object" && "palette" in paletteOrOptions
            ? paletteOrOptions.palette
            : paletteOrOptions;
        return this._inner.quantize_layer(
            normalizeLayerId(id),
            normalizeByteInput(palette, "quantizeLayer.palette"),
        );
    }

    packSprites(layerIdsOrOptions, maxWidth, padding) {
        const options = typeof layerIdsOrOptions === "object" &&
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
        const options = typeof layerIdsOrOptions === "object" &&
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
        const options = typeof layerIdsOrOptions === "object" &&
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
    Composition.prototype[Symbol.dispose] = function () {
        return this._inner.free();
    };
}

export { simdSupported };

export async function contrastRatio(a, b) {
    return withPreload(() => contrast_ratio(a, b));
}

export async function decodeImage(data) {
    return withPreload(() => decode_image(normalizeByteInput(data, "decodeImage.data")));
}

export async function detectFormat(data) {
    return withPreload(() => detect_format(normalizeByteInput(data, "detectFormat.data")));
}

export async function dominantRgbFromRgba(data, widthOrOptions, height) {
    const size = normalizeImageGeometryArg(widthOrOptions, height, "dominantRgbFromRgba");
    return withPreload(() =>
        dominant_rgb_from_rgba(normalizeByteInput(data, "dominantRgbFromRgba.data"), size.width, size.height),
    );
}

export async function extractPaletteFromRgba(data, widthOrOptions, height, maxColors) {
    const size = normalizeImageGeometryArg(widthOrOptions, height, "extractPaletteFromRgba");
    const paletteSize = typeof widthOrOptions === "object" && widthOrOptions !== null
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

export async function hexToRgb(hex) {
    return withPreload(() => hex_to_rgb(hex));
}

export async function histogramRgba(data, widthOrOptions, height) {
    const size = normalizeImageGeometryArg(widthOrOptions, height, "histogramRgba");
    return withPreload(() =>
        histogram_rgba(normalizeByteInput(data, "histogramRgba.data"), size.width, size.height),
    );
}

export async function quantizeRgba(data, widthOrOptions, height, palette) {
    const size = normalizeImageGeometryArg(widthOrOptions, height, "quantizeRgba");
    const paletteInput = typeof widthOrOptions === "object" && widthOrOptions !== null
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

export async function relativeLuminance(hex) {
    return withPreload(() => relative_luminance(hex));
}

export async function rgbToHex(rOrOptions, g, b) {
    const rgb = normalizeRgbArgs(rOrOptions, g, b);
    return withPreload(() => rgb_to_hex(rgb.r, rgb.g, rgb.b));
}

export default preload;

import { relative_luminance } from "./kimg_wasm.js";

/**
 * Pick a readable text color (black or white) for the given background.
 * Uses WCAG 2.x relative luminance with a 0.179 threshold.
 *
 * Requires WASM to be initialized before calling.
 *
 * @param {string} bgHex - Background color as hex string (e.g. "#3b82f6")
 * @returns {string} "#000000" for light backgrounds, "#ffffff" for dark
 */
export function readableTextColor(bgHex) {
    const lum = relative_luminance(bgHex);
    if (lum < 0) {
        return "#000000"; // fallback on invalid input
    }
    return lum > 0.179 ? "#000000" : "#ffffff";
}

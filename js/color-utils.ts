import { preload } from "./index.js";
import { relative_luminance } from "./raw.js";

await preload();

/**
 * Pick a readable text color (black or white) for the given background.
 * Uses WCAG 2.x relative luminance with a 0.179 threshold.
 *
 */
export function readableTextColor(bgHex: string): string {
    const lum = relative_luminance(bgHex);
    if (lum < 0) {
        return "#000000"; // fallback on invalid input
    }
    return lum > 0.179 ? "#000000" : "#ffffff";
}

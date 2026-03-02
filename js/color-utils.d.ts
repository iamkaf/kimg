/**
 * Pick a readable text color (black or white) for the given background.
 * Uses WCAG 2.x relative luminance with a 0.179 threshold.
 *
 * Requires WASM to be initialized before calling.
 *
 * @param bgHex - Background color as hex string (e.g. "#3b82f6")
 * @returns "#000000" for light backgrounds, "#ffffff" for dark
 */
export function readableTextColor(bgHex: string): string;

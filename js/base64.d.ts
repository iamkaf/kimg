/**
 * Encode raw RGBA bytes to a Base64 string.
 */
export function rgbaToBase64(data: Uint8Array): string;

/**
 * Decode a Base64 string back to raw RGBA bytes.
 */
export function base64ToRgba(str: string): Uint8Array;

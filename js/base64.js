/**
 * Encode raw RGBA bytes to a Base64 string.
 * @param {Uint8Array} data - RGBA pixel buffer
 * @returns {string} Base64-encoded string
 */
export function rgbaToBase64(data) {
  if (typeof Buffer !== "undefined") {
    return Buffer.from(data).toString("base64");
  }
  let binary = "";
  for (let i = 0; i < data.length; i++) {
    binary += String.fromCharCode(data[i]);
  }
  return btoa(binary);
}

/**
 * Decode a Base64 string back to raw RGBA bytes.
 * @param {string} str - Base64-encoded string
 * @returns {Uint8Array} RGBA pixel buffer
 */
export function base64ToRgba(str) {
  if (typeof Buffer !== "undefined") {
    const buf = Buffer.from(str, "base64");
    return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
  }
  const binary = atob(str);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Encode raw RGBA bytes to a Base64 string.
 */
export function rgbaToBase64(data: Uint8Array): string {
  const BufferCtor = (
    globalThis as {
      Buffer?: {
        from(input: Uint8Array | string, encoding?: string): { toString(encoding: string): string };
      };
    }
  ).Buffer;
  if (BufferCtor !== undefined) {
    return BufferCtor.from(data).toString("base64");
  }
  let binary = "";
  for (let i = 0; i < data.length; i++) {
    binary += String.fromCharCode(data[i]);
  }
  return btoa(binary);
}

/**
 * Decode a Base64 string back to raw RGBA bytes.
 */
export function base64ToRgba(str: string): Uint8Array {
  const BufferCtor = (
    globalThis as { Buffer?: { from(input: Uint8Array | string, encoding?: string): Uint8Array } }
  ).Buffer;
  if (BufferCtor !== undefined) {
    const buf = BufferCtor.from(str, "base64");
    return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
  }
  const binary = atob(str);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

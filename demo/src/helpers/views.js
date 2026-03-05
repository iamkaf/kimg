export function rgbaView(label, rgba, width, height, options = {}) {
  return { kind: "rgba", label, rgba, width, height, ...options };
}

export function swatchView(label, palette) {
  return { kind: "swatches", label, palette, wide: true };
}

export function codeView(label, text) {
  return { kind: "code", label, text, wide: true };
}

export function messageView(label, text) {
  return { kind: "message", label, text, wide: true };
}

export function splitPalette(palette) {
  const colors = [];
  for (let i = 0; i + 3 < palette.length; i += 4) {
    colors.push(palette.slice(i, i + 4));
  }
  return colors;
}

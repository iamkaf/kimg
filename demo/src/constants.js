export const SECTION_INFO = {
  setup: {
    chip: "Calibration",
    title: "Boot and Fixture Intake",
    description:
      "Source fixtures, preload, and decode utilities. These cards establish the known-good pixels the rest of the page will manipulate.",
  },
  layers: {
    chip: "Document",
    title: "Layer Graph and Compositing",
    description:
      "Layer creation, ordering, grouping, visibility, masking, clipping, flattening, and canvas resizing through the stable Composition facade.",
  },
  transforms: {
    chip: "Transforms",
    title: "Transform and Resample Paths",
    description:
      "Non-destructive translate, rotate, flip, anchor, scale, and destructive raster resampling paths side by side.",
  },
  filters: {
    chip: "Filters",
    title: "Color and Filter Paths",
    description:
      "Scoped filter layers and destructive layer filters. The same source image is reused so regressions are easy to spot.",
  },
  shapes: {
    chip: "Shapes",
    title: "Shape, Fill, and Palette Tools",
    description:
      "Vector-style shape layers, brush strokes, bucket fill behavior, palette extraction, and quantization outputs that should be visually legible without any interaction.",
  },
  text: {
    chip: "Text",
    title: "Text Layer Surface",
    description:
      "Registered real-font text layers rendered through the public API. Registration, multiline layout, tracking, color, and transform updates should all stay visually obvious.",
  },
  brushStrokes: {
    chip: "Brushes",
    title: "Brush Stroke Engine",
    description:
      "Streaming and static brush stroke sessions isolated as individual tests. Each exercises one aspect: tip kind, pressure, erase mode, cancel path, and alpha lock.",
  },
  colorUtils: {
    chip: "Colors",
    title: "Color Utilities",
    description:
      "Free-function color API: hex/RGB round trips, relative luminance, contrast ratio, dominant color extraction, histogram, and palette extraction on the teapot fixture.",
  },
  io: {
    chip: "Formats",
    title: "Format, SVG, Sprite, and Utility Surface",
    description:
      "Retained SVG layers, serialization, PNG/JPEG/WebP import-export, GIF frame import, sprite helpers, and utility outputs that verify package-level APIs beyond rendering.",
  },
  volume: {
    chip: "Volume",
    title: "Verification Through Volume",
    description:
      "High-cardinality matrices over real assets and feature combinations. The goal is visual bug discovery through breadth, not a handful of handpicked examples.",
  },
  experimental: {
    chip: "Experimental",
    title: "Experimental Surface",
    description:
      "Unstable or intentionally deprioritized paths that remain visible on the page without being counted as stable pass/fail requirements.",
  },
};

export const SECTION_ORDER = [
  "setup",
  "layers",
  "transforms",
  "filters",
  "shapes",
  "text",
  "brushStrokes",
  "colorUtils",
  "io",
  "volume",
  "experimental",
];

export const VOLUME_RASTER_ASSETS = [
  { key: "teapot", label: "Teapot PNG", path: "./assets/teapot.png", maxEdge: 164 },
  { key: "flower", label: "Flower PNG", path: "./assets/flower.png", maxEdge: 164 },
  { key: "croissantPng", label: "Croissant PNG", path: "./assets/croissant.png", maxEdge: 164 },
  { key: "gardenPhoto", label: "Garden JPG", path: "./assets/garden-photo.jpg", maxEdge: 164 },
  { key: "mugPhoto", label: "Mug JPG", path: "./assets/mug-photo.jpg", maxEdge: 164 },
  { key: "cactusPhoto", label: "Cactus JPG", path: "./assets/cactus-photo.jpg", maxEdge: 164 },
];

export const VOLUME_SVG_ASSETS = [
  { key: "croissantSvg", label: "Croissant SVG", path: "./assets/croissant.svg" },
];

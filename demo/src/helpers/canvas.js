import { splitPalette } from "./views.js";

export function canvasFromRgba(rgba, width, height, options = {}) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);

  const maxDisplay = options.maxDisplay ?? 220;
  const scale = Math.max(1, Math.floor(maxDisplay / Math.max(width, height)));
  canvas.style.width = `${width * scale}px`;
  canvas.style.height = `${height * scale}px`;
  return canvas;
}

export function chooseViewMaxDisplay(view) {
  if (view.width > view.height * 1.25) return 320;
  if (view.width >= 160 && view.height >= 160) return 280;
  if (view.width <= 96 && view.height <= 96) return 200;
  return 220;
}

export function downloadCardImage(payload) {
  const canvas = renderCardExport(payload);
  canvas.toBlob((blob) => {
    if (blob === null) return;
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${slugify(payload.title)}.png`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }, "image/png");
}

export function slugify(text) {
  return text
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, "-")
    .replaceAll(/^-+|-+$/g, "")
    .slice(0, 80);
}

function renderCardExport(payload) {
  const width = 1380;
  const padding = 44;
  const gap = 24;
  const contentWidth = width - padding * 2;
  const metrics = payload.result.metrics ?? [];
  const views = payload.result.views ?? [];
  const columns = chooseExportColumns(views);
  const cellWidth =
    columns === 1 ? contentWidth : Math.floor((contentWidth - gap * (columns - 1)) / columns);

  const measureCanvas = document.createElement("canvas");
  const mc = measureCanvas.getContext("2d");

  let height = padding;
  height += measureWrappedText(mc, payload.title, "700 48px Iowan Old Style", contentWidth, 54);
  height += 18;
  height += measureWrappedText(
    mc,
    `Look for: ${payload.expectation ?? ""}`.trim(),
    "400 28px Iowan Old Style",
    contentWidth,
    40,
  );

  if (payload.result.note) {
    height += 20;
    height += measureWrappedText(mc, `Note: ${payload.result.note}`, "400 25px Iowan Old Style", contentWidth, 36);
  }

  if (metrics.length > 0) {
    height += 28;
    height += measureMetricGrid(mc, metrics, contentWidth, gap);
  }

  if (views.length > 0) {
    height += 30;
    height += measureViewGrid(mc, views, columns, cellWidth, gap);
  }

  height += 26 + 34 + padding;

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = Math.ceil(height);
  const ctx = canvas.getContext("2d");

  ctx.fillStyle = "#0c0c10";
  ctx.fillRect(0, 0, width, canvas.height);

  let y = padding;
  ctx.font = "700 48px Iowan Old Style";
  y = drawWrappedText(ctx, payload.title, padding, y, contentWidth, 54, "#ddddf0");

  y += 10;
  ctx.font = "400 28px Iowan Old Style";
  y = drawWrappedText(
    ctx,
    `Look for: ${payload.expectation ?? ""}`.trim(),
    padding,
    y,
    contentWidth,
    40,
    "#7878a0",
  );

  if (payload.result.note) {
    y += 20;
    ctx.font = "400 25px Iowan Old Style";
    y = drawWrappedText(ctx, `Note: ${payload.result.note}`, padding, y, contentWidth, 36, "#7878a0");
  }

  if (metrics.length > 0) {
    y += 24;
    y = drawMetricGrid(ctx, metrics, padding, y, contentWidth, gap);
  }

  if (views.length > 0) {
    y += 26;
    y = drawViewGrid(ctx, views, padding, y, columns, cellWidth, gap);
  }

  y += 24;
  ctx.strokeStyle = "rgba(255,255,255,0.07)";
  ctx.beginPath();
  ctx.moveTo(padding, y);
  ctx.lineTo(width - padding, y);
  ctx.stroke();

  y += 28;
  ctx.font = "600 22px Cascadia Code";
  ctx.fillStyle = "#7878a0";
  ctx.fillText(`${payload.result.assertions} checks in ${Math.round(payload.elapsedMs)} ms`, padding, y);

  return canvas;
}

function chooseExportColumns(views) {
  const denseCount = views.filter((v) => v.kind === "rgba" || v.kind === "swatches").length;
  if (denseCount >= 8) return 3;
  if (denseCount >= 2) return 2;
  return 1;
}

function measureWrappedText(ctx, text, font, maxWidth, lineHeight) {
  ctx.font = font;
  return wrapText(ctx, text, maxWidth).length * lineHeight;
}

function drawWrappedText(ctx, text, x, y, maxWidth, lineHeight, color) {
  const lines = wrapText(ctx, text, maxWidth);
  ctx.fillStyle = color;
  for (const line of lines) {
    ctx.fillText(line, x, y);
    y += lineHeight;
  }
  return y;
}

function wrapText(ctx, text, maxWidth) {
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length === 0) return [""];
  const lines = [];
  let line = words[0];
  for (let i = 1; i < words.length; i++) {
    const candidate = `${line} ${words[i]}`;
    if (ctx.measureText(candidate).width <= maxWidth) {
      line = candidate;
    } else {
      lines.push(line);
      line = words[i];
    }
  }
  lines.push(line);
  return lines;
}

function measureMetricGrid(ctx, metrics, contentWidth, gap) {
  const mw = Math.floor((contentWidth - gap) / 2);
  ctx.font = "600 18px Cascadia Code";
  let height = 0;
  for (let i = 0; i < metrics.length; i += 2) {
    const rh = Math.max(
      measureMetricBlock(ctx, metrics[i], mw),
      metrics[i + 1] ? measureMetricBlock(ctx, metrics[i + 1], mw) : 0,
    );
    height += rh + gap;
  }
  return height - gap;
}

function measureMetricBlock(ctx, [label, value], width) {
  ctx.font = "600 18px Cascadia Code";
  const lh = wrapText(ctx, String(label), width).length * 26;
  ctx.font = "700 22px Cascadia Code";
  const vh = wrapText(ctx, String(value), width).length * 30;
  return Math.max(86, 20 + lh + vh);
}

function drawMetricGrid(ctx, metrics, x, y, contentWidth, gap) {
  const mw = Math.floor((contentWidth - gap) / 2);
  for (let i = 0; i < metrics.length; i += 2) {
    const lh = drawMetricBlock(ctx, metrics[i], x, y, mw);
    const rh = metrics[i + 1] ? drawMetricBlock(ctx, metrics[i + 1], x + mw + gap, y, mw) : 0;
    y += Math.max(lh, rh) + gap;
  }
  return y - gap;
}

function drawMetricBlock(ctx, [label, value], x, y, width) {
  const h = measureMetricBlock(ctx, [label, value], width);
  drawRoundedRect(ctx, x, y, width, h, 18, "#14141a", "rgba(255,255,255,0.07)");
  ctx.font = "600 18px Cascadia Code";
  y = drawWrappedText(ctx, String(label), x + 18, y + 28, width - 36, 26, "#7878a0");
  ctx.font = "700 22px Cascadia Code";
  drawWrappedText(ctx, String(value), x + 18, y + 8, width - 36, 30, "#ddddf0");
  return h;
}

function measureViewGrid(ctx, views, columns, cellWidth, gap) {
  let height = 0;
  let rowHeight = 0;
  let col = 0;
  for (const view of views) {
    const full = view.kind === "code" || view.kind === "message" || view.wide === true;
    const vh = measureExportView(ctx, view, full ? cellWidth * columns + gap * (columns - 1) : cellWidth);
    if (full) {
      if (col !== 0) { height += rowHeight + gap; rowHeight = 0; col = 0; }
      height += vh + gap;
      continue;
    }
    rowHeight = Math.max(rowHeight, vh);
    col++;
    if (col >= columns) { height += rowHeight + gap; rowHeight = 0; col = 0; }
  }
  if (col !== 0) height += rowHeight + gap;
  return Math.max(0, height - gap);
}

function drawViewGrid(ctx, views, x, y, columns, cellWidth, gap) {
  let rowHeight = 0;
  let col = 0;
  for (const view of views) {
    const full = view.kind === "code" || view.kind === "message" || view.wide === true;
    const dw = full ? cellWidth * columns + gap * (columns - 1) : cellWidth;
    const vh = measureExportView(ctx, view, dw);
    if (full) {
      if (col !== 0) { y += rowHeight + gap; rowHeight = 0; col = 0; }
      drawExportView(ctx, view, x, y, dw, vh);
      y += vh + gap;
      continue;
    }
    drawExportView(ctx, view, x + col * (cellWidth + gap), y, dw, vh);
    rowHeight = Math.max(rowHeight, vh);
    col++;
    if (col >= columns) { y += rowHeight + gap; rowHeight = 0; col = 0; }
  }
  if (col !== 0) y += rowHeight + gap;
  return y - gap;
}

function measureExportView(ctx, view, width) {
  if (view.kind === "rgba") {
    return Math.min(420, Math.max(140, Math.round((width - 28) * (view.height / view.width)))) + 72;
  }
  if (view.kind === "swatches") {
    const colors = splitPalette(view.palette).length;
    const cols = Math.max(4, Math.min(8, Math.floor((width - 36) / 56)));
    return 72 + Math.ceil(colors / cols) * 56;
  }
  ctx.font = "500 20px Cascadia Code";
  const text = view.kind === "code" || view.kind === "message" ? view.text : "";
  return 86 + wrapText(ctx, text, width - 36).length * 28;
}

function drawExportView(ctx, view, x, y, width, height) {
  drawRoundedRect(ctx, x, y, width, height, 22, "#14141a", "rgba(255,255,255,0.07)");
  ctx.font = "600 18px Cascadia Code";
  ctx.fillStyle = "#7878a0";
  ctx.fillText(view.label, x + 18, y + 30);

  if (view.kind === "rgba") {
    const bx = x + 14, by = y + 44, bw = width - 28, bh = height - 58;
    drawCheckerboard(ctx, bx, by, bw, bh, 18);
    const temp = document.createElement("canvas");
    temp.width = view.width; temp.height = view.height;
    temp.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(view.rgba), view.width, view.height), 0, 0);
    const scale = Math.min(bw / view.width, bh / view.height);
    const dw = Math.max(1, Math.round(view.width * scale));
    const dh = Math.max(1, Math.round(view.height * scale));
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(temp, bx + Math.floor((bw - dw) / 2), by + Math.floor((bh - dh) / 2), dw, dh);
    ctx.imageSmoothingEnabled = true;
    return;
  }

  if (view.kind === "swatches") {
    const colors = splitPalette(view.palette);
    const cols = Math.max(4, Math.min(8, Math.floor((width - 36) / 56)));
    const size = Math.floor((width - 36 - (cols - 1) * 10) / cols);
    let cx = x + 18, cy = y + 48;
    colors.forEach((color, i) => {
      ctx.fillStyle = `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${color[3] / 255})`;
      drawRoundedRect(ctx, cx, cy, size, size, 12, ctx.fillStyle, "rgba(255,255,255,0.07)");
      cx += size + 10;
      if ((i + 1) % cols === 0) { cx = x + 18; cy += size + 10; }
    });
    return;
  }

  ctx.font = "500 20px Cascadia Code";
  drawWrappedText(ctx, view.text, x + 18, y + 56, width - 36, 28, "#ddddf0");
}

export function drawCheckerboard(ctx, x, y, width, height, size) {
  ctx.fillStyle = "#14141a";
  ctx.fillRect(x, y, width, height);
  for (let row = 0; row < Math.ceil(height / size); row++) {
    for (let col = 0; col < Math.ceil(width / size); col++) {
      if ((row + col) % 2 === 0) {
        ctx.fillStyle = "#1c1c24";
        ctx.fillRect(x + col * size, y + row * size, size, size);
      }
    }
  }
  ctx.strokeStyle = "rgba(255,255,255,0.04)";
  ctx.strokeRect(x, y, width, height);
}

export function drawRoundedRect(ctx, x, y, width, height, radius, fill, stroke) {
  ctx.beginPath();
  ctx.moveTo(x + radius, y);
  ctx.lineTo(x + width - radius, y);
  ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
  ctx.lineTo(x + width, y + height - radius);
  ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  ctx.lineTo(x + radius, y + height);
  ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
  ctx.lineTo(x, y + radius);
  ctx.quadraticCurveTo(x, y, x + radius, y);
  ctx.closePath();
  ctx.fillStyle = fill;
  ctx.fill();
  ctx.strokeStyle = stroke;
  ctx.stroke();
}

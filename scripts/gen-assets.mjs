// Dependency-free placeholder-asset generator.
//
// Produces:
//   - src/sprites/idle.png      the idle sprite sheet (horizontal strip)
//   - src-tauri/icon-source.png  a 1024x1024 source for `pnpm tauri icon`
//
// The sprite-sheet convention (the asset contract every mood follows):
//   A sheet is a single horizontal strip of `frames` cells, each `frameW x
//   frameH` pixels, frame 0 leftmost. The renderer plays cells left-to-right
//   at a fixed `fps`, looping. Background is fully transparent (RGBA).
//
// Run: `pnpm gen:assets`  (re-run after editing this file).

import zlib from "node:zlib";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// ---- minimal PNG encoder (8-bit RGBA) ----------------------------------
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([len, body, crc]);
}

function encodePNG(width, height, rgba) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  const stride = width * 4;
  const raw = Buffer.alloc(height * (stride + 1));
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0; // filter type: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, y * stride + stride);
  }
  const idat = zlib.deflateSync(raw, { level: 9 });
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
}

// ---- drawing -----------------------------------------------------------
const BODY = [91, 209, 176, 255];
const EDGE = [40, 120, 100, 255];
const EYE = [28, 38, 38, 255];

function setPx(buf, w, h, x, y, col) {
  if (x < 0 || y < 0 || x >= w || y >= h) return;
  const i = (y * w + x) * 4;
  buf[i] = col[0];
  buf[i + 1] = col[1];
  buf[i + 2] = col[2];
  buf[i + 3] = col[3];
}

// Draw one breathing pixel-blob centered in a cell whose top-left is (ox, oy).
function drawBlob(buf, w, h, ox, oy, scale, breath) {
  const cx = ox + 16 * scale;
  const cy = oy + 22 * scale;
  const rx = 11 * scale;
  const ry = (10 + breath) * scale;
  const x0 = Math.floor(ox);
  const y0 = Math.floor(oy);
  for (let y = y0; y < y0 + 32 * scale; y++) {
    for (let x = x0; x < x0 + 32 * scale; x++) {
      const dx = (x - cx) / rx;
      const dy = (y - cy) / ry;
      const d = dx * dx + dy * dy;
      if (d <= 1) setPx(buf, w, h, x, y, d > 0.78 ? EDGE : BODY);
    }
  }
  // Eyes, riding just above the blob's vertical center.
  const ey = Math.round(cy - ry * 0.45);
  for (const ex of [Math.round(ox + 12 * scale), Math.round(ox + 20 * scale)]) {
    for (let dy = 0; dy < Math.max(1, scale); dy++) {
      for (let dx = 0; dx < Math.max(1, scale); dx++) {
        setPx(buf, w, h, ex + dx, ey + dy, EYE);
      }
    }
  }
}

// ---- idle sprite sheet -------------------------------------------------
const FW = 32;
const FH = 32;
const FRAMES = 4;
const BREATH = [0, -1, 0, 1]; // gentle squash/stretch over the loop
const sheetW = FW * FRAMES;
const sheet = Buffer.alloc(sheetW * FH * 4); // zeroed => transparent
for (let f = 0; f < FRAMES; f++) {
  drawBlob(sheet, sheetW, FH, f * FW, 0, 1, BREATH[f]);
}
fs.mkdirSync(path.join(ROOT, "src/sprites"), { recursive: true });
fs.writeFileSync(path.join(ROOT, "src/sprites/idle.png"), encodePNG(sheetW, FH, sheet));
console.log(`wrote src/sprites/idle.png (${sheetW}x${FH}, ${FRAMES} frames of ${FW}x${FH})`);

// ---- app icon source ---------------------------------------------------
const IS = 1024;
const icon = Buffer.alloc(IS * IS * 4);
drawBlob(icon, IS, IS, 0, 0, IS / 32, 0);
fs.writeFileSync(path.join(ROOT, "src-tauri/icon-source.png"), encodePNG(IS, IS, icon));
console.log(`wrote src-tauri/icon-source.png (${IS}x${IS})`);

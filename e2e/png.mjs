// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Decodes 8-bit RGB and RGBA PNG screenshots so tests can measure overlay pixels.

import { inflateSync } from "node:zlib";

const SIGNATURE = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

export function decodePng(buffer) {
  if (!buffer.subarray(0, 8).equals(SIGNATURE)) throw new Error("not a PNG image");
  let offset = 8;
  let header;
  const data = [];
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("latin1", offset + 4, offset + 8);
    const body = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      header = {
        width: body.readUInt32BE(0),
        height: body.readUInt32BE(4),
        depth: body[8],
        colorType: body[9],
        interlace: body[12],
      };
    } else if (type === "IDAT") {
      data.push(body);
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }
  if (!header) throw new Error("PNG has no header");
  const channels = { 2: 3, 6: 4 }[header.colorType];
  if (header.depth !== 8 || !channels || header.interlace !== 0) {
    throw new Error(`unsupported PNG: depth ${header.depth}, color type ${header.colorType}`);
  }
  const raw = inflateSync(Buffer.concat(data));
  const stride = header.width * channels;
  const pixels = Buffer.alloc(header.width * header.height * 4);
  let previous = Buffer.alloc(stride);
  for (let y = 0; y < header.height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = Buffer.from(raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)));
    unfilter(filter, line, previous, channels);
    for (let x = 0; x < header.width; x++) {
      const target = (y * header.width + x) * 4;
      pixels[target] = line[x * channels];
      pixels[target + 1] = line[x * channels + 1];
      pixels[target + 2] = line[x * channels + 2];
      pixels[target + 3] = channels === 4 ? line[x * channels + 3] : 255;
    }
    previous = line;
  }
  return { width: header.width, height: header.height, pixels };
}

function unfilter(filter, line, previous, bpp) {
  for (let i = 0; i < line.length; i++) {
    const left = i >= bpp ? line[i - bpp] : 0;
    const up = previous[i];
    const upLeft = i >= bpp ? previous[i - bpp] : 0;
    const predictor = [0, left, up, (left + up) >> 1, paeth(left, up, upLeft)][filter];
    if (predictor === undefined) throw new Error(`unknown PNG filter ${filter}`);
    line[i] = (line[i] + predictor) & 0xff;
  }
}

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

/** Mean alpha (0-255) over a rectangle given in fractions of the image size. */
export function meanAlpha(image, { x0, y0, x1, y1 }) {
  const [left, right] = [Math.floor(x0 * image.width), Math.ceil(x1 * image.width)];
  const [top, bottom] = [Math.floor(y0 * image.height), Math.ceil(y1 * image.height)];
  let sum = 0;
  let count = 0;
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < right; x++) {
      sum += image.pixels[(y * image.width + x) * 4 + 3];
      count++;
    }
  }
  return count ? sum / count : 0;
}

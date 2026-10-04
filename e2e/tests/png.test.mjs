// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the screenshot PNG decoder and alpha measurement used by the end-to-end test.

import assert from "node:assert/strict";
import { test } from "node:test";
import { deflateSync } from "node:zlib";

import { decodePng, meanAlpha } from "../png.mjs";

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, body) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(body.length);
  const typed = Buffer.concat([Buffer.from(type, "latin1"), body]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typed));
  return Buffer.concat([length, typed, crc]);
}

/** Encodes RGBA rows, applying the given PNG filter to every line. */
function encode(width, height, rgba, filter) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  const stride = width * 4;
  const raw = [];
  for (let y = 0; y < height; y++) {
    const line = rgba.subarray(y * stride, (y + 1) * stride);
    const previous = y ? rgba.subarray((y - 1) * stride, y * stride) : Buffer.alloc(stride);
    const encoded = Buffer.alloc(stride);
    for (let i = 0; i < stride; i++) {
      const left = i >= 4 ? line[i - 4] : 0;
      const predictor = filter === 1 ? left : filter === 2 ? previous[i] : 0;
      encoded[i] = (line[i] - predictor) & 0xff;
    }
    raw.push(Buffer.from([filter]), encoded);
  }
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(Buffer.concat(raw))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function image(width, height, alphaAt) {
  const rgba = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      rgba.set([x * 10, y * 10, 7, alphaAt(x, y)], i);
    }
  }
  return rgba;
}

for (const filter of [0, 1, 2]) {
  test(`decodes RGBA with filter ${filter}`, () => {
    const rgba = image(6, 4, (x) => (x < 3 ? 0 : 255));
    const decoded = decodePng(encode(6, 4, rgba, filter));
    assert.equal(decoded.width, 6);
    assert.deepEqual([...decoded.pixels], [...rgba]);
  });
}

test("mean alpha measures a fractional rectangle", () => {
  const decoded = decodePng(encode(10, 10, image(10, 10, (x) => (x >= 5 ? 200 : 0)), 0));
  assert.equal(meanAlpha(decoded, { x0: 0.5, y0: 0, x1: 1, y1: 1 }), 200);
  assert.equal(meanAlpha(decoded, { x0: 0, y0: 0, x1: 0.5, y1: 1 }), 0);
});

test("rejects data that is not a PNG", () => {
  assert.throws(() => decodePng(Buffer.from("not a png")), /not a PNG/);
});

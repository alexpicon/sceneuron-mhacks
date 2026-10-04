// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Reads a ZIP archive the way an unzip tool does, from its central directory to each local
// header, checking every file's size and CRC, so the profile tests read what the build wrote.

import { crc32, inflateRawSync } from "node:zlib";

const END_OF_CENTRAL_DIRECTORY = Buffer.from([0x50, 0x4b, 0x05, 0x06]);
const CENTRAL_HEADER = 0x02014b50;
const LOCAL_HEADER = 0x04034b50;

/** The entries of `buffer` in central directory order: `{path, method, external, data}`. */
export function readZip(buffer) {
  const end = buffer.lastIndexOf(END_OF_CENTRAL_DIRECTORY);
  if (end < 0) {
    throw new Error("no end of central directory");
  }
  const count = buffer.readUInt16LE(end + 10);
  let at = buffer.readUInt32LE(end + 16);
  const entries = [];
  for (let index = 0; index < count; index += 1) {
    if (buffer.readUInt32LE(at) !== CENTRAL_HEADER) {
      throw new Error(`no central header at ${at}`);
    }
    const method = buffer.readUInt16LE(at + 10);
    const crc = buffer.readUInt32LE(at + 16);
    const compressed = buffer.readUInt32LE(at + 20);
    const size = buffer.readUInt32LE(at + 24);
    const nameLength = buffer.readUInt16LE(at + 28);
    const skipped = buffer.readUInt16LE(at + 30) + buffer.readUInt16LE(at + 32);
    const external = buffer.readUInt32LE(at + 38);
    const offset = buffer.readUInt32LE(at + 42);
    const path = buffer.toString("utf8", at + 46, at + 46 + nameLength);
    if (buffer.readUInt32LE(offset) !== LOCAL_HEADER) {
      throw new Error(`no local header for ${path}`);
    }
    const start = offset + 30 + buffer.readUInt16LE(offset + 26) + buffer.readUInt16LE(offset + 28);
    const raw = buffer.subarray(start, start + compressed);
    if (method !== 0 && method !== 8) {
      throw new Error(`${path} uses compression method ${method}`);
    }
    const data = method === 8 ? inflateRawSync(raw) : raw;
    if (data.length !== size || crc32(data) !== crc) {
      throw new Error(`${path} does not match its size or CRC`);
    }
    entries.push({ path, method, external, data });
    at += 46 + nameLength + skipped;
  }
  return entries;
}

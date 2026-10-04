// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * A small ZIP writer for profiles: folders stored, files deflated, as Stream Deck's own exported
 * profiles are, and every entry dated the same, so the same entries always make the same bytes.
 */

import { crc32, deflateRawSync } from "node:zlib";

/** A file, or a folder when `path` ends with "/" and there is no `data`. */
export interface ZipEntry {
  readonly path: string;
  readonly data?: Buffer;
}

const LOCAL_HEADER = 0x04034b50;
const CENTRAL_HEADER = 0x02014b50;
const END_OF_CENTRAL_DIRECTORY = 0x06054b50;
/** ZIP 2.0, the first version with folders and deflate. */
const VERSION = 20;
const STORED = 0;
const DEFLATED = 8;
/** The MS-DOS attribute that marks a folder. */
const FOLDER_ATTRIBUTE = 0x10;
/** 2026-10-01 at midnight, in MS-DOS date and time fields. */
const DOS_DATE = ((2026 - 1980) << 9) | (10 << 5) | 1;
const DOS_TIME = 0;

/** The entries as one ZIP archive, in the order given. */
export function zip(entries: readonly ZipEntry[]): Buffer {
  const locals: Buffer[] = [];
  const centrals: Buffer[] = [];
  let offset = 0;
  for (const entry of entries) {
    const folder = entry.path.endsWith("/");
    if (folder === (entry.data !== undefined)) {
      const message = `zip entry ${entry.path} must be a folder ending in "/" or a file with data`;
      throw new Error(message);
    }
    const name = Buffer.from(entry.path, "utf8");
    const data = entry.data ?? Buffer.alloc(0);
    const stored = folder ? data : deflateRawSync(data);
    const fields = {
      method: folder ? STORED : DEFLATED,
      crc: folder ? 0 : crc32(data),
      compressed: stored.length,
      size: data.length,
      name: name.length,
    };
    const local = Buffer.alloc(30);
    local.writeUInt32LE(LOCAL_HEADER, 0);
    writeCommon(local, 4, fields);
    locals.push(local, name, stored);

    const central = Buffer.alloc(46);
    central.writeUInt32LE(CENTRAL_HEADER, 0);
    central.writeUInt16LE(VERSION, 4);
    writeCommon(central, 6, fields);
    central.writeUInt32LE(folder ? FOLDER_ATTRIBUTE : 0, 38);
    central.writeUInt32LE(offset, 42);
    centrals.push(central, name);
    offset += local.length + name.length + stored.length;
  }
  const directory = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(END_OF_CENTRAL_DIRECTORY, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, directory, end]);
}

interface Fields {
  readonly method: number;
  readonly crc: number;
  readonly compressed: number;
  readonly size: number;
  readonly name: number;
}

/**
 * The fields local and central headers share, from "version needed" to the extra field length;
 * the flags and the extra field stay zero.
 */
function writeCommon(header: Buffer, at: number, fields: Fields): void {
  header.writeUInt16LE(VERSION, at);
  header.writeUInt16LE(fields.method, at + 4);
  header.writeUInt16LE(DOS_TIME, at + 6);
  header.writeUInt16LE(DOS_DATE, at + 8);
  header.writeUInt32LE(fields.crc, at + 10);
  header.writeUInt32LE(fields.compressed, at + 14);
  header.writeUInt32LE(fields.size, at + 18);
  header.writeUInt16LE(fields.name, at + 22);
}

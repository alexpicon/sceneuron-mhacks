// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how the end-to-end proof reads the camera preview: the size in a JPEG's start-of-frame segment, and
// whether a picture is a whole JPEG of at most 640 pixels a side in the proof clip's 16:9 shape; how it compares the
// editor's audio sources with OBS's own record of its global audio devices; and whether a live description counts a
// person.

import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  describesPerson,
  globalDeviceNames,
  isClipPreview,
  jpegSize,
  listsGlobalDevicesFirst,
} from "../camera-mic-obs-steps.mjs";

/** A JPEG's markers without image data: SOI, an APP0 segment, a start of frame of `marker`, and EOI. */
function jpeg(width, height, marker = 0xc0) {
  const app0 = [0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0, 0];
  const frame = [0xff, marker, 0x00, 0x11, 0x08, height >> 8, height & 0xff, width >> 8, width & 0xff, 0x03];
  return Uint8Array.from([0xff, 0xd8, ...app0, ...frame, ...Array(9).fill(0), 0xff, 0xd9]);
}

test("the size comes from the start of frame, past other segments", () => {
  assert.deepEqual(jpegSize(jpeg(640, 360)), { width: 640, height: 360 });
  assert.deepEqual(jpegSize(jpeg(1280, 720, 0xc2)), { width: 1280, height: 720 }, "progressive");
});

test("bytes that are not a JPEG have no size", () => {
  assert.equal(jpegSize(Uint8Array.from([0x89, 0x50, 0x4e, 0x47])), null);
  assert.equal(jpegSize(Uint8Array.from([0xff, 0xd8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])), null);
  assert.equal(jpegSize(Uint8Array.from([0xff, 0xd8, 0xff, 0xd9])), null);
});

test("a clip preview is a whole 16:9 JPEG at most 640 pixels wide", () => {
  assert.equal(isClipPreview(jpeg(640, 360)), true);
  assert.equal(isClipPreview(jpeg(320, 180)), true);
  assert.equal(isClipPreview(jpeg(1280, 720)), false, "too large");
  assert.equal(isClipPreview(jpeg(320, 240)), false, "stretched to 4:3");
  assert.equal(isClipPreview(jpeg(640, 360).slice(0, -2)), false, "cut short");
});

test("a live description counts a person when it starts with one or more people", () => {
  for (const line of ["1 person", "1 person; mouth open", "2 people; right hand raised", "12 people"]) {
    assert.equal(describesPerson(line), true, line);
  }
  for (const line of ["Nobody in view", "", "0 people", "1 personal note", "Hand raised", null, undefined]) {
    assert.equal(describesPerson(line), false, String(line));
  }
});

test("OBS's global audio devices are named Mic/Aux first, then Desktop Audio", () => {
  const special = {
    desktop1: "Desktop Audio",
    desktop2: null,
    mic1: "Mic/Aux",
    mic2: null,
    mic3: "Mic/Aux 3",
    mic4: null,
  };
  assert.deepEqual(globalDeviceNames(special), ["Mic/Aux", "Mic/Aux 3", "Desktop Audio"]);
  const none = { desktop1: null, desktop2: null, mic1: null, mic2: null, mic3: null, mic4: null };
  assert.deepEqual(globalDeviceNames(none), []);
});

describe("the editor's audio sources against OBS's global devices", () => {
  const source = (name, global) => ({ uuid: `uuid-${name}`, name, global, voice: false, in_use: false });
  const devices = ["Mic/Aux", "Desktop Audio"];
  const clip = source("Camera clip", false);

  test("the global devices come first, in order, once each, and marked global", () => {
    const listed = [source("Mic/Aux", true), source("Desktop Audio", true), clip];
    assert.equal(listsGlobalDevicesFirst(listed, devices), true);
    assert.equal(listsGlobalDevicesFirst([clip], []), true, "an OBS without audio devices");
  });

  test("a device missing, out of order, unmarked, listed again, or an extra global one is caught", () => {
    const check = (listed) => listsGlobalDevicesFirst(listed, devices);
    assert.equal(check([source("Mic/Aux", true), clip]), false, "missing");
    assert.equal(check([source("Desktop Audio", true), source("Mic/Aux", true), clip]), false, "out of order");
    assert.equal(check([source("Mic/Aux", false), source("Desktop Audio", true), clip]), false, "unmarked");
    const twice = [source("Mic/Aux", true), source("Desktop Audio", true), source("Mic/Aux", false)];
    assert.equal(check(twice), false, "listed again");
    assert.equal(check([...devices.map((name) => source(name, true)), source("Mic/Aux 2", true)]), false, "extra");
  });
});

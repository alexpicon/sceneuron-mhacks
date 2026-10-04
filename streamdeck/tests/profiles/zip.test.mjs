// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the ZIP writer the profiles use: entries read back as written, folders stored and
// files deflated, the same entries always making the same bytes, an entry that is neither a
// folder nor a file refused, and the system's unzip, where there is one, accepting the archive.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { zip } from "../../src/profiles/zip.ts";
import { readZip } from "./zip-reader.mjs";

const RUNS = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", ".test-runs");

const ENTRIES = [
  { path: "A.sdProfile/" },
  { path: "A.sdProfile/manifest.json", data: Buffer.from('{"Name":"Sceneuron ⏱️"}') },
  { path: "A.sdProfile/Profiles/" },
  { path: "A.sdProfile/Profiles/page.json", data: Buffer.from("x".repeat(500)) },
];

/** True when an `unzip` program can be run here. */
function hasUnzip() {
  try {
    execFileSync("unzip", ["-v"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

test("entries read back as written, folders stored and files deflated", () => {
  const entries = readZip(zip(ENTRIES));
  assert.deepEqual(
    entries.map((entry) => [entry.path, entry.method, entry.external]),
    [
      ["A.sdProfile/", 0, 0x10],
      ["A.sdProfile/manifest.json", 8, 0],
      ["A.sdProfile/Profiles/", 0, 0x10],
      ["A.sdProfile/Profiles/page.json", 8, 0],
    ],
  );
  assert.equal(entries[1].data.toString("utf8"), '{"Name":"Sceneuron ⏱️"}');
  assert.equal(entries[3].data.toString("utf8"), "x".repeat(500));
});

test("the same entries always make the same bytes", () => {
  assert.deepEqual(zip(ENTRIES), zip(ENTRIES.map((entry) => ({ ...entry }))));
  assert.notDeepEqual(zip(ENTRIES), zip(ENTRIES.slice(1)));
});

test("an entry that is neither a folder nor a file is refused", () => {
  assert.throws(() => zip([{ path: "folder" }]), /must be a folder ending in "\/" or a file with data/);
  assert.throws(() => zip([{ path: "folder/", data: Buffer.from("x") }]), /must be a folder/);
});

test("the system's unzip accepts the archive", { skip: hasUnzip() ? false : "no unzip here" }, () => {
  mkdirSync(RUNS, { recursive: true });
  const folder = mkdtempSync(path.join(RUNS, "zip-"));
  try {
    const file = path.join(folder, "test.streamDeckProfile");
    writeFileSync(file, zip(ENTRIES));
    const output = execFileSync("unzip", ["-t", file], { encoding: "utf8" });
    assert.match(output, /No errors detected/);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});

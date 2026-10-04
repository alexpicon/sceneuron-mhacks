// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises scripts/copyright.mjs: the comment syntax and exclusion each file gets, where the
// header goes after shebang, XML declaration, and doctype lines, that line endings and byte order
// marks survive, and the --check and --fix runs over a scratch git repository.

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";

import { HEADER, PROBLEMS, headerStatus, ruleFor, withHeader } from "../../scripts/copyright.mjs";

const SCRIPT = resolve(import.meta.dirname, "..", "..", "scripts", "copyright.mjs");
const [FIRST, SECOND] = HEADER;

/** The style `ruleFor` gives `file`, which must be covered. */
function styleOf(file) {
  const rule = ruleFor(file);
  assert.ok(rule?.style, `${file} is covered`);
  return rule.style;
}

test("each covered type gets its own comment syntax", () => {
  const firstLine = (file) => withHeader("", styleOf(file)).split("\n")[0];
  for (const file of ["crates/a/src/lib.rs", "web/app.js", "e2e/run.mjs", "streamdeck/src/a.ts", "e2e/linux/a.c"]) {
    assert.equal(firstLine(file), `// ${FIRST}`, file);
  }
  for (const file of ["Cargo.toml", "room/module/Cargo.toml", "a.toml", "scripts/a.sh", "e2e/a.sh"]) {
    assert.equal(firstLine(file), `# ${FIRST}`, file);
  }
  for (const file of [".gitignore", "streamdeck/.gitignore", ".gitattributes", "streamdeck/.npmrc", ".env.example"]) {
    assert.equal(firstLine(file), `# ${FIRST}`, file);
  }
  assert.equal(firstLine("web/editor/css/base.css"), `/* ${FIRST} */`);
  assert.equal(firstLine("web/editor.html"), `<!-- ${FIRST} -->`);
  assert.equal(firstLine("streamdeck/assets/marketplace.svg"), `<!-- ${FIRST} -->`);
});

test("the license, third-party, verbatim, lock, JSON, Markdown, and binary files are excluded", () => {
  for (const file of [
    "LICENSE",
    "crates/sceneuron-vision/models/LICENSE-ultraface.txt",
    "crates/sceneuron-vision/models/ultraface-rfb-320.onnx",
    "crates/sceneuron-vision/models/README.md",
    "web/fonts/AlcarinTengwarVF.woff2",
    "web/fonts/pIqaD-qolqoS.ttf",
    "web/fonts/nasin-nanpa-4.0.2-UCSUR.otf",
    "web/fonts/LICENSE-AlcarinTengwar.txt",
    "crates/sceneuron-ai/src/prompts/judge.txt",
    "Cargo.lock",
    "room/module/Cargo.lock",
    "streamdeck/package-lock.json",
    "streamdeck/package.json",
    "README.md",
    "docs/images/editor-cards.png",
    "crates/sceneuron-vision/tests/fixtures/face-320x240.rgb",
    "crates/sceneuron-voice/tests/fixtures/test-ca.der",
  ]) {
    assert.equal(typeof ruleFor(file)?.excluded, "string", file);
  }
});

test("a type without a rule is reported rather than skipped", () => {
  for (const file of [
    "notes.txt",
    "tool.py",
    "Makefile",
    "LICENSE-other",
    "a.env.example",
    "docs/LICENSE",
    "web/fonts/sub/a.ttf",
    "web/a.woff2",
  ]) {
    assert.equal(ruleFor(file), null, file);
  }
});

test("Rust gets plain comments above its inner docs and attributes", () => {
  const style = styleOf("lib.rs");
  assert.equal(
    withHeader("//! The crate.\n\nmod a;\n", style),
    `// ${FIRST}\n// ${SECOND}\n\n//! The crate.\n\nmod a;\n`,
  );
  assert.equal(withHeader("#![allow(dead_code)]\n", style), `// ${FIRST}\n// ${SECOND}\n\n#![allow(dead_code)]\n`);
});

test("the header follows a shebang, an XML declaration, or a doctype", () => {
  assert.equal(
    withHeader("#!/usr/bin/env bash\nset -eu\n", styleOf("a.sh")),
    `#!/usr/bin/env bash\n# ${FIRST}\n# ${SECOND}\n\nset -eu\n`,
  );
  assert.equal(
    withHeader('#!/usr/bin/env node\nimport "x";\n', styleOf("a.mjs")),
    `#!/usr/bin/env node\n// ${FIRST}\n// ${SECOND}\n\nimport "x";\n`,
  );
  assert.equal(
    withHeader('<?xml version="1.0" encoding="UTF-8"?>\n<svg/>\n', styleOf("a.svg")),
    `<?xml version="1.0" encoding="UTF-8"?>\n<!-- ${FIRST} -->\n<!-- ${SECOND} -->\n\n<svg/>\n`,
  );
  assert.equal(
    withHeader("<!doctype html>\n<html></html>\n", styleOf("a.html")),
    `<!doctype html>\n<!-- ${FIRST} -->\n<!-- ${SECOND} -->\n\n<html></html>\n`,
  );
});

test("one blank line separates the header, and an empty file gets just the header", () => {
  const style = styleOf("a.toml");
  assert.equal(withHeader("\n[a]\n", style), `# ${FIRST}\n# ${SECOND}\n\n[a]\n`);
  assert.equal(withHeader("[a]", style), `# ${FIRST}\n# ${SECOND}\n\n[a]`);
  assert.equal(withHeader("", style), `# ${FIRST}\n# ${SECOND}\n`);
});

test("line endings and a byte order mark are kept", () => {
  const style = styleOf("a.js");
  assert.equal(withHeader("a;\r\nb;\r\n", style), `// ${FIRST}\r\n// ${SECOND}\r\n\r\na;\r\nb;\r\n`);
  assert.equal(withHeader("\uFEFFa;\n", style), `\uFEFF// ${FIRST}\n// ${SECOND}\n\na;\n`);
});

test("the status tells a header at the top from a partial, lower, or differently commented one", () => {
  const style = styleOf("a.rs");
  for (const text of ["", "fn a() {}\n", "#![deny(x)]\n", "a\r\nb\r\n", "\uFEFFfn a() {}\n"]) {
    const headed = withHeader(text, style);
    assert.equal(headerStatus(headed, style), "ok", JSON.stringify(text));
  }
  assert.equal(headerStatus("fn a() {}\n", style), "missing");
  assert.equal(headerStatus(`# ${FIRST}\n# ${SECOND}\n`, style), "missing");
  assert.equal(headerStatus(`// ${FIRST}\n`, style), "misplaced");
  assert.equal(headerStatus(`//! Docs.\n// ${FIRST}\n// ${SECOND}\n`, style), "misplaced");
  assert.equal(headerStatus(`#!/bin/sh\n# ${FIRST}\n# ${SECOND}\n`, styleOf("a.sh")), "ok");
});

/** Runs the script in `cwd` with `args`. */
function copyright(cwd, ...args) {
  return spawnSync(process.execPath, [SCRIPT, ...args], { cwd, encoding: "utf8" });
}

/** A scratch git repository holding `files`, a map of paths to contents, all staged. */
function scratchRepo(files) {
  const repo = mkdtempSync(join(tmpdir(), "sceneuron-copyright-"));
  execFileSync("git", ["init", "-q"], { cwd: repo });
  for (const [file, text] of Object.entries(files)) {
    mkdirSync(dirname(join(repo, file)), { recursive: true });
    writeFileSync(join(repo, file), text);
  }
  execFileSync("git", ["add", "-A"], { cwd: repo });
  return repo;
}

test("--check lists what lacks the header, --fix adds it, and excluded files stay as they are", (t) => {
  const repo = scratchRepo({
    "a.rs": "//! A.\n",
    "web/b.js": "b();\n",
    "c.sh": "#!/bin/sh\necho c\n",
    "d.css": `/* ${FIRST} */\n/* ${SECOND} */\n\nd {}\n`,
    "README.md": "# Readme\n",
    "package.json": "{}\n",
  });
  t.after(() => rmSync(repo, { recursive: true, force: true }));
  chmodSync(join(repo, "c.sh"), 0o755);

  const checked = copyright(join(repo, "web"), "--check");
  assert.equal(checked.status, 1);
  const listed = checked.stderr.split("\n").filter((line) => line.endsWith(PROBLEMS.missing));
  assert.deepEqual(listed.sort(), ["a.rs", "c.sh", "web/b.js"].map((file) => `${file}: ${PROBLEMS.missing}`));
  assert.equal(readFileSync(join(repo, "a.rs"), "utf8"), "//! A.\n", "--check changes nothing");

  const fixed = copyright(repo, "--fix");
  assert.equal(fixed.status, 0, fixed.stderr);
  assert.equal(readFileSync(join(repo, "c.sh"), "utf8"), `#!/bin/sh\n# ${FIRST}\n# ${SECOND}\n\necho c\n`);
  assert.equal(statSync(join(repo, "c.sh")).mode & 0o777, 0o755, "the script stays executable");
  assert.equal(readFileSync(join(repo, "README.md"), "utf8"), "# Readme\n");
  assert.equal(readFileSync(join(repo, "package.json"), "utf8"), "{}\n");

  const rechecked = copyright(repo, "--check");
  assert.equal(rechecked.status, 0, rechecked.stderr);
  assert.equal(copyright(repo, "--fix").stdout.includes("added"), false, "a second --fix adds nothing");
});

test("a file without a rule or with a misplaced header fails both modes, and --fix leaves it", (t) => {
  const misplaced = `fn a() {}\n// ${FIRST}\n// ${SECOND}\n`;
  const repo = scratchRepo({ "notes.txt": "notes\n", "a.rs": misplaced });
  t.after(() => rmSync(repo, { recursive: true, force: true }));
  for (const mode of ["--check", "--fix"]) {
    const run = copyright(repo, mode);
    assert.equal(run.status, 1, mode);
    const lines = run.stderr.split("\n");
    assert.ok(lines.includes(`notes.txt: ${PROBLEMS.unknown}`), run.stderr);
    assert.ok(lines.includes(`a.rs: ${PROBLEMS.misplaced}`), run.stderr);
  }
  assert.equal(readFileSync(join(repo, "a.rs"), "utf8"), misplaced);
});

test("the script needs exactly one of --check and --fix", () => {
  const root = resolve(import.meta.dirname, "..", "..");
  for (const args of [[], ["--check", "--fix"]]) {
    const run = copyright(root, ...args);
    assert.equal(run.status, 2, args.join(" "));
    assert.match(run.stderr, /usage: node scripts\/copyright\.mjs --check \| --fix/);
  }
});

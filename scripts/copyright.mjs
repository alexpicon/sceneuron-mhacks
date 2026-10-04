// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Checks or adds the copyright header that every tracked source and text file starts with, in the
// comment syntax of its type and after any shebang, XML declaration, or doctype line. Every
// tracked file needs a rule: a comment style below, or an exclusion that says why it has no header.
//
// Usage: node scripts/copyright.mjs --check   lists the files that lack the header; exits 1 if any
//        node scripts/copyright.mjs --fix     adds the header to them
// Run it anywhere in the repository; it reads the files git tracks there.

import { execFileSync } from "node:child_process";
import { lstatSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

/** The header's lines, without comment syntax. */
export const HEADER = [
  "Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.",
  "Proprietary software; see LICENSE.",
];

const SHEBANG = /^#!(?!\[)/;

/** How each style comments a line, and the lines it keeps above the header. */
const STYLES = {
  slash: { open: "// ", close: "", prologue: SHEBANG },
  hash: { open: "# ", close: "", prologue: SHEBANG },
  block: { open: "/* ", close: " */", prologue: null },
  markup: { open: "<!-- ", close: " -->", prologue: /^(<\?xml\s.*\?>|<!doctype\s[^>]*>)\s*$/i },
};

/** The comment style of each covered file type, by file name or else by extension. */
const COVERED = {
  ".c": "slash",
  ".css": "block",
  ".env.example": "hash",
  ".gitattributes": "hash",
  ".gitignore": "hash",
  ".html": "markup",
  ".js": "slash",
  ".mjs": "slash",
  ".npmrc": "hash",
  ".rs": "slash",
  ".sh": "hash",
  ".svg": "markup",
  ".toml": "hash",
  ".ts": "slash",
};

/** Tracked files that do not carry the header, each with why; the first match wins. */
export const EXCLUDED = [
  [/^LICENSE$/, "the license itself"],
  [/^crates\/sceneuron-vision\/models\//, "the UltraFace model and its license, third-party under MIT"],
  [
    /^web\/fonts\/[^/]+\.(woff2|ttf|otf|txt)$/,
    "the overlay's fonts and their license texts, third-party under the SIL Open Font License or MIT",
  ],
  [/^crates\/sceneuron-ai\/src\/prompts\/[^/]+\.txt$/, "AI prompts, sent to the model verbatim"],
  [/(^|\/)(Cargo\.lock|package-lock\.json)$/, "lock files, written by Cargo and npm"],
  [/\.json$/, "JSON has no comments; the manifests that can name a license and author do"],
  // Once the documentation carries the notice, replace this with ".md": "markup" in COVERED.
  [/\.md$/, "Markdown, not covered yet: how the documentation carries the notice is undecided"],
  [/\.(der|onnx|png|rgb)$/, "binary files"],
];

/**
 * The rule for a repository path with `/` separators: `{ excluded: reason }`, `{ style }` with the
 * comment style it is checked with, or null when no rule covers it.
 */
export function ruleFor(file) {
  const exclusion = EXCLUDED.find(([pattern]) => pattern.test(file));
  if (exclusion) {
    return { excluded: exclusion[1] };
  }
  const name = path.posix.basename(file);
  const style = COVERED[name] ?? COVERED[path.posix.extname(name)];
  return style ? { style: STYLES[style] } : null;
}

/** The header as lines commented in `style`. */
export function headerLines(style) {
  return HEADER.map((line) => `${style.open}${line}${style.close}`);
}

/** Splits `text` into its byte order mark, line ending, and lines. */
function split(text) {
  const bom = text.startsWith("\uFEFF") ? "\uFEFF" : "";
  const eol = text.includes("\r\n") ? "\r\n" : "\n";
  return { bom, eol, lines: text.slice(bom.length).split(/\r?\n/) };
}

/** The index of the line the header starts at: the first after the prologue lines `style` keeps. */
function headerAt(lines, style) {
  let at = 0;
  while (style.prologue && at < lines.length && style.prologue.test(lines[at])) {
    at += 1;
  }
  return at;
}

/**
 * "ok" when `text` starts with the header; "misplaced" when its first line is elsewhere or the rest
 * is missing, which `--fix` leaves to a person; else "missing".
 */
export function headerStatus(text, style) {
  const { lines } = split(text);
  const at = headerAt(lines, style);
  const header = headerLines(style);
  if (header.every((line, index) => lines[at + index] === line)) {
    return "ok";
  }
  return lines.includes(header[0]) ? "misplaced" : "missing";
}

/** `text` with the header added at the top, followed by a blank line unless one is there already. */
export function withHeader(text, style) {
  const { bom, eol, lines } = split(text);
  const at = headerAt(lines, style);
  const rest = lines.slice(at);
  const gap = rest[0] === "" ? [] : [""];
  return bom + [...lines.slice(0, at), ...headerLines(style), ...gap, ...rest].join(eol);
}

/** The tracked regular files under `root`, as paths with `/` separators. */
function trackedFiles(root) {
  const listed = execFileSync("git", ["ls-files", "-z"], { cwd: root, encoding: "utf8" });
  return listed
    .split("\0")
    .filter(Boolean)
    .filter((file) => lstatSync(path.join(root, file), { throwIfNoEntry: false })?.isFile());
}

/** What each kind of problem `run` reports means. */
export const PROBLEMS = {
  missing: "lacks the copyright header",
  misplaced: "has a copyright header that is incomplete or not at the top; fix it by hand",
  unknown: "has no rule; add its type to COVERED or EXCLUDED in scripts/copyright.mjs",
};

/**
 * Checks every tracked file under `root`, and with `fix` adds the headers that are missing.
 * Returns the files fixed and the problems left, each `{ file, kind }` with a kind of `PROBLEMS`.
 */
export function run(root, { fix = false } = {}) {
  const fixed = [];
  const problems = [];
  for (const file of trackedFiles(root)) {
    const rule = ruleFor(file);
    if (!rule) {
      problems.push({ file, kind: "unknown" });
      continue;
    }
    if (rule.excluded) {
      continue;
    }
    const full = path.join(root, file);
    const text = readFileSync(full, "utf8");
    const status = headerStatus(text, rule.style);
    if (status === "missing" && fix) {
      writeFileSync(full, withHeader(text, rule.style));
      fixed.push(file);
    } else if (status !== "ok") {
      problems.push({ file, kind: status });
    }
  }
  return { fixed, problems };
}

function main() {
  const { values } = parseArgs({ options: { check: { type: "boolean" }, fix: { type: "boolean" } } });
  const fix = Boolean(values.fix);
  if (Boolean(values.check) === fix) {
    console.error("usage: node scripts/copyright.mjs --check | --fix");
    return 2;
  }
  const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
  const { fixed, problems } = run(root, { fix });
  for (const file of fixed) {
    console.log(`copyright: added the header to ${file}`);
  }
  for (const { file, kind } of problems) {
    console.error(`${file}: ${PROBLEMS[kind]}`);
  }
  if (problems.length === 0) {
    console.log("copyright: every tracked file has the header or an exclusion");
    return 0;
  }
  const fixable = problems.some(({ kind }) => kind === "missing");
  const hint = fixable ? "; node scripts/copyright.mjs --fix adds the missing headers" : "";
  console.error(`copyright: ${problems.length} files need attention${hint}`);
  return 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main();
}

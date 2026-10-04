// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A rollup plugin that writes the license of every npm package bundled into the plugin next to the
// bundle, so the packaged plugin carries the notices those licenses require.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";

/** The package directory that holds `file`, for files under node_modules; null otherwise. */
function packageRoot(file) {
  const parts = file.split(path.sep);
  const at = parts.lastIndexOf("node_modules");
  if (at < 0) {
    return null;
  }
  const length = parts[at + 1]?.startsWith("@") ? 3 : 2;
  return parts.slice(0, at + length).join(path.sep);
}

function licenseText(root) {
  const file = readdirSync(root).find((name) => /^(licen[cs]e|copying)(\..*)?$/i.test(name));
  return file ? readFileSync(path.join(root, file), "utf8").trim() : null;
}

/** Emits `fileName` listing each bundled package with its version, license, and license text. */
export function notices({ fileName = "third-party-notices.txt" } = {}) {
  return {
    name: "third-party-notices",
    generateBundle(_, bundle) {
      const roots = new Set();
      for (const chunk of Object.values(bundle)) {
        for (const id of chunk.type === "chunk" ? Object.keys(chunk.modules) : []) {
          const root = packageRoot(id.replace(/^\0/, "").split("?")[0]);
          if (root && existsSync(path.join(root, "package.json"))) {
            roots.add(root);
          }
        }
      }
      const sections = [...roots]
        .map((root) => ({ root, manifest: JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")) }))
        .sort((a, b) => a.manifest.name.localeCompare(b.manifest.name))
        .map(({ root, manifest }) => {
          const text = licenseText(root);
          if (!text) {
            this.error(`${manifest.name} has no license file to ship`);
          }
          return `${manifest.name} ${manifest.version} (${manifest.license})\n\n${text}`;
        });
      const header = "The Sceneuron Stream Deck plugin bundles these packages, under these licenses.";
      const source = `${[header, ...sections].join(`\n\n${"-".repeat(72)}\n\n`)}\n`;
      this.emitFile({ type: "asset", fileName, source });
    },
  };
}

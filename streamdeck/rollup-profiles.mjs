// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A rollup plugin that writes the bundled Stream Deck profiles into the plugin folder on every
// build, from the one layout in src/profiles/layout.ts and the actions in the plugin's manifest,
// so no profile is kept by hand. The manifest's "Profiles" entries name the files it writes.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { pluginInfo, profileFile } from "./src/profiles/profile.ts";

const LAYOUT = new URL("./src/profiles/layout.ts", import.meta.url);

/**
 * The layout as the file holds it now. Node keeps every module it has loaded, so a watch session
 * would otherwise go on writing the layout it read when the config first loaded; a fresh address
 * makes Node read the file again.
 */
async function readLayout() {
  return import(`${LAYOUT.href}?build=${Date.now()}`);
}

/** Writes each profile in the layout's `PROFILES` under `sdPlugin` once the bundle is written. */
export function profiles(sdPlugin) {
  return {
    name: "profiles",
    buildStart() {
      // The plugin bundle does not include the layout, so a watch session rebuilds on an edit of it
      // only when told to.
      this.addWatchFile(fileURLToPath(LAYOUT));
    },
    async writeBundle() {
      const { PAGES, PROFILES } = await readLayout();
      const plugin = pluginInfo(JSON.parse(readFileSync(path.join(sdPlugin, "manifest.json"), "utf8")));
      for (const profile of PROFILES) {
        const file = path.join(sdPlugin, `${profile.file}.streamDeckProfile`);
        mkdirSync(path.dirname(file), { recursive: true });
        writeFileSync(file, profileFile(profile, PAGES, plugin));
      }
    },
  };
}

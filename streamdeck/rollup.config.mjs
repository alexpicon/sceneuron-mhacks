// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import commonjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import typescript from "@rollup/plugin-typescript";
import path from "node:path";
import url from "node:url";

import { notices } from "./rollup-notices.mjs";
import { profiles } from "./rollup-profiles.mjs";

const isWatching = !!process.env.ROLLUP_WATCH;
const sdPlugin = "ai.chaski.sceneuron.sdPlugin";

/** @type {import("rollup").RollupOptions} */
export default {
  input: "src/plugin.ts",
  output: {
    file: `${sdPlugin}/bin/plugin.js`,
    format: "es",
    sourcemap: isWatching,
    sourcemapPathTransform: (relativeSourcePath, sourcemapPath) =>
      url.pathToFileURL(path.resolve(path.dirname(sourcemapPath), relativeSourcePath)).href,
  },
  plugins: [
    {
      name: "watch-externals",
      buildStart() {
        this.addWatchFile(`${sdPlugin}/manifest.json`);
      },
    },
    typescript({ mapRoot: isWatching ? "./" : undefined, noEmitOnError: !isWatching }),
    nodeResolve({ browser: false, exportConditions: ["node"], preferBuiltins: true }),
    commonjs(),
    notices(),
    profiles(sdPlugin),
    {
      name: "emit-module-package-file",
      generateBundle() {
        this.emitFile({ fileName: "package.json", source: `{ "type": "module" }`, type: "asset" });
      },
    },
  ],
};

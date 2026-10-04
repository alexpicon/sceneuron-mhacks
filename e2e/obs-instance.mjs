// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Launches a separate, isolated OBS instance for end-to-end tests (macOS).
//
// CFFIXED_USER_HOME points Foundation's home-directory lookups at a scratch directory, so the
// instance gets its own config, plugins, scene collections, and logs and never touches the
// user's real OBS setup. "--multi" lets it run beside an OBS the user already has open.

import { spawn, execFileSync } from "node:child_process";
import { cpSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const OBS_APP = "/Applications/OBS.app/Contents/MacOS/OBS";

export function obsConfigDir(home) {
  return join(home, "Library", "Application Support", "obs-studio");
}

/** Creates the isolated config: the plugin, obs-websocket without auth, and fixed editor settings. */
export function prepareHome({ home, pluginBundle, websocketPort, editorPort, token }) {
  const config = obsConfigDir(home);
  mkdirSync(join(config, "plugins"), { recursive: true });
  cpSync(pluginBundle, join(config, "plugins", "sceneuron.plugin"), { recursive: true });
  // LastVersion and FirstRun suppress the first-run auto-configuration wizard. The test needs no
  // camera, microphone, or screen access, so the macOS permissions dialog is marked as shown.
  writeFileSync(join(config, "global.ini"), "[General]\nLastVersion=537001986\nMacOSPermissionsDialogLastShown=1000\n");
  writeFileSync(join(config, "user.ini"), "[General]\nFirstRun=true\n");
  const websocket = join(config, "plugin_config", "obs-websocket");
  mkdirSync(websocket, { recursive: true });
  writeFileSync(
    join(websocket, "config.json"),
    JSON.stringify({
      alerts_enabled: false,
      auth_required: false,
      first_load: false,
      server_enabled: true,
      server_password: "",
      server_port: websocketPort,
    }),
  );
  const sceneuron = join(config, "plugin_config", "sceneuron");
  mkdirSync(sceneuron, { recursive: true });
  writeFileSync(join(sceneuron, "server.json"), JSON.stringify({ port: editorPort, token }));
}

/** Renders a looping test clip: a face for `faceSeconds`, then nobody for `emptySeconds`. */
export function makeClip({ workDir, fixturesDir, faceSeconds, emptySeconds }) {
  const toPng = (name) => {
    const output = join(workDir, `${name}.png`);
    execFileSync("ffmpeg", ["-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", "320x240",
      "-i", join(fixturesDir, `${name}-320x240.rgb`), output]);
    return output;
  };
  const face = toPng("face-close");
  const empty = toPng("empty");
  const clip = join(workDir, "presence-clip.mp4");
  execFileSync("ffmpeg", ["-loglevel", "error", "-y",
    "-loop", "1", "-t", String(faceSeconds), "-i", face,
    "-loop", "1", "-t", String(emptySeconds), "-i", empty,
    "-filter_complex", "[0][1]concat=n=2:v=1:a=0,scale=1280:720,format=yuv420p",
    "-r", "30", "-c:v", "libx264", "-pix_fmt", "yuv420p", clip]);
  return clip;
}

export class ObsInstance {
  #child;
  exited;

  constructor(home) {
    this.home = home;
  }

  start() {
    const args = ["--multi", "--disable-updater", "--disable-missing-files-check"];
    this.#child = spawn(OBS_APP, args, {
      cwd: "/Applications/OBS.app/Contents/MacOS",
      env: { ...process.env, CFFIXED_USER_HOME: this.home },
      stdio: "ignore",
    });
    this.exited = new Promise((resolve) => this.#child.once("exit", (code, signal) => resolve({ code, signal })));
    return this.#child.pid;
  }

  /** Asks OBS to quit like a user would; SIGTERM triggers a normal shutdown on macOS. */
  async stop({ timeoutMs = 30_000 } = {}) {
    if (!this.#child || this.#child.exitCode !== null) return { clean: true };
    this.#child.kill("SIGTERM");
    const timeout = new Promise((resolve) => setTimeout(() => resolve(null), timeoutMs));
    const result = await Promise.race([this.exited, timeout]);
    // A crash during shutdown still ends the process; only a normal exit counts as clean.
    if (result) return { clean: result.code === 0 && result.signal === null, ...result };
    this.#child.kill("SIGKILL");
    await this.exited;
    return { clean: false };
  }

  /** Text of the newest OBS log file of this instance. */
  latestLog() {
    const dir = join(obsConfigDir(this.home), "logs");
    const files = readdirSync(dir)
      .filter((name) => name.endsWith(".txt"))
      .map((name) => ({ name, time: statSync(join(dir, name)).mtimeMs }))
      .sort((a, b) => b.time - a.time);
    return files.length ? readFileSync(join(dir, files[0].name), "utf8") : "";
  }
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A fake Stream Deck app for the built plugin: it starts bin/plugin.js the way Stream Deck does,
// answers its registration and global settings requests over a WebSocket, sends it events, and
// records every message it sends back.

import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { WebSocketServer } from "ws";

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
export const PLUGIN_DIR = path.join(ROOT, "ai.chaski.sceneuron.sdPlugin");
export const BUNDLE = path.join(PLUGIN_DIR, "bin", "plugin.js");
const RUNS = path.join(ROOT, ".test-runs");

/**
 * A working folder for one plugin run, named like the plugin folder and holding its manifest, the
 * two things Stream Deck's SDK reads from it. The SDK writes its log there and rotates the older
 * logs at start, so plugins started side by side by parallel test files each need their own.
 */
function runFolder() {
  mkdirSync(RUNS, { recursive: true });
  const folder = path.join(mkdtempSync(path.join(RUNS, "run-")), path.basename(PLUGIN_DIR));
  mkdirSync(folder);
  copyFileSync(path.join(PLUGIN_DIR, "manifest.json"), path.join(folder, "manifest.json"));
  return folder;
}

const INFO = {
  application: {
    font: ".AppleSystemUIFont",
    language: "en",
    platform: "mac",
    platformVersion: "15.0",
    version: "7.4.2.22730",
  },
  colors: {
    buttonMouseOverBackgroundColor: "#464646FF",
    buttonPressedBackgroundColor: "#303030FF",
    buttonPressedBorderColor: "#646464FF",
    buttonPressedTextColor: "#969696FF",
    highlightColor: "#0078FFFF",
  },
  devicePixelRatio: 2,
  devices: [{ id: "deck", name: "Stream Deck +", size: { columns: 4, rows: 2 }, type: 7 }],
  plugin: { uuid: "ai.chaski.sceneuron", version: "0.4.0.0" },
};

export function bundleExists() {
  return existsSync(BUNDLE);
}

/**
 * Starts a fake Stream Deck holding `globals`, then the plugin connected to it. `nodeArgs` go to
 * Node before the plugin's path, such as an `--import` that watches the plugin. With `holdGlobals`,
 * answers to getGlobalSettings wait for `releaseGlobals()`, and then carry the settings as they
 * were when asked, like answers still on their way.
 */
export async function startStreamDeck(globals = {}, { nodeArgs = [], holdGlobals = false } = {}) {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  await new Promise((resolve) => server.once("listening", resolve));
  const state = { globals, messages: [], socket: null, output: "", held: holdGlobals ? [] : null };
  const answer = (id, settings) =>
    state.socket.send(JSON.stringify({ event: "didReceiveGlobalSettings", id, payload: { settings } }));
  server.on("connection", (socket) => {
    state.socket = socket;
    socket.on("message", (data) => {
      const message = JSON.parse(String(data));
      state.messages.push(message);
      if (message.event === "getGlobalSettings") {
        if (state.held) {
          state.held.push([message.id, state.globals]);
        } else {
          answer(message.id, state.globals);
        }
      } else if (message.event === "setGlobalSettings") {
        state.globals = message.payload;
      }
    });
  });

  const args = [...nodeArgs, BUNDLE, "-port", String(server.address().port), "-pluginUUID", "plugin-instance"];
  args.push("-registerEvent", "registerPlugin", "-info", JSON.stringify(INFO));
  const cwd = runFolder();
  const child = spawn(process.execPath, args, { cwd, stdio: ["ignore", "pipe", "pipe"] });
  child.stdout.on("data", (chunk) => (state.output += chunk));
  child.stderr.on("data", (chunk) => (state.output += chunk));
  const exited = () => child.exitCode !== null || child.signalCode !== null;

  const deck = {
    get globals() {
      return state.globals;
    },
    get messages() {
      return state.messages;
    },
    get output() {
      return state.output;
    },
    /** The text of each log this run of the plugin wrote. */
    logs() {
      const folder = path.join(cwd, "logs");
      return existsSync(folder) ? readdirSync(folder).map((file) => readFileSync(path.join(folder, file), "utf8")) : [];
    },
    /** Answers the getGlobalSettings requests held so far, and every one after. */
    releaseGlobals() {
      for (const [id, settings] of state.held ?? []) {
        answer(id, settings);
      }
      state.held = null;
    },
    /** Sends Stream Deck event `event` to the plugin. */
    send(event) {
      state.socket.send(JSON.stringify(event));
    },
    /** Sends `event`, then waits for a message that `match` accepts among those that follow it. */
    async after(event, match, what) {
      const from = state.messages.length;
      this.send(event);
      return this.next(match, what, from);
    },
    /** Waits for a message from the plugin that `match` accepts, counting from message `from`. */
    async next(match, what, from = 0) {
      const deadline = Date.now() + 5000;
      for (;;) {
        const found = state.messages.slice(from).find(match);
        if (found) {
          return found;
        }
        if (Date.now() > deadline || exited()) {
          throw new Error(`no ${what} from the plugin; it wrote:\n${state.output}`);
        }
        await new Promise((resolve) => setTimeout(resolve, 10));
      }
    },
    async stop() {
      child.kill();
      await new Promise((resolve) => (exited() ? resolve() : child.once("exit", resolve)));
      await new Promise((resolve) => server.close(resolve));
      rmSync(path.dirname(cwd), { recursive: true, force: true });
    },
  };
  try {
    await deck.next((message) => message.event === "registerPlugin", "registration");
  } catch (error) {
    await deck.stop();
    throw error;
  }
  return deck;
}

const AT = { column: 0, row: 0 };

/** A willAppear event for an action at `context`. */
export function appear(action, context, settings, controller = "Keypad") {
  const payload = { controller, coordinates: AT, isInMultiAction: false, settings, state: 0 };
  return { event: "willAppear", action: `ai.chaski.sceneuron.${action}`, context, device: "deck", payload };
}

/** A keyDown event for the key at `context`. */
export function keyDown(action, context, settings, state = 0) {
  const payload = { coordinates: AT, isInMultiAction: false, settings, state };
  return { event: "keyDown", action: `ai.chaski.sceneuron.${action}`, context, device: "deck", payload };
}

/** A keyUp event for the key at `context`. */
export function keyUp(action, context, settings, state = 0) {
  const payload = { coordinates: AT, isInMultiAction: false, settings, state };
  return { event: "keyUp", action: `ai.chaski.sceneuron.${action}`, context, device: "deck", payload };
}

/** A willDisappear event for an action at `context`, as when its page changes or it is removed. */
export function disappear(action, context, settings, controller = "Keypad") {
  const payload = { controller, coordinates: AT, isInMultiAction: false, settings, state: 0 };
  return { event: "willDisappear", action: `ai.chaski.sceneuron.${action}`, context, device: "deck", payload };
}

/** A deviceDidDisconnect event for the fake's one device, as when it is unplugged. */
export function deviceDisconnect() {
  return { event: "deviceDidDisconnect", device: "deck" };
}

const LINK_PREFIX = "streamdeck://plugins/message/ai.chaski.sceneuron";

/**
 * The didReceiveDeepLink event for deep link `link`: Stream Deck routes a link to the plugin whose UUID follows
 * `streamdeck://plugins/message/`, and hands it over without that prefix.
 */
export function deepLink(link) {
  if (!link.startsWith(`${LINK_PREFIX}/`)) {
    throw new Error(`${link} is not a deep link to this plugin`);
  }
  return { event: "didReceiveDeepLink", payload: { url: link.slice(LINK_PREFIX.length) } };
}

/** A dial event (`dialRotate`, `dialDown`, or `touchTap`) for the dial at `context`, with `extra` in its payload. */
export function dialEvent(event, context, settings, extra = {}) {
  const payload = { controller: "Encoder", coordinates: AT, settings, ...extra };
  return { event, action: "ai.chaski.sceneuron.dial", context, device: "deck", payload };
}

/** The SVG inside a base64 data URL. */
export function svgOf(dataUrl) {
  return Buffer.from(dataUrl.replace(/^data:image\/svg\+xml;base64,/, ""), "base64").toString("utf8");
}

/** Matches a key image at `context` whose SVG passes `check`. */
export function image(context, check = () => true) {
  return (message) =>
    message.event === "setImage" && message.context === context && check(svgOf(message.payload.image));
}

/** Matches a touch strip drawing at `context` whose SVG passes `check`. */
export function strip(context, check = () => true) {
  return (message) =>
    message.event === "setFeedback" && message.context === context && check(svgOf(message.payload.card));
}

/** Matches the alert sign on `context`. */
export function alert(context) {
  return (message) => message.event === "showAlert" && message.context === context;
}

/** The text a drawing shows, its lines joined by spaces, as a title fitted on several lines reads. */
export function words(svg) {
  return [...svg.matchAll(/>([^<]+)<\/text>/g)].map(([, line]) => line).join(" ");
}

/** True for a drawing lit amber, as a raised card or a shown element is. */
export function lit(svg) {
  return svg.includes('rx="16" fill="#d6a76c"/>');
}

/** True for a drawing darkened because its control cannot act. */
export function dim(svg) {
  return svg.includes('fill="#1c1c1e" fill-opacity="0.6"');
}

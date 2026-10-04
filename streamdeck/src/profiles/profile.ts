// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * A `.streamDeckProfile` for one device, in the format of the default profiles the Stream Deck app
 * itself ships for the 15-key Stream Deck and the XL (profile format 2.0, which Stream Deck 7.1
 * and newer import and migrate): a ZIP holding one `<UUID>.sdProfile` folder with the profile's
 * manifest, and under `Profiles/` a folder per page, named after the page's UUID, with the page's
 * manifest. The manifest lists the pages and a `Default` page that is empty. Each page lists its
 * keys by "column,row", counting from 0 at the top left.
 *
 * Elgato documents only exporting a profile from the app and naming it in the plugin's manifest;
 * the layout of the file follows the profiles the app ships. Every UUID in it comes from a hash of
 * the profile and the layout, so the same layout always makes the same file, and a changed layout
 * gets new UUIDs rather than those of a profile already installed.
 */

import { createHash } from "node:crypto";

import type { DeviceProfile, LayoutKey, LayoutPage } from "./layout.ts";
import { type ZipEntry, zip } from "./zip.ts";

/** What a profile needs from the plugin's manifest. */
export interface PluginInfo {
  readonly uuid: string;
  readonly name: string;
  readonly version: string;
  /** Each action's name and number of states, by UUID. */
  readonly actions: ReadonlyMap<string, { readonly name: string; readonly states: number }>;
}

/** Stream Deck's own page actions, from the app's Pages plugin. */
const PAGES_PLUGIN = { Name: "Pages", UUID: "com.elgato.streamdeck.page", Version: "1.0" };
const GO_TO_PAGE = "com.elgato.streamdeck.page.goto";
const PAGE_INDICATOR = "com.elgato.streamdeck.page.indicator";

/** How the app's own profiles title a key, here along its bottom. */
const TITLE = {
  FontFamily: "",
  FontSize: 12,
  FontStyle: "",
  FontUnderline: false,
  OutlineThickness: 2,
  ShowTitle: true,
  TitleAlignment: "bottom",
  TitleColor: "#ffffff",
};

/** The digits of a page folder's name: 0-9 and A-W without U, five bits each. */
const FOLDER_DIGITS = "0123456789ABCDEFGHIJKLMNOPQRSTVW";

type Json = null | boolean | number | string | readonly Json[] | { readonly [key: string]: Json };

/** The plugin's name, version, and actions, read from its manifest. */
export function pluginInfo(manifest: unknown): PluginInfo {
  const fields = manifest as { UUID: string; Name: string; Version: string; Actions: unknown[] };
  const actions = new Map<string, { name: string; states: number }>();
  for (const entry of fields.Actions as { UUID: string; Name: string; States: unknown[] }[]) {
    actions.set(entry.UUID, { name: entry.Name, states: entry.States.length });
  }
  return { uuid: fields.UUID, name: fields.Name, version: fields.Version, actions };
}

/**
 * The folder a page lives in: its UUID's 128 bits five at a time, first bit first and the last
 * group padded with zeros, in `FOLDER_DIGITS`, then "Z".
 */
export function pageFolder(uuid: string): string {
  const bytes = [...Buffer.from(uuid.replace(/-/g, ""), "hex")];
  const bits = bytes.map((byte) => byte.toString(2).padStart(8, "0")).join("");
  const groups = bits.padEnd(Math.ceil(bits.length / 5) * 5, "0").match(/.{5}/g) ?? [];
  return `${groups.map((group) => FOLDER_DIGITS[Number.parseInt(group, 2)]).join("")}Z`;
}

/** A version 4 UUID made from the hash of `seed`. */
export function hashedUuid(seed: string): string {
  const bytes = createHash("sha256").update(seed).digest().subarray(0, 16);
  bytes[6] = (bytes[6]! & 0x0f) | 0x40;
  bytes[8] = (bytes[8]! & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return [hex.slice(0, 8), hex.slice(8, 12), hex.slice(12, 16), hex.slice(16, 20), hex.slice(20)].join("-");
}

/** The settings a Sceneuron key is saved with; Stream Deck's page keys carry their own. */
function settingsOf(key: LayoutKey): Json {
  switch (key.kind) {
    case "card":
      return { deck: key.deck, card: key.card };
    case "lower":
      return { deck: key.deck };
    case "reaction":
      return { reaction: key.reaction };
    default:
      return {};
  }
}

function action(key: LayoutKey, id: string, plugin: PluginInfo): Json {
  if (key.kind === "go-to-page" || key.kind === "page-indicator") {
    const goTo = key.kind === "go-to-page";
    return {
      ActionID: id,
      LinkedTitle: true,
      Name: goTo ? "Go to Page" : "Page Indicator",
      Plugin: PAGES_PLUGIN,
      Resources: null,
      Settings: goTo ? { PageIndex: key.page } : {},
      State: 0,
      States: goTo ? [{ ...TITLE, Title: key.title }] : [{}],
      UUID: goTo ? GO_TO_PAGE : PAGE_INDICATOR,
    };
  }
  const uuid = `${plugin.uuid}.${key.kind}`;
  const found = plugin.actions.get(uuid);
  if (!found) {
    throw new Error(`the layout has a ${key.kind} key, but the plugin's manifest has no ${uuid}`);
  }
  return {
    ActionID: id,
    LinkedTitle: true,
    Name: found.name,
    Plugin: { Name: plugin.name, UUID: plugin.uuid, Version: plugin.version },
    Resources: null,
    Settings: settingsOf(key),
    State: 0,
    States: Array.from({ length: found.states }, () => ({})),
    UUID: uuid,
  };
}

function pageManifest(page: LayoutPage | null, uuid: (name: string) => string, plugin: PluginInfo): Json {
  const actions: Record<string, Json> = {};
  page?.rows.forEach((keys, row) =>
    keys.forEach((key, column) => {
      if (key) {
        actions[`${column},${row}`] = action(key, uuid(`${page.name}/${column},${row}`), plugin);
      }
    }),
  );
  return { Controllers: [{ Actions: actions, Type: "Keypad" }], Icon: "", Name: page?.name ?? "" };
}

function json(value: Json): Buffer {
  return Buffer.from(JSON.stringify(value), "utf8");
}

/** The files of `profile` holding `pages`, as ZIP entries in the order the app's own profiles have. */
export function profileEntries(profile: DeviceProfile, pages: readonly LayoutPage[], plugin: PluginInfo): ZipEntry[] {
  const fits = (page: LayoutPage) =>
    page.rows.length <= profile.rows && page.rows.every((keys) => keys.length <= profile.columns);
  if (!pages.every(fits)) {
    throw new Error(`the layout does not fit the ${profile.columns} by ${profile.rows} keys of ${profile.file}`);
  }
  const salt = createHash("sha256").update(JSON.stringify({ profile, pages })).digest("hex");
  const uuid = (name: string) => hashedUuid(`${salt}/${name}`);
  const pageIds = pages.map((page) => uuid(`page/${page.name}`));
  const defaultId = uuid("default page");
  const root = `${uuid("profile").toUpperCase()}.sdProfile/`;
  const manifest = {
    Device: { Model: profile.model, UUID: "" },
    Name: profile.name,
    Pages: { Current: pageIds[0] ?? defaultId, Default: defaultId, Pages: pageIds },
    Version: "2.0",
  };
  const folders: [string, Json][] = [
    ...pages.map((page, index): [string, Json] => [pageIds[index]!, pageManifest(page, uuid, plugin)]),
    [defaultId, pageManifest(null, uuid, plugin)],
  ];
  return [
    { path: root },
    { path: `${root}Images/` },
    { path: `${root}manifest.json`, data: json(manifest) },
    { path: `${root}Profiles/` },
    ...folders.flatMap(([id, page]) => {
      const folder = `${root}Profiles/${pageFolder(id)}/`;
      return [{ path: folder }, { path: `${folder}Images/` }, { path: `${folder}manifest.json`, data: json(page) }];
    }),
  ];
}

/** `profile` as the bytes of its `.streamDeckProfile` file. */
export function profileFile(profile: DeviceProfile, pages: readonly LayoutPage[], plugin: PluginInfo): Buffer {
  return zip(profileEntries(profile, pages, plugin));
}

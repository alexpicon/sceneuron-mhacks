// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * What a Reaction key shows: its preset's emoji and short name, from the plugin's copy of the presets, so it looks
 * the same before the plugin is paired; lit and saying "Playing" while the preset plays on the overlay.
 */

import { BUILT_IN_REACTIONS, type BuiltInReaction } from "./built-in-reactions.ts";
import { type Reach, control, status } from "./faces.ts";
import { ICONS } from "./icons.ts";
import type { ReactionSettings } from "./settings.ts";
import type { KeyFace } from "./svg.ts";

/** The preset with id `id`. */
export function findReaction(id: string | undefined): BuiltInReaction | undefined {
  return id ? BUILT_IN_REACTIONS.find((preset) => preset.id === id) : undefined;
}

/**
 * A Reaction key for `settings` while the overlay plays the preset `playing`, or none. Like a card before any view,
 * it is drawn in full until the plugin is paired, and darkened while Sceneuron is out of reach or turned the plugin
 * away.
 */
export function reactionFace(reach: Reach, settings: ReactionSettings, playing: string | null): KeyFace {
  if (!settings.reaction) {
    return control("Choose a reaction", ICONS.reaction, reach);
  }
  const preset = findReaction(settings.reaction);
  if (!preset) {
    return { ...control(settings.reaction, ICONS.reaction, reach, "Not found"), dim: true };
  }
  const lit = reach === "live" && playing === preset.id;
  return {
    title: preset.short,
    emoji: preset.emojis[0],
    ...status(reach, lit ? "Playing" : undefined),
    lit,
    dim: reach === "offline" || reach === "refused",
  };
}

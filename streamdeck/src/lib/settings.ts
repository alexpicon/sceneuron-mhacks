// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Settings as Stream Deck stores them. Action settings travel with exported profiles, so they
 * only name decks, cards, elements, and reaction presets; the token lives in the plugin's global
 * settings, which Stream Deck keeps on this computer.
 */

/** The port Sceneuron's editor listens on unless the user moved it. */
export const DEFAULT_PORT = 4466;

export interface GlobalSettings {
  readonly port: number;
  readonly token: string | null;
}

export interface CardSettings {
  readonly deck?: string;
  readonly card?: string;
}

/** No deck means every deck. */
export interface LowerSettings {
  readonly deck?: string;
}

export interface ElementSettings {
  readonly element?: string;
}

export interface DialSettings {
  readonly deck?: string;
}

/** The reaction preset a key plays, by its id, such as "email". */
export interface ReactionSettings {
  readonly reaction?: string;
}

const TOKEN = /^[A-Za-z0-9._~-]{16,128}$/;
const CODE = /^\d{6}$/;

/** True for a whole number from 1 to 65535. */
export function isPort(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 65535;
}

/** True for a pairing code: exactly six digits. */
export function isPairingCode(value: unknown): value is string {
  return typeof value === "string" && CODE.test(value);
}

/**
 * True for a value shaped like a Sceneuron token: 16-128 characters of the URL-safe alphabet the
 * server accepts, A-Z, a-z, 0-9, '-', '_', '.', and '~'.
 */
export function isToken(value: unknown): value is string {
  return typeof value === "string" && TOKEN.test(value);
}

function record(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function id(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

export function readGlobalSettings(value: unknown): GlobalSettings {
  const fields = record(value);
  return {
    port: isPort(fields.port) ? fields.port : DEFAULT_PORT,
    token: isToken(fields.token) ? fields.token : null,
  };
}

export function readCardSettings(value: unknown): CardSettings {
  const fields = record(value);
  return { deck: id(fields.deck), card: id(fields.card) };
}

export function readLowerSettings(value: unknown): LowerSettings {
  return { deck: id(record(value).deck) };
}

export function readElementSettings(value: unknown): ElementSettings {
  return { element: id(record(value).element) };
}

export function readDialSettings(value: unknown): DialSettings {
  return { deck: id(record(value).deck) };
}

export function readReactionSettings(value: unknown): ReactionSettings {
  return { reaction: id(record(value).reaction) };
}

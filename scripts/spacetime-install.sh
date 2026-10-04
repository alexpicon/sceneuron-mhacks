#!/usr/bin/env bash
# Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
# Proprietary software; see LICENSE.

# Installs what the shared planning poker room needs on a Mac: the spacetime CLI at the version
# room/module is built against, and Rust's wasm32 target, which publishing the module compiles for.
# Safe to run again: it skips what is already there. Runs on macOS (bash 3.2+).
#
# Usage: scripts/spacetime-install.sh
set -euo pipefail

readonly VERSION="2.10.2"
readonly SPACETIME="$HOME/.local/bin/spacetime"

say() { printf '%s\n' "$*"; }
fail() { printf 'spacetime-install: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fail "this script is for macOS"

# The module builds with rustup's toolchain; Homebrew's Rust has no wasm32 standard library.
if ! command -v rustup >/dev/null 2>&1; then
  fail "rustup is not installed; install it from https://rustup.rs, then run this again"
fi

if [ -x "$SPACETIME" ]; then
  say "spacetime CLI found at $SPACETIME"
else
  say "Installing the spacetime CLI with SpacetimeDB's official installer"
  curl -sSf https://install.spacetimedb.com | sh -s -- --yes
  [ -x "$SPACETIME" ] || fail "the installer did not put spacetime at $SPACETIME"
fi

if "$SPACETIME" version list 2>/dev/null | grep -q "^$VERSION (current)"; then
  say "SpacetimeDB $VERSION is the current version"
else
  say "Installing SpacetimeDB $VERSION and making it current"
  "$SPACETIME" version install "$VERSION" --use --yes
fi

if rustup target list --installed | grep -qx "wasm32-unknown-unknown"; then
  say "Rust's wasm32-unknown-unknown target is installed"
else
  say "Adding Rust's wasm32-unknown-unknown target"
  rustup target add wasm32-unknown-unknown
fi

case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *) say "Note: $HOME/.local/bin is not on your PATH; scripts/spacetime.sh finds spacetime anyway." ;;
esac
say "Done. Start the room server with: scripts/spacetime.sh start"

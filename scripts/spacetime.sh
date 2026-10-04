#!/usr/bin/env bash
# Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
# Proprietary software; see LICENSE.

# Runs the local SpacetimeDB server for the shared planning poker room, in the background, and
# publishes room/module to it, so the editor's Room section works with its prefilled address
# (http://127.0.0.1:3000) and database (sceneuron-room). Runs on macOS (bash 3.2+); install the
# CLI first with scripts/spacetime-install.sh.
#
# Usage: scripts/spacetime.sh start|restart|stop|status|publish|log
#   start    start the server unless it is running, then publish the module
#   restart  stop the server, start it again, and publish the module
#   stop     stop the server this script started
#   status   say whether the server answers
#   publish  publish room/module again, as after changing it
#   log      follow the server's log
# Environment: SPACETIME_LISTEN (default 127.0.0.1:3000; 0.0.0.0:3000 lets other computers on a
# trusted network join), SPACETIME (the CLI, default ~/.local/bin/spacetime).
set -euo pipefail

readonly LISTEN="${SPACETIME_LISTEN:-127.0.0.1:3000}"
readonly SPACETIME="${SPACETIME:-$HOME/.local/bin/spacetime}"
readonly DATABASE="sceneuron-room"
readonly STATE_DIR="$HOME/.local/share/spacetime/sceneuron"
readonly PID_FILE="$STATE_DIR/server.pid"
readonly LOG_FILE="$STATE_DIR/server.log"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
readonly REPO
readonly PORT="${LISTEN##*:}"
readonly URL="http://127.0.0.1:$PORT"

say() { printf '%s\n' "$*"; }
fail() { printf 'spacetime.sh: %s\n' "$*" >&2; exit 1; }

[ -x "$SPACETIME" ] || fail "no spacetime CLI at $SPACETIME; run scripts/spacetime-install.sh first"
mkdir -p "$STATE_DIR"

answers() { curl -s -o /dev/null --max-time 2 "$URL/v1/ping"; }

running_pid() {
  [ -f "$PID_FILE" ] || return 1
  local pid
  pid="$(cat "$PID_FILE")"
  kill -0 "$pid" 2>/dev/null && printf '%s' "$pid"
}

publish() {
  say "Publishing room/module as $DATABASE (the first build takes a minute)"
  # Build with rustup's toolchain first on the PATH: Homebrew's Rust has no wasm32 standard library.
  local toolchain
  toolchain="$(dirname "$(dirname "$(rustup which cargo)")")"
  (cd "$REPO" && PATH="$toolchain/bin:$PATH" DYLD_FALLBACK_LIBRARY_PATH="$toolchain/lib" \
    "$SPACETIME" publish --server "$URL" --module-path room/module --yes=skip-login "$DATABASE")
}

start() {
  if answers; then
    say "SpacetimeDB already answers at $URL"
  else
    if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
      fail "port $PORT is taken by another program; free it or set SPACETIME_LISTEN"
    fi
    say "Starting SpacetimeDB on $LISTEN (log: $LOG_FILE)"
    nohup "$SPACETIME" start --listen-addr "$LISTEN" >>"$LOG_FILE" 2>&1 &
    printf '%s' "$!" >"$PID_FILE"
    local tries=0
    until answers; do
      tries=$((tries + 1))
      [ "$tries" -le 60 ] || fail "SpacetimeDB did not answer in 30 s; see $LOG_FILE"
      running_pid >/dev/null || fail "SpacetimeDB stopped while starting; see $LOG_FILE"
      sleep 0.5
    done
    say "SpacetimeDB answers at $URL"
  fi
  publish
  say "Ready: in the editor's Cards tab, Room section, create a room with $URL and $DATABASE."
}

stop() {
  local pid
  if pid="$(running_pid)"; then
    say "Stopping SpacetimeDB (pid $pid)"
    kill "$pid"
    local tries=0
    while kill -0 "$pid" 2>/dev/null; do
      tries=$((tries + 1))
      if [ "$tries" -gt 20 ]; then
        kill -9 "$pid" 2>/dev/null || true
        break
      fi
      sleep 0.5
    done
    rm -f "$PID_FILE"
  elif answers; then
    fail "a SpacetimeDB this script did not start answers at $URL; stop it where it runs"
  else
    say "SpacetimeDB is not running"
  fi
}

case "${1:-}" in
  start) start ;;
  restart) stop; start ;;
  stop) stop ;;
  status)
    if answers; then say "SpacetimeDB answers at $URL"; else say "SpacetimeDB is not running at $URL"; fi
    ;;
  publish)
    answers || fail "SpacetimeDB does not answer at $URL; run: scripts/spacetime.sh start"
    publish
    ;;
  log) tail -f "$LOG_FILE" ;;
  *) fail "usage: scripts/spacetime.sh start|restart|stop|status|publish|log" ;;
esac

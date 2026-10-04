# Room module

The SpacetimeDB module behind Sceneuron's shared planning poker room. SpacetimeDB runs it as
WebAssembly inside the database; it is not linked into the OBS plugin or `sceneuron-dev`. Each
Sceneuron reaches it through its own client of SpacetimeDB's documented WebSocket API, in
`crates/sceneuron-room`.

## What it holds

| Table | Clients can read it | Contents |
| --- | --- | --- |
| `room` | Yes | Each room by its six-character code, the deck it was created for, the round, and whether the round is revealed. |
| `participant` | Yes | Who is in which room, by SpacetimeDB identity, with a display name. |
| `ballot` | Yes | That someone voted this round, never what. |
| `vote` | No | What each person voted this round. |
| `result` | Yes | This round's votes, copied from `vote` by the reveal all at once. |

The reducers `create_room` and `join_room` put the caller in a room, leaving any other. The
reducers `cast_vote`, `reveal`, `next_round`, and `leave_room` act on the caller's own room, found
from the caller's identity, so nobody can vote, reveal, or start a round in a room they are not
in. A vote after the reveal is refused. `client_disconnected` takes a
connection that ends out of its room, and a room with nobody left is deleted. `src/rules.rs` holds
the checks for room codes, names, and card ids, which need no database.

## License

The module's own code is part of Sceneuron, which is proprietary software of Alexander Picon and
Chaski AI; see [LICENSE](../../LICENSE). `Cargo.toml` names them
as its authors and `LICENSE` as its license file.

A SpacetimeDB module has to be built against SpacetimeDB's module crates: the `spacetimedb` crate
gives the table and reducer macros and the calls into the host. Those crates come under their own
license, not Sceneuron's. For version 2.10.2, which `Cargo.toml` pins, `spacetimedb` and the six
crates it brings in (`spacetimedb-bindings-macro`, `-bindings-sys`, `-lib`, `-primitives`,
`-query-builder`, and `-sats`) each ship the Apache License 2.0 as their license file. Check them
again when the version changes: other SpacetimeDB crates, such as the client SDK, ship the
Business Source License 1.1.

SpacetimeDB itself, the server that runs the module, is under the Business Source License 1.1,
which is not an open source license. Its additional use grant allows production use with no more
than one SpacetimeDB instance, and not as a Database Service, a commercial offering that lets
others create tables whose schemas they control. Sceneuron's rooms are meant to stay inside that
grant: one server for the rooms, and no tables that anyone else defines.

The OBS plugin links libobs, which is GPL-2.0-or-later, so its binary is for Chaski AI's own use
and demos, and it is not distributed until the plugin is restructured into a thin open OBS bridge
and a proprietary app that talk over a local socket, an item in
[docs/backlog.md](../../docs/backlog.md). The license gate still requires every crate built into the
plugin or `sceneuron-dev` to have a permissive license, which suits both proprietary use and the GPL
plugin. SpacetimeDB's client SDK does not pass it, so Sceneuron has its own client, written from
SpacetimeDB's public docs, and no SpacetimeDB code is in the plugin or `sceneuron-dev`. This module
is not part of the plugin: it is not a member of the Cargo workspace, has its own `Cargo.lock`, and
is reached only over the network, so the license gate, which reads the workspace, never sees it.

## Build and test

Building the module needs Rust 1.96 or newer, which the `spacetimedb` 2.10.2 crates ask for (the
workspace needs only 1.91), and Rust's `wasm32-unknown-unknown` target through rustup. The tables
and reducers call into the SpacetimeDB host, so they compile only for wasm32; on the build machine,
`cargo test` runs the 3 tests of the rules. From the repository root, run these checks before
committing:

```bash
cargo fmt --manifest-path room/module/Cargo.toml --check
cargo test --locked --manifest-path room/module/Cargo.toml
cargo clippy --locked --manifest-path room/module/Cargo.toml --target wasm32-unknown-unknown -- -D warnings
```

## Publish it locally

Install the spacetime CLI 2.10.2 as [docs/development.md](../../docs/development.md) says. Start a
server in one terminal, then publish from the repository root in another:

```bash
spacetime start --listen-addr 127.0.0.1:3000
spacetime publish --server local --module-path room/module --yes=skip-login sceneuron-room
```

Publishing builds the module for wasm32 with the `cargo` first on the `PATH`. When that is not
rustup's, such as Homebrew's Rust, which has no wasm32 standard library, the development guide
shows how to put rustup's toolchain first. The editor's Room section is prefilled with
`http://127.0.0.1:3000` and `sceneuron-room`. Publishing to Maincloud needs `spacetime login`
first.

## How the live test publishes it

`e2e/room-live.mjs`, through `e2e/spacetime-server.mjs`, runs `spacetime start` on a free port on
127.0.0.1 with its data directory, signing keys, and CLI config inside the run's work directory, so
the SpacetimeDB install on the computer is never read or changed. It then runs
`spacetime publish --server <that address> --module-path room/module --yes sceneuron-room` with
that config and with the toolchain rustup selects first on the `PATH`. A first run spends a minute
or two building the module. The test then checks the reducers from two Sceneurons and from a
client outside Sceneuron, including that no vote can be read before the reveal; with starting the
server and publishing, it has 17 steps. See [e2e/README.md](../../e2e/README.md).

## Changing it

Nothing is generated from the module. After changing its tables or reducers, update by hand the
tables' field order and names in `crates/sceneuron-room/src/spacetime/cache.rs`, the reducers and
their arguments in `crates/sceneuron-room/src/spacetime/mod.rs`, the fake rooms in
`crates/sceneuron-room/tests/common`, and the browser test's stand-in, `e2e/room-stand-in.mjs`.

Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved. Proprietary software; see [LICENSE](../../LICENSE).

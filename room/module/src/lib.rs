// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The SpacetimeDB module behind Sceneuron's shared planning poker room.
//!
//! Everyone in a call joins one room. A vote is recorded in a private table, so others see only
//! that someone voted; revealing copies the round's votes into a public table at once, and the
//! next round clears them. Connections that end leave their room, and empty rooms are deleted.
//!
//! Tables and reducers call into the SpacetimeDB host, which only exists for wasm32, so they are
//! compiled for wasm32 alone; `cargo test` on the build machine checks the rules they share.

#[cfg(target_arch = "wasm32")]
mod reducers;
pub mod rules;
#[cfg(target_arch = "wasm32")]
mod tables;

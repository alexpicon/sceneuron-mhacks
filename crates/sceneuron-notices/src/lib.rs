// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Third-party license notices for Sceneuron packages: the crates linked into the plugin on
//! each target, grouped by identical license text, plus the bundled face model's license and the
//! bundled fonts' licenses; and the license gate, which refuses a crate whose license the plugin
//! cannot ship with.

mod fonts;
mod gate;
mod graph;
mod render;

pub use fonts::{FONTS, font_notices};
pub use gate::{ALLOWED, Refusal, allows, refusals};
pub use graph::{Package, shipped_packages};
pub use render::{Extra, borrow_from_siblings, ijg_credit, license_texts, render};

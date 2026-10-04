// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Ready-made card decks: meeting signals, planning poker, classroom signals, and a status banner;
//! and two bars without a deck, for live subtitles and for the live description of what the
//! camera sees.
//!
//! Packs are authored for a 1920x1080 canvas, and [`pack_for_canvas`] fits one to a project's
//! canvas. Element and deck ids are namespaced by the pack, so packs combine in one project.

mod catalog;
mod live_description;
mod parts;

use serde::{Deserialize, Serialize};

use crate::decks::Deck;
use crate::model::{Canvas, Element, Project, ValidationError};
use crate::props::{ElementProps, PropsPatch};

/// Canvas width and height the packs are authored for.
const AUTHORED: (f64, f64) = (1920.0, 1080.0);

/// The ranges [`ElementProps::validate`] accepts for the lengths a pack scales.
const POSITION: (f64, f64) = (-10_000.0, 10_000.0);
const SIZE: (f64, f64) = (1.0, 10_000.0);
const FONT_SIZE: (f64, f64) = (1.0, 1_000.0);
const BORDER_WIDTH: (f64, f64) = (0.0, 200.0);
const CORNER_RADIUS: (f64, f64) = (0.0, 5_000.0);

/// Card slot elements and the decks they show, added to a project together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub elements: Vec<Element>,
    pub decks: Vec<Deck>,
}

/// Every pack, as authored for a 1920x1080 canvas.
pub fn packs() -> Vec<Pack> {
    catalog::packs()
}

/// One pack fitted to `canvas`, or `None` for an unknown id.
///
/// Sizes, font sizes, border widths, and corner radii of elements and cards scale by the smaller
/// of the canvas width over 1920 and the canvas height over 1080, and each element keeps its
/// center at the same fraction of the canvas. On a 16:9 canvas that is plain scaling; on any
/// other shape every slot stays inside the canvas and slots keep the gaps between them. Values
/// are rounded to whole pixels and kept within the valid ranges.
pub fn pack_for_canvas(id: &str, canvas: &Canvas) -> Option<Pack> {
    let mut pack = packs().into_iter().find(|pack| pack.id == id)?;
    let fit = Fit::new(canvas);
    for deck in &mut pack.decks {
        let slot = pack.elements.iter().find(|element| element.id == deck.element);
        let size = slot.map_or((0.0, 0.0), |slot| (slot.props.width, slot.props.height));
        for card in &mut deck.cards {
            scale_patch(&mut card.props, fit, size);
        }
    }
    for element in &mut pack.elements {
        scale_props(&mut element.props, fit);
    }
    Some(pack)
}

impl Project {
    /// Returns a copy of this project with a pack's elements and decks added, validated.
    ///
    /// A pack whose ids the project already uses is refused rather than renamed, so adding a pack
    /// twice never leaves two copies of it. A pack without decks, such as the subtitles bar, is
    /// in the project when all its elements are.
    pub fn add_pack(&self, pack: &Pack) -> Result<Project, ValidationError> {
        let has_elements = || pack.elements.iter().all(|element| self.element(&element.id).is_some());
        if pack.decks.iter().any(|deck| self.deck(&deck.id).is_some()) || (pack.decks.is_empty() && has_elements()) {
            return Err(ValidationError(format!("{} is already in this project", pack.name)));
        }
        if let Some(element) = pack.elements.iter().find(|element| self.element(&element.id).is_some()) {
            let message = format!(
                "{} needs the element id {:?}, which this project already uses",
                pack.name, element.id
            );
            return Err(ValidationError(message));
        }
        let mut next = self.clone();
        next.elements.extend(pack.elements.iter().cloned());
        next.decks.extend(pack.decks.iter().cloned());
        next.validate()?;
        Ok(next)
    }
}

/// How the authored layout maps onto the canvas: a factor per axis for where boxes sit, and the
/// smaller one for every length, so a box fits the canvas on both axes.
#[derive(Clone, Copy, Debug)]
struct Fit {
    x: f64,
    y: f64,
    length: f64,
}

impl Fit {
    fn new(canvas: &Canvas) -> Fit {
        let x = f64::from(canvas.width) / AUTHORED.0;
        let y = f64::from(canvas.height) / AUTHORED.1;
        Fit { x, y, length: x.min(y) }
    }

    /// Where a box of authored `size` starts on an axis with `factor`, keeping its center at the
    /// same fraction of the canvas.
    fn start(self, start: f64, size: f64, factor: f64) -> f64 {
        (start + size / 2.0) * factor - size * self.length / 2.0
    }
}

fn scale_props(props: &mut ElementProps, fit: Fit) {
    props.x = fit.start(props.x, props.width, fit.x);
    props.y = fit.start(props.y, props.height, fit.y);
    let lengths = [
        (&mut props.x, 1.0, POSITION),
        (&mut props.y, 1.0, POSITION),
        (&mut props.width, fit.length, SIZE),
        (&mut props.height, fit.length, SIZE),
        (&mut props.font_size, fit.length, FONT_SIZE),
        (&mut props.border_width, fit.length, BORDER_WIDTH),
        (&mut props.corner_radius, fit.length, CORNER_RADIUS),
    ];
    for (value, factor, range) in lengths {
        scale(value, factor, range);
    }
}

/// A card that moves its slot is placed by its own size, or else by the slot's `(width, height)`.
fn scale_patch(patch: &mut PropsPatch, fit: Fit, (width, height): (f64, f64)) {
    let (width, height) = (patch.width.unwrap_or(width), patch.height.unwrap_or(height));
    patch.x = patch.x.map(|x| fit.start(x, width, fit.x));
    patch.y = patch.y.map(|y| fit.start(y, height, fit.y));
    let lengths = [
        (&mut patch.x, 1.0, POSITION),
        (&mut patch.y, 1.0, POSITION),
        (&mut patch.width, fit.length, SIZE),
        (&mut patch.height, fit.length, SIZE),
        (&mut patch.font_size, fit.length, FONT_SIZE),
        (&mut patch.border_width, fit.length, BORDER_WIDTH),
        (&mut patch.corner_radius, fit.length, CORNER_RADIUS),
    ];
    for (value, factor, range) in lengths {
        if let Some(value) = value {
            scale(value, factor, range);
        }
    }
}

fn scale(value: &mut f64, factor: f64, (min, max): (f64, f64)) {
    *value = (*value * factor).round().clamp(min, max);
}

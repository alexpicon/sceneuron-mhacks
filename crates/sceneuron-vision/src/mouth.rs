// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Mouth geometry: how far apart the lips are, from the lip outlines a face landmark model
//! located. The function is pure so that any landmark model can share it.
//!
//! Outlines use normalized frame coordinates with the origin at the top left, like the pose
//! joints; distances are measured after scaling `x` by the frame's aspect ratio.

use crate::pose::{EPSILON, Point, Space};

/// A mouth whose [`mouth_opening`] reaches this counts as open. Closed lips measure under 0.03,
/// lips parted in speech about 0.05 to 0.2, and a mouth opened wide 0.3 or more.
pub const MOUTH_OPEN: f32 = 0.08;
/// The hint for an open mouth; the measure follows it in parentheses, as in `mouth open (0.21)`.
pub const MOUTH_OPEN_FACT: &str = "mouth open";
/// The hint for a closed mouth, with its measure like [`MOUTH_OPEN_FACT`].
pub const MOUTH_CLOSED_FACT: &str = "mouth closed";

/// How far apart the lips are, as a fraction of the mouth's width: the area inside the inner lip
/// outline divided by that outline's width, which is the mean gap between the lips, over the
/// width of the outer lip outline. Widths are measured across the frame, so a mouth opened
/// taller than wide still measures by its width; a head tilted by 15 degrees measures about 7%
/// more than an upright one.
///
/// `inner` and `outer` list the outline points in order around the lips. `aspect` is the frame's
/// width divided by its height. The result is at most 1. `None` when the inner outline has fewer
/// than three points, the outer one fewer than two, a point is not finite, or an outline has no
/// width.
pub fn mouth_opening(inner: &[Point], outer: &[Point], aspect: f32) -> Option<f32> {
    if inner.len() < 3 || outer.len() < 2 {
        return None;
    }
    let finite = |point: &Point| point.x.is_finite() && point.y.is_finite();
    if !inner.iter().chain(outer).all(finite) {
        return None;
    }
    let space = Space::new(aspect);
    let (inner_width, outer_width) = (width(inner, space), width(outer, space));
    if inner_width < EPSILON || outer_width < EPSILON {
        return None;
    }
    let gap = area(inner, space) / inner_width;
    Some((gap / outer_width).min(1.0))
}

/// The hint for a mouth that measures `opening`, such as `mouth open (0.21)`.
pub fn mouth_fact(opening: f32) -> String {
    let state = if opening >= MOUTH_OPEN {
        MOUTH_OPEN_FACT
    } else {
        MOUTH_CLOSED_FACT
    };
    format!("{state} ({opening:.2})")
}

/// The outline's extent across the frame.
fn width(outline: &[Point], space: Space) -> f32 {
    let (left, right) = outline
        .iter()
        .map(|point| space.vector(outline[0], *point).0)
        .fold((0.0f32, 0.0f32), |(left, right), x| (left.min(x), right.max(x)));
    right - left
}

/// The shoelace area of the closed polygon through `outline`.
fn area(outline: &[Point], space: Space) -> f32 {
    let origin = outline[0];
    let twice: f32 = outline
        .iter()
        .zip(outline.iter().cycle().skip(1))
        .map(|(a, b)| {
            let (ax, ay) = space.vector(origin, *a);
            let (bx, by) = space.vector(origin, *b);
            ax * by - bx * ay
        })
        .sum();
    twice.abs() / 2.0
}

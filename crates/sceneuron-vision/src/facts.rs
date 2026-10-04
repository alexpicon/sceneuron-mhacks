// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Short, human-readable hints about a [`Perception`] for a vision-language model, and the
//! test for whether a perception shows something new.

use std::collections::HashSet;

use crate::mouth::mouth_fact;
use crate::perception::{Chirality, Gesture, Hand, Label, Perception};

/// An animal or label is new only when it reaches this confidence and was absent from the
/// previous perception; the gap above the perceivers' own thresholds keeps a label that
/// hovers around them from counting as new over and over.
pub const NOVEL_LABEL_CONFIDENCE: f32 = 0.6;
/// The hint for a hand over the upper half of the largest face.
pub const FACEPALM_FACT: &str = "hand over face";
/// Two text lines with at least this similarity (1 minus the edit distance over the longer
/// length, on lowercase letters and digits) count as the same, so recognition noise in one
/// character does not count as new text.
pub const SAME_TEXT_SIMILARITY: f32 = 0.75;
/// Text with fewer letters and digits than this is never new.
pub const MIN_NOVEL_TEXT_CHARS: usize = 3;

/// Renders a perception as short hints, such as `face in view (0.98)`, `mouth open (0.21)`,
/// [`FACEPALM_FACT`], `right hand raised, thumbs up`, `3 fingers up`, `cat (0.93)`, or
/// `text: "OAT MILK"`. The mouth and facepalm hints describe the largest face. A hand's finger
/// count, when known, follows its hint. Repeated hints appear once, except that a hand with a
/// known count is always listed, so counts can be added up across hands.
pub fn facts(perception: &Perception) -> Vec<String> {
    let mut facts = Vec::new();
    match perception.faces.as_slice() {
        [] => {}
        [face] => facts.push(format!("face in view ({:.2})", face.score)),
        faces => facts.push(format!(
            "{} faces in view (best {:.2})",
            faces.len(),
            perception.best_face
        )),
    }
    facts.extend(perception.mouth_opening.map(mouth_fact));
    if perception.facepalm() {
        facts.push(FACEPALM_FACT.to_owned());
    }
    let mut hands = HashSet::new();
    for detected in &perception.hands {
        let hint = hand(detected);
        if detected.extended.is_some() || hands.insert(hint.clone()) {
            facts.push(hint);
            facts.extend(detected.extended.map(fingers));
        }
    }
    let first_other = facts.len();
    for body in &perception.bodies {
        let pose = match (body.arms_up, body.left_arm_up, body.right_arm_up) {
            (true, ..) => "arms up",
            (false, true, _) => "left arm up",
            (false, false, true) => "right arm up",
            (false, false, false) => continue,
        };
        facts.push(pose.to_owned());
    }
    let animals: HashSet<&str> = perception.animals.iter().map(|animal| animal.name.as_str()).collect();
    let labels = perception
        .labels
        .iter()
        .filter(|label| !animals.contains(label.name.as_str()));
    for label in perception.animals.iter().chain(labels) {
        facts.push(format!("{} ({:.2})", label.name, label.confidence));
    }
    for line in &perception.text {
        facts.push(format!("text: {:?}", line.text));
    }
    let others = facts.split_off(first_other);
    let mut seen = HashSet::new();
    facts.extend(others.into_iter().filter(|fact| seen.insert(fact.clone())));
    facts
}

fn hand(hand: &Hand) -> String {
    let side = match hand.chirality {
        Chirality::Left => "left hand",
        Chirality::Right => "right hand",
        Chirality::Unknown => "hand",
    };
    match (hand.raised, hand.gesture) {
        (true, Some(gesture)) => format!("{side} raised, {}", gesture.describe()),
        (true, None) => format!("{side} raised"),
        (false, Some(gesture)) => format!("{side}, {}", gesture.describe()),
        (false, None) => format!("{side} in view"),
    }
}

fn fingers(count: u8) -> String {
    let noun = if count == 1 { "finger" } else { "fingers" };
    format!("{count} {noun} up")
}

/// True when `current` shows something `previous` did not: a person appearing or leaving
/// (the count of faces or bodies, whichever is larger, changed), a new animal or label at
/// [`NOVEL_LABEL_CONFIDENCE`], a new gesture, more raised hands or arms, or a new line of
/// text (see [`SAME_TEXT_SIMILARITY`]).
pub fn novelty(previous: &Perception, current: &Perception) -> bool {
    people(previous) != people(current)
        || new_label(&previous.animals, &current.animals)
        || new_label(&previous.labels, &current.labels)
        || !gestures(current).is_subset(&gestures(previous))
        || raised(current) > raised(previous)
        || current.text.iter().any(|line| new_text(previous, &line.text))
}

fn people(perception: &Perception) -> usize {
    perception.faces.len().max(perception.bodies.len())
}

fn new_label(previous: &[Label], current: &[Label]) -> bool {
    current
        .iter()
        .filter(|label| label.confidence >= NOVEL_LABEL_CONFIDENCE)
        .any(|label| previous.iter().all(|seen| seen.name != label.name))
}

fn gestures(perception: &Perception) -> HashSet<Gesture> {
    perception.hands.iter().filter_map(|hand| hand.gesture).collect()
}

fn raised(perception: &Perception) -> usize {
    let hands = perception.hands.iter().filter(|hand| hand.raised).count();
    let arms: usize = perception
        .bodies
        .iter()
        .map(|body| usize::from(body.left_arm_up) + usize::from(body.right_arm_up))
        .sum();
    hands.max(arms)
}

fn new_text(previous: &Perception, text: &str) -> bool {
    let text = normalized(text);
    text.len() >= MIN_NOVEL_TEXT_CHARS
        && previous
            .text
            .iter()
            .all(|seen| similarity(&normalized(&seen.text), &text) < SAME_TEXT_SIMILARITY)
}

fn normalized(text: &str) -> Vec<char> {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// 1 minus the Levenshtein distance over the longer length; 1.0 for two empty strings.
fn similarity(a: &[char], b: &[char]) -> f32 {
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substitution = diagonal + usize::from(ca != cb);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(diagonal + 1);
        }
    }
    1.0 - row[b.len()] as f32 / longest as f32
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the selection of Vision results, without running Vision: box and point conversion
//! from Vision's bottom-left origin, and which labels, animals, and text lines are kept.

use sceneuron_vision::{FaceBox, Label, Point, TextLine};
use sceneuron_vision_apple::select::{
    GENERIC_LABELS, MAX_LABELS, MAX_TEXT_CHARS, MAX_TEXT_LINES, MIN_ANIMAL_CONFIDENCE, MIN_LABEL_CONFIDENCE,
    MIN_TEXT_CONFIDENCE, TextCandidate, VisionRect, animals, face_box, label_name, labels, point, text_lines,
};

fn rect(x: f64, y: f64, width: f64, height: f64) -> VisionRect {
    VisionRect { x, y, width, height }
}

fn candidates(entries: &[(&str, f32)]) -> Vec<(String, f32)> {
    entries
        .iter()
        .map(|(name, confidence)| ((*name).to_owned(), *confidence))
        .collect()
}

fn label(name: &str, confidence: f32) -> Label {
    Label {
        name: name.to_owned(),
        confidence,
    }
}

fn line(text: &str, confidence: f32, height: f32) -> TextCandidate {
    TextCandidate {
        text: text.to_owned(),
        confidence,
        height,
    }
}

mod coordinates {
    use super::*;

    #[test]
    fn boxes_flip_to_a_top_left_origin() {
        let face = face_box(rect(0.25, 0.5, 0.25, 0.25), 0.9).unwrap();
        assert_eq!(
            face,
            FaceBox {
                x1: 0.25,
                y1: 0.25,
                x2: 0.5,
                y2: 0.5,
                score: 0.9
            }
        );
    }

    #[test]
    fn boxes_are_clamped_to_the_frame() {
        let face = face_box(rect(-0.25, -0.25, 0.5, 0.5), 1.5).unwrap();
        assert_eq!(
            face,
            FaceBox {
                x1: 0.0,
                y1: 0.75,
                x2: 0.25,
                y2: 1.0,
                score: 1.0
            }
        );
    }

    #[test]
    fn empty_outside_or_invalid_boxes_are_dropped() {
        assert_eq!(face_box(rect(0.5, 0.5, 0.0, 0.2), 0.9), None);
        assert_eq!(face_box(rect(1.2, 0.5, 0.2, 0.2), 0.9), None);
        assert_eq!(face_box(rect(f64::NAN, 0.5, 0.2, 0.2), 0.9), None);
        assert_eq!(face_box(rect(0.1, 0.5, 0.2, f64::INFINITY), 0.9), None);
        assert_eq!(face_box(rect(0.1, 0.5, 0.2, 0.2), f32::NAN), None);
    }

    #[test]
    fn points_flip_to_a_top_left_origin() {
        assert_eq!(point(0.25, 0.75), Point { x: 0.25, y: 0.25 });
        assert_eq!(point(1.0, 0.0), Point { x: 1.0, y: 1.0 });
    }
}

mod image_labels {
    use super::*;

    #[test]
    fn names_are_lowercase_words() {
        assert_eq!(label_name("Coffee_Mug"), "coffee mug");
        assert_eq!(label_name(" Cat "), "cat");
    }

    #[test]
    fn keeps_the_most_confident_specific_labels() {
        let kept = labels(candidates(&[
            ("people", 0.9),
            ("adult", 0.9),
            ("helmet", 0.23),
            ("guitar", 0.6),
            ("headphones", 0.019),
            ("chair_other", 0.7),
            ("pizza", 0.45),
            ("microphone", 0.3),
        ]));
        assert_eq!(
            kept,
            [label("guitar", 0.6), label("pizza", 0.45), label("microphone", 0.3)]
        );
        assert_eq!(kept.len(), MAX_LABELS);
    }

    #[test]
    fn the_confidence_threshold_is_inclusive() {
        let kept = labels(candidates(&[("helmet", MIN_LABEL_CONFIDENCE), ("bottle", 0.19)]));
        assert_eq!(kept, [label("helmet", MIN_LABEL_CONFIDENCE)]);
    }

    #[test]
    fn every_generic_label_is_dropped() {
        assert!(
            GENERIC_LABELS.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted and unique"
        );
        for generic in GENERIC_LABELS {
            assert_eq!(label_name(generic), *generic, "{generic} is stored as a label name");
            let identifier = generic.replace(' ', "_");
            assert!(labels(candidates(&[(&identifier, 0.99)])).is_empty(), "{generic}");
        }
    }

    #[test]
    fn duplicates_and_invalid_confidences_are_dropped() {
        let kept = labels(candidates(&[
            ("Guitar", 0.4),
            ("guitar", 0.7),
            ("lamp", f32::NAN),
            ("", 0.9),
        ]));
        assert_eq!(kept, [label("guitar", 0.7)]);
    }
}

mod animal_labels {
    use super::*;

    #[test]
    fn keeps_one_confident_entry_per_animal() {
        let kept = animals(candidates(&[("Cat", 0.62), ("Dog", 0.95), ("Cat", 0.93), ("Dog", 0.2)]));
        assert_eq!(kept, [label("dog", 0.95), label("cat", 0.93)]);
    }

    #[test]
    fn unsure_animals_are_dropped() {
        let threshold = MIN_ANIMAL_CONFIDENCE;
        let kept = animals(candidates(&[("Cat", threshold), ("Dog", threshold - 0.01)]));
        assert_eq!(kept, [label("cat", threshold)]);
        assert!(animals(Vec::new()).is_empty());
    }
}

mod text {
    use super::*;

    fn texts(lines: &[TextLine]) -> Vec<&str> {
        lines.iter().map(|line| line.text.as_str()).collect()
    }

    #[test]
    fn keeps_confident_lines_tallest_first() {
        let kept = text_lines([
            line("small print", 1.0, 0.02),
            line("OAT MILK", 1.0, 0.2),
            line("unsure", MIN_TEXT_CONFIDENCE - 0.01, 0.3),
            line("BARISTA", MIN_TEXT_CONFIDENCE, 0.1),
        ]);
        assert_eq!(texts(&kept), ["OAT MILK", "BARISTA", "small print"]);
        assert_eq!(kept[1].confidence, MIN_TEXT_CONFIDENCE);
    }

    #[test]
    fn equal_heights_prefer_confidence() {
        let kept = text_lines([line("maybe", 0.5, 0.1), line("surely", 1.0, 0.1)]);
        assert_eq!(texts(&kept), ["surely", "maybe"]);
    }

    #[test]
    fn lines_are_trimmed_capped_and_never_empty() {
        let long = "x".repeat(MAX_TEXT_CHARS + 10);
        let kept = text_lines([
            line("  LIVE NOW  ", 1.0, 0.3),
            line(&long, 1.0, 0.2),
            line("   ", 1.0, 0.1),
            line(&format!("{} y", "é".repeat(MAX_TEXT_CHARS - 1)), 1.0, 0.05),
        ]);
        assert_eq!(kept[0].text, "LIVE NOW");
        assert_eq!(kept[1].text.chars().count(), MAX_TEXT_CHARS);
        assert_eq!(
            kept[2].text,
            "é".repeat(MAX_TEXT_CHARS - 1),
            "cut lines lose trailing spaces"
        );
        assert_eq!(kept.len(), 3);
    }

    #[test]
    fn at_most_a_few_lines_are_kept() {
        let many = (0..10).map(|index| line(&format!("line {index}"), 1.0, index as f32 / 100.0));
        let kept = text_lines(many);
        assert_eq!(kept.len(), MAX_TEXT_LINES);
        assert_eq!(kept[0].text, "line 9");
    }
}

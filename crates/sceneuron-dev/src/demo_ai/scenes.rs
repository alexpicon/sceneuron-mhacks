// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A scripted camera for the simulated vision model: a loop of scenes, each judged against the
//! project's own conditions by what they are about, so any condition turns on at some point.

use sceneuron_core::{Condition, SceneObservation, Verdict};

/// What a condition watches for, guessed from its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Topic {
    /// Nothing in particular happens.
    Idle,
    Holding,
    Pet,
    Wave,
    Celebrate,
    /// Conditions about anything else, so they flip too.
    Other,
}

/// Keywords per topic, checked in order against a condition's id, name, and prompt.
const KEYWORDS: [(Topic, &[&str]); 4] = [
    (Topic::Pet, &["pet", "cat", "dog", "animal", "kitten", "puppy"]),
    (Topic::Wave, &["wave", "waving"]),
    (Topic::Celebrate, &["arms", "celebrat", "cheer", "hype"]),
    (Topic::Holding, &["hold", "showing", "product", "object", "item"]),
];

/// One pass of the scripted camera; every step is one look.
const SCRIPT: [Topic; 20] = [
    Topic::Idle,
    Topic::Holding,
    Topic::Holding,
    Topic::Idle,
    Topic::Idle,
    Topic::Pet,
    Topic::Pet,
    Topic::Idle,
    Topic::Idle,
    Topic::Wave,
    Topic::Wave,
    Topic::Idle,
    Topic::Idle,
    Topic::Celebrate,
    Topic::Celebrate,
    Topic::Idle,
    Topic::Idle,
    Topic::Other,
    Topic::Other,
    Topic::Idle,
];

/// Products held up in successive passes, so the overlay text changes between them.
const PRODUCTS: [&str; 3] = ["Mechanical keyboard", "Coffee mug", "Vintage camera"];

/// The topic a condition is about.
pub(crate) fn topic_of(condition: &Condition) -> Topic {
    let words = format!("{} {} {}", condition.id, condition.name, condition.prompt).to_lowercase();
    KEYWORDS
        .iter()
        .find(|(_, keywords)| keywords.iter().any(|keyword| mentions(&words, keyword)))
        .map_or(Topic::Other, |(topic, _)| *topic)
}

/// Whether lowercase `text` has a word that starts with `keyword`, or contains `keyword` when it
/// is several words, so "cats" matches "cat" but "chatting" and "carpet" do not.
pub(crate) fn mentions(text: &str, keyword: &str) -> bool {
    if keyword.contains(' ') {
        return text.contains(keyword);
    }
    text.split(|c: char| !c.is_alphanumeric())
        .any(|word| word.starts_with(keyword))
}

/// What the simulated model reports for the look at `step`.
pub(crate) fn observation(step: u64, conditions: &[Condition]) -> SceneObservation {
    let topic = SCRIPT[(step % SCRIPT.len() as u64) as usize];
    let product = PRODUCTS[((step / SCRIPT.len() as u64) % PRODUCTS.len() as u64) as usize];
    let verdicts = conditions
        .iter()
        .filter(|condition| condition.enabled)
        .map(|condition| {
            let holds = topic != Topic::Idle && topic_of(condition) == topic;
            Verdict {
                condition: condition.id.clone(),
                holds,
                confidence: if holds { 0.88 + 0.02 * (step % 5) as f64 } else { 0.07 },
                detail: if holds && !condition.detail.is_empty() {
                    detail(topic, product, condition)
                } else {
                    String::new()
                },
                reason: if holds {
                    reason(topic, product)
                } else {
                    "Nothing like that is in this image.".to_owned()
                },
            }
        })
        .collect();
    SceneObservation {
        caption: caption(topic, product, step),
        verdicts,
    }
}

fn caption(topic: Topic, product: &str, step: u64) -> String {
    let product = product.to_lowercase();
    match topic {
        Topic::Idle if step.is_multiple_of(2) => "A person sits at a desk and talks to the camera.".to_owned(),
        Topic::Idle => "The person leans back and keeps chatting with viewers.".to_owned(),
        Topic::Holding => format!("The person holds a {product} up to the camera."),
        Topic::Pet => "A gray cat jumps onto the desk beside the person.".to_owned(),
        Topic::Wave => "The person waves at the camera with a big smile.".to_owned(),
        Topic::Celebrate => "The person throws both arms up and cheers.".to_owned(),
        Topic::Other => "The person does something worth reacting to.".to_owned(),
    }
}

fn reason(topic: Topic, product: &str) -> String {
    match topic {
        Topic::Holding => format!("A {} is held up close to the lens.", product.to_lowercase()),
        Topic::Pet => "A cat is clearly visible on the desk.".to_owned(),
        Topic::Wave => "One hand is raised and moving side to side.".to_owned(),
        Topic::Celebrate => "Both arms are raised above the head.".to_owned(),
        Topic::Idle | Topic::Other => "The image matches what this condition describes.".to_owned(),
    }
}

fn detail(topic: Topic, product: &str, condition: &Condition) -> String {
    match topic {
        Topic::Holding => product.to_owned(),
        Topic::Pet => "Cat".to_owned(),
        Topic::Wave => "Wave".to_owned(),
        Topic::Celebrate => "Arms up".to_owned(),
        Topic::Idle | Topic::Other => condition.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use sceneuron_core::default_project;

    use super::*;

    fn condition(id: &str, prompt: &str, detail: &str) -> Condition {
        Condition {
            id: id.to_owned(),
            name: id.to_owned(),
            prompt: prompt.to_owned(),
            detail: detail.to_owned(),
            enabled: true,
            min_confidence: 0.6,
            confirm: 1,
            release: 2,
        }
    }

    #[test]
    fn topics_come_from_the_condition_words() {
        let project = default_project();
        let topics: Vec<Topic> = project.conditions.iter().map(topic_of).collect();
        assert_eq!(topics, [Topic::Holding, Topic::Pet]);
        assert_eq!(topic_of(&condition("waving", "The person waves.", "")), Topic::Wave);
        assert_eq!(
            topic_of(&condition("arms-up", "Both arms go up.", "")),
            Topic::Celebrate
        );
        assert_eq!(
            topic_of(&condition("drinking", "The person sips tea.", "")),
            Topic::Other
        );
        assert_eq!(topic_of(&condition("rug", "The carpet is visible.", "")), Topic::Other);
    }

    #[test]
    fn mentions_matches_word_starts_and_phrases() {
        assert!(mentions("my cats jump", "cat"));
        assert!(!mentions("just chatting", "cat"));
        assert!(mentions("when i hold up a mug", "hold up"));
        assert!(!mentions("when i hold it up", "hold up"));
    }

    #[test]
    fn every_condition_holds_in_some_step_and_idle_steps_hold_nothing() {
        let conditions = vec![
            condition("showing-item", "The person holds an object up.", "the object's name"),
            condition("pet", "A cat is visible.", "the animal"),
            condition("waving", "The person waves.", ""),
            condition("arms-up", "Both arms go up.", ""),
            condition("drinking", "The person sips tea.", "the drink"),
        ];
        for (index, condition) in conditions.iter().enumerate() {
            let holds = (0..SCRIPT.len() as u64).any(|step| observation(step, &conditions).verdicts[index].holds);
            assert!(holds, "{} never holds", condition.id);
        }
        let idle = observation(0, &conditions);
        assert!(
            idle.verdicts
                .iter()
                .all(|verdict| !verdict.holds && verdict.detail.is_empty())
        );
        assert!(!idle.caption.is_empty());
    }

    #[test]
    fn held_products_change_between_passes_and_details_follow_the_condition() {
        let conditions = vec![
            condition("showing-item", "The person holds an object up.", "the object's name"),
            condition("held", "The person holds something.", ""),
        ];
        let first = observation(1, &conditions);
        assert_eq!(first.verdicts[0].detail, "Mechanical keyboard");
        assert!(first.verdicts[0].holds && first.verdicts[0].confidence >= 0.6);
        assert_eq!(
            first.verdicts[1].detail, "",
            "a condition that asks for no detail gets none"
        );
        let next = observation(1 + SCRIPT.len() as u64, &conditions);
        assert_eq!(next.verdicts[0].detail, "Coffee mug");
        assert!(next.caption.contains("coffee mug"));
    }

    #[test]
    fn disabled_conditions_get_no_verdict() {
        let mut off = condition("pet", "A cat is visible.", "");
        off.enabled = false;
        assert!(observation(5, &[off]).verdicts.is_empty());
    }
}

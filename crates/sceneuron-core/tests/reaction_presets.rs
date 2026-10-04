// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the reaction presets: the table and its JSON ids, the meeting presets' banners, each
//! preset as a rule action and a manual flood with what the event log says, how long a flood plays,
//! the trigger phrases, and the switches that turn off a preset's camera or voice, with their
//! validation and defaults.

use sceneuron_core::{
    Action, Camera, Engine, Flood, FloodState, Project, Rule, Trigger, default_project, presets, reactions,
};

fn ids() -> Vec<String> {
    presets()
        .iter()
        .map(|preset| serde_json::to_value(preset.flood).unwrap().as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_presets_are_the_tongue_and_nine_for_meetings_in_this_order() {
    assert_eq!(
        ids(),
        [
            "tongue",
            "email",
            "mind-blown",
            "ship-it",
            "coffee",
            "absolutely-right",
            "more-tokens",
            "works-on-my-gpu",
            "meatproxy",
            "clanker",
        ]
    );
    let banners: Vec<_> = presets().iter().map(|preset| preset.banner).collect();
    assert_eq!(
        banners,
        [
            None,
            Some("This meeting could have been an email"),
            Some("Mind blown"),
            Some("Ship it!"),
            Some("Coffee break?"),
            Some("You're absolutely right!"),
            Some("We need more tokens!"),
            Some("Works on my GPU"),
            Some("Meatproxy alert!"),
            Some("My clanker will do it!"),
        ]
    );
    let email = Flood::Email.preset();
    assert_eq!(email.emojis, ["📧", "✉️", "📨", "📬", "🤦"]);
    assert_eq!((email.short, email.duration_ms), ("Email", 4_000));
    assert_eq!(Flood::Tongue.preset().emojis, ["😜", "😛", "🤪", "😝"]);
    assert_eq!(Flood::Tongue.preset().duration_ms, 3_000, "the tongue keeps its length");
}

#[test]
fn every_preset_is_whole_and_tells_itself_apart() {
    let mut shorts = Vec::new();
    for preset in presets() {
        assert_eq!(preset.flood.preset(), preset);
        assert!(preset.emojis.len() >= 4, "{:?} has few emojis", preset.flood);
        assert!(
            !preset.name.is_empty() && preset.short.chars().count() <= 10,
            "{preset:?}"
        );
        let color = preset.color.strip_prefix('#').unwrap();
        assert!(
            color.len() == 6 && color.chars().all(|c| c.is_ascii_hexdigit()),
            "{}",
            preset.color
        );
        assert!((3_000..=5_000).contains(&preset.duration_ms), "{preset:?}");
        assert!(!shorts.contains(&preset.short), "{} twice", preset.short);
        shorts.push(preset.short);
    }
}

#[test]
fn the_camera_plays_the_tongue_by_a_look_and_the_email_by_a_facepalm_only() {
    let cameras: Vec<_> = presets().iter().map(|preset| preset.camera).collect();
    assert_eq!(cameras[..2], [Some(Camera::Look), Some(Camera::Facepalm)]);
    assert!(cameras[2..].iter().all(Option::is_none));
    for reaction in reactions() {
        assert_eq!(
            reaction.flood.preset().camera,
            Some(Camera::Look),
            "{:?}",
            reaction.flood
        );
    }
    let looks = presets().iter().filter(|preset| preset.camera == Some(Camera::Look));
    assert_eq!(
        looks.count(),
        reactions().len(),
        "every look has its built-in condition"
    );
}

#[test]
fn every_preset_is_a_rule_action_and_unknown_ones_are_refused() {
    for id in ids() {
        let json = format!(r#"{{"type": "flood", "flood": "{id}"}}"#);
        let action: Action = serde_json::from_str(&json).unwrap();
        let mut project = default_project();
        project.rules.push(Rule {
            id: format!("play-{id}"),
            name: format!("Play {id}"),
            enabled: true,
            when: Trigger::away(),
            actions: vec![action],
        });
        assert!(project.validate().is_ok(), "{id}");
    }
    assert!(serde_json::from_str::<Flood>(r#""mind_blown""#).is_err());
    assert!(serde_json::from_str::<Flood>(r#""confetti""#).is_err());
}

#[test]
fn a_manual_banner_flood_logs_its_emojis_and_its_line() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.trigger_flood(Flood::Email, 1_000);
    let flood = engine.flood().unwrap();
    assert_eq!((flood.flood, flood.at_ms), (Flood::Email, 1_000));
    let event = &engine.snapshot(1_000).events[0];
    assert_eq!(
        event.detail,
        r#"flooded the overlay with 📧 ✉️ 📨 📬 🤦 and "This meeting could have been an email""#
    );
    let published = serde_json::to_value(engine.snapshot(1_000)).unwrap();
    assert_eq!(published["flood"]["flood"], "email");
    engine.trigger_flood(Flood::ShipIt, 2_000);
    assert_eq!(
        engine.flood().unwrap().flood,
        Flood::ShipIt,
        "a manual flood replaces one playing"
    );
}

#[test]
fn a_flood_plays_for_its_presets_length() {
    let flood = |flood, at_ms| FloodState { flood, nonce: 1, at_ms };
    assert_eq!(flood(Flood::Email, 1_000).left_ms(1_000), 4_000);
    assert_eq!(flood(Flood::Email, 1_000).left_ms(4_999), 1);
    assert_eq!(flood(Flood::Email, 1_000).left_ms(5_000), 0);
    assert_eq!(flood(Flood::Tongue, 1_000).left_ms(3_500), 500);
    assert_eq!(
        flood(Flood::Coffee, 9_000).left_ms(100),
        4_000,
        "a clock behind the flood"
    );
}

#[test]
fn every_camera_is_on_by_default_and_older_projects_load_that_way() {
    let reactions = default_project().reactions;
    assert!(reactions.camera_off.is_empty());
    assert!(reactions.watches(Flood::Tongue) && reactions.watches(Flood::Email));
    assert!(
        !reactions.watches(Flood::ShipIt),
        "a preset without a camera is never watched"
    );
    let older: Project = serde_json::from_str(r#"{"reactions": {"enabled": true, "cooldown_ms": 30000}}"#).unwrap();
    assert!(older.reactions.camera_off.is_empty());
    assert_eq!(older.reactions.cooldown_ms, 30_000);
}

#[test]
fn a_switched_off_camera_is_saved_by_its_preset() {
    let mut project = default_project();
    project.reactions.camera_off = vec![Flood::Email];
    assert!(project.validate().is_ok());
    assert!(!project.reactions.watches(Flood::Email) && project.reactions.watches(Flood::Tongue));
    let saved = serde_json::to_value(&project.reactions).unwrap();
    assert_eq!(saved["camera_off"], serde_json::json!(["email"]));
    project.reactions.enabled = false;
    assert!(
        !project.reactions.watches(Flood::Tongue),
        "the switch turns every camera off"
    );
}

#[test]
fn camera_switches_name_presets_with_a_camera_once_each() {
    let invalid = |camera_off: Vec<Flood>| {
        let mut project = default_project();
        project.reactions.camera_off = camera_off;
        project.validate().unwrap_err().0
    };
    assert_eq!(
        invalid(vec![Flood::ShipIt]),
        r#"reactions.camera_off names "ship-it", which the camera never plays"#
    );
    assert_eq!(
        invalid(vec![Flood::Email, Flood::Tongue, Flood::Email]),
        r#"reactions.camera_off names "email" twice"#
    );
    let unknown = r#"{"reactions": {"camera_off": ["confetti"]}}"#;
    assert!(serde_json::from_str::<Project>(unknown).is_err());
}

#[test]
fn every_meeting_preset_has_trigger_phrases_of_its_own() {
    let phrases: Vec<&[&str]> = presets().iter().map(|preset| preset.phrases).collect();
    assert_eq!(phrases[0], [] as [&str; 0], "the tongue has none");
    let mut seen: Vec<&str> = Vec::new();
    for preset in &presets()[1..] {
        assert!(!preset.phrases.is_empty(), "{:?}", preset.flood);
        for phrase in preset.phrases {
            assert!(
                phrase.split(' ').count() >= 2,
                "{phrase:?} is too short to say on purpose"
            );
            assert!(!seen.contains(phrase), "{phrase:?} twice");
            seen.push(phrase);
        }
    }
    assert_eq!(Flood::Coffee.preset().phrases, ["coffee break"]);
}

#[test]
fn every_voice_is_on_by_default_and_a_switched_off_one_is_saved_by_its_preset() {
    let mut project = default_project();
    let hears = |project: &Project, flood| project.reactions.hears(flood);
    assert!(hears(&project, Flood::Email) && hears(&project, Flood::Coffee));
    assert!(
        !hears(&project, Flood::Tongue),
        "a preset without phrases is never heard"
    );
    let older: Project = serde_json::from_str(r#"{"reactions": {"enabled": true, "camera_off": []}}"#).unwrap();
    assert!(older.reactions.voice_off.is_empty());
    project.reactions.voice_off = vec![Flood::ShipIt];
    assert!(project.validate().is_ok());
    assert!(!hears(&project, Flood::ShipIt) && hears(&project, Flood::MindBlown));
    let saved = serde_json::to_value(&project.reactions).unwrap();
    assert_eq!(saved["voice_off"], serde_json::json!(["ship-it"]));
    project.reactions.enabled = false;
    assert!(!hears(&project, Flood::MindBlown), "the switch turns every voice off");
}

#[test]
fn voice_switches_name_presets_with_phrases_once_each() {
    let invalid = |voice_off: Vec<Flood>| {
        let mut project = default_project();
        project.reactions.voice_off = voice_off;
        project.validate().unwrap_err().0
    };
    assert_eq!(
        invalid(vec![Flood::Tongue]),
        r#"reactions.voice_off names "tongue", which has no trigger phrase"#
    );
    assert_eq!(
        invalid(vec![Flood::Coffee, Flood::Coffee]),
        r#"reactions.voice_off names "coffee" twice"#
    );
}

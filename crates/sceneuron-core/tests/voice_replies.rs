// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises carrying out the AI's reply to words addressed to Sceneuron: every action kind in
//! order, with the theme and the project edit saved and the heard line saying what the AI took
//! the words for and what was done; and what cannot be done (a deck whose voice is off, an
//! unknown element, text too long, an edit that does not apply, an action only the host can do)
//! says why, as do a reply with nothing to do and a refusal.

mod common;

use common::requests::{heard_result, held, hosted, newest, reply};
use common::visible;
use sceneuron_core::{EditOp, Flood, Theme, VoiceAction, VoiceReply, VoiceStep, VoiceTask};

#[test]
fn the_ais_actions_are_carried_out_in_order_and_said() {
    let mut engine = hosted();
    held(&mut engine, "make it festive", 100);
    let (nonce, task) = newest(&engine);
    assert_eq!(
        task,
        VoiceTask::Ask {
            text: "make it festive".to_owned()
        }
    );
    assert_eq!(heard_result(&engine), "asking the AI");
    let revision = engine.project_revision();
    let steps = vec![
        VoiceStep::Act(VoiceAction::React {
            reaction: Flood::ShipIt,
        }),
        VoiceStep::Act(VoiceAction::Theme {
            theme: Theme::Vaporwave,
        }),
        VoiceStep::Act(VoiceAction::ShowElement {
            element: "brb".to_owned(),
            visible: true,
        }),
        VoiceStep::Act(VoiceAction::SetText {
            element: "name-label".to_owned(),
            text: "Party time".to_owned(),
        }),
        VoiceStep::Act(VoiceAction::RaiseCard {
            deck: "meeting".to_owned(),
            card: "agree".to_owned(),
        }),
        VoiceStep::Done("turned subtitles on".to_owned()),
        VoiceStep::Act(VoiceAction::Automation { enabled: false }),
        VoiceStep::Act(VoiceAction::EditProject {
            summary: "Removed the pet alert.".to_owned(),
            operations: vec![EditOp::RemoveElement {
                id: "pet-alert".to_owned(),
            }],
        }),
    ];
    let outcome = engine.voice_replied(nonce, reply(steps), 2_000);
    assert!(outcome.changed && outcome.project_dirty);
    assert_eq!(
        heard_result(&engine),
        "the AI took it as \"make it festive\": played 🚀 Ship it; switched the overlay to the Vaporwave theme; \
         showed Be right back; Name label says \"Party time\"; raised Meeting signals: Agree; turned subtitles on; \
         paused automation; changed the project: Removed the pet alert."
    );
    assert_eq!(engine.flood().map(|flood| flood.flood), Some(Flood::ShipIt));
    assert_eq!(engine.project().theme, Theme::Vaporwave);
    assert!(visible(&engine, "brb"));
    assert_eq!(engine.effective_props("name-label").unwrap().text, "Party time");
    assert_eq!(engine.raised_card("meeting"), Some("agree"));
    assert!(!engine.project().automation_enabled);
    assert!(engine.project().element("pet-alert").is_none());
    assert_eq!(engine.project_revision(), revision + 2, "the theme and the edit");
    held(&mut engine, "now release them", 3_000);
    let (nonce, _) = newest(&engine);
    let steps = vec![
        VoiceStep::Act(VoiceAction::ReleaseElement {
            element: "brb".to_owned(),
        }),
        VoiceStep::Act(VoiceAction::LowerCards { deck: String::new() }),
        VoiceStep::Act(VoiceAction::Automation { enabled: true }),
    ];
    engine.voice_replied(nonce, reply(steps), 3_500);
    assert!(!visible(&engine, "brb") && engine.raised_card("meeting").is_none());
    assert!(engine.project().automation_enabled);
}

#[test]
fn what_cannot_be_done_says_why() {
    let mut engine = hosted();
    let mut project = engine.project().clone();
    project.decks[1].voice = false;
    engine.replace_project(project, 0).unwrap();
    held(&mut engine, "do the impossible", 100);
    let (nonce, _) = newest(&engine);
    let steps = vec![
        VoiceStep::Act(VoiceAction::RaiseCard {
            deck: "poker".to_owned(),
            card: "p5".to_owned(),
        }),
        VoiceStep::Act(VoiceAction::ShowElement {
            element: "ghost".to_owned(),
            visible: true,
        }),
        VoiceStep::Act(VoiceAction::SetText {
            element: "brb".to_owned(),
            text: "x".repeat(1_001),
        }),
        VoiceStep::Act(VoiceAction::EditProject {
            summary: "Nothing.".to_owned(),
            operations: vec![EditOp::RemoveRule { id: "ghost".to_owned() }],
        }),
        VoiceStep::Act(VoiceAction::Look),
    ];
    let outcome = engine.voice_replied(nonce, reply(steps), 200);
    assert!(!outcome.project_dirty);
    let said = heard_result(&engine);
    for part in [
        "Planning poker does not answer voice; turn its voice on in the Cards tab",
        "there is no element \"ghost\"",
        "Be right back cannot take that: text must be at most 1000 characters",
        "the project change was refused: operation 1: unknown rule \"ghost\"",
        "only Sceneuron's AI can do that",
    ] {
        assert!(said.contains(part), "{part:?} in {said:?}");
    }
    held(&mut engine, "order a pizza", 300);
    let (nonce, _) = newest(&engine);
    let nothing = Ok(VoiceReply {
        understood: "there is no pizza button".to_owned(),
        steps: Vec::new(),
    });
    engine.voice_replied(nonce, nothing, 400);
    assert_eq!(
        heard_result(&engine),
        "the AI found nothing to do: there is no pizza button"
    );
    held(&mut engine, "order a pizza", 500);
    let (nonce, _) = newest(&engine);
    let refused = "the AI is not set up: add a Gemini API key in the AI settings".to_owned();
    engine.voice_replied(nonce, Err(refused.clone()), 600);
    assert_eq!(heard_result(&engine), refused);
}

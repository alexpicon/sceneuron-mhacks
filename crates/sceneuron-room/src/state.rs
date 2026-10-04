// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The room as `/api/room` shows it to the editor: the connection's settings, who is in the room
//! and who voted, this Sceneuron's own vote, and the revealed cards by their names, each person
//! and card keyed by the voter's identity.

use serde_json::{Value, json};

use crate::client::RoomClient;
use crate::summary::{in_deck_order, label, summary};

pub(crate) fn state(client: &RoomClient) -> Value {
    let (form, joined, error) = {
        let state = client.lock();
        let joined = state.session.as_ref().map(|session| {
            (
                session.deck.clone(),
                session.name.clone(),
                session.view.clone(),
                session.voted.clone(),
            )
        });
        (state.form.clone(), joined, state.error.clone())
    };
    let deck_id = joined.as_ref().map_or(&form.deck, |(deck, ..)| deck).clone();
    let deck = client.hub.read(|engine| engine.project().deck(&deck_id).cloned());
    let deck = deck.as_ref();
    let mut value = json!({
        "available": true,
        "practice": client.practice.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_some(),
        "joined": false,
        "server": form.target.server,
        "database": form.target.database,
        "name": form.name,
        "deck": deck_id,
        "code": null,
        "deck_name": null,
        "round": null,
        "revealed": false,
        "participants": [],
        "results": [],
        "my_vote": null,
        "summary": "",
        "error": error,
    });
    let Some((_, name, Some(view), voted)) = joined else {
        return value;
    };
    let name_of = |id: &str| {
        view.members
            .iter()
            .find(|member| member.id == id)
            .map_or_else(String::new, |member| member.name.clone())
    };
    // Each person's key is their identity, which the room's public tables already show; names
    // need not be unique.
    let participants: Vec<Value> = view
        .members
        .iter()
        .map(|member| json!({ "id": member.id, "name": member.name, "voted": member.voted, "me": member.me }))
        .collect();
    let me = view
        .members
        .iter()
        .find(|member| member.me)
        .map(|member| member.id.clone());
    let results: Vec<Value> = in_deck_order(&view.results, deck)
        .into_iter()
        .map(|shown| {
            json!({
                "id": shown.id,
                "name": name_of(&shown.id),
                "card": shown.card,
                "label": label(&shown.card, deck),
                "me": me.as_deref() == Some(shown.id.as_str()),
            })
        })
        .collect();
    let my_vote = voted
        .filter(|_| !view.revealed)
        .map(|card| json!({ "card": card, "label": label(&card, deck) }));
    let fields = json!({
        "joined": true,
        "name": name,
        "code": view.code,
        "deck_name": view.deck_name,
        "round": view.round,
        "revealed": view.revealed,
        "participants": participants,
        "results": results,
        "my_vote": my_vote,
        "summary": summary(&view, deck),
    });
    if let (Some(target), Some(fields)) = (value.as_object_mut(), fields.as_object()) {
        target.extend(fields.clone());
    }
    value
}

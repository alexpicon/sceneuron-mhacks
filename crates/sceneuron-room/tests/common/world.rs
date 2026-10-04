// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The fake rooms' rules, the same as `room/module`'s: creating, joining, leaving, voting,
//! revealing, and the next round, and the room each connection sees.

use std::collections::{BTreeMap, BTreeSet};

use sceneuron_room::{Call, Member, RoomView, Shown};

#[derive(Default)]
pub(super) struct World {
    rooms: BTreeMap<String, Room>,
    /// Each connection's room, name, and order of joining.
    members: BTreeMap<String, (String, String, usize)>,
    pub(super) votes: BTreeMap<String, String>,
    ballots: BTreeSet<String>,
    results: BTreeMap<String, String>,
    created: usize,
    joined: usize,
}

struct Room {
    deck_name: String,
    round: u32,
    revealed: bool,
}

impl World {
    pub(super) fn apply(&mut self, id: &str, call: Call) -> Result<(), String> {
        match call {
            Call::Create { deck_name, name } => {
                self.leave(id);
                self.created += 1;
                let code = format!("K7M2{:02}", self.created);
                let room = Room {
                    deck_name,
                    round: 1,
                    revealed: false,
                };
                self.rooms.insert(code.clone(), room);
                self.enter(id, &code, name);
            }
            Call::Open { code, deck_name, name } => {
                self.rooms.entry(code.clone()).or_insert(Room {
                    deck_name,
                    round: 1,
                    revealed: false,
                });
                self.leave(id);
                self.enter(id, &code, name);
            }
            Call::Join { code, name } => {
                if !self.rooms.contains_key(&code) {
                    return Err(format!("no room has the code {code}"));
                }
                self.leave(id);
                self.enter(id, &code, name);
            }
            Call::Leave => {
                self.room_of(id)?;
                self.leave(id);
            }
            Call::Vote { card } => {
                let code = self.room_of(id)?;
                if self.rooms[&code].revealed {
                    return Err("the cards are already revealed".to_owned());
                }
                self.votes.insert(id.to_owned(), card);
                self.ballots.insert(id.to_owned());
            }
            Call::Reveal => {
                let code = self.room_of(id)?;
                for member in self.in_room(&code) {
                    if let Some(card) = self.votes.get(&member).cloned() {
                        self.results.insert(member, card);
                    }
                }
                self.rooms.get_mut(&code).unwrap().revealed = true;
            }
            Call::NextRound => {
                let code = self.room_of(id)?;
                for member in self.in_room(&code) {
                    self.votes.remove(&member);
                    self.ballots.remove(&member);
                    self.results.remove(&member);
                }
                let room = self.rooms.get_mut(&code).unwrap();
                room.round += 1;
                room.revealed = false;
            }
        }
        Ok(())
    }

    fn room_of(&self, id: &str) -> Result<String, String> {
        self.members
            .get(id)
            .map(|(code, ..)| code.clone())
            .ok_or_else(|| "join a room first".to_owned())
    }

    fn in_room(&self, code: &str) -> Vec<String> {
        let mut ids: Vec<(usize, String)> = self
            .members
            .iter()
            .filter(|(_, (room, ..))| room == code)
            .map(|(id, (_, _, order))| (*order, id.clone()))
            .collect();
        ids.sort();
        ids.into_iter().map(|(_, id)| id).collect()
    }

    fn enter(&mut self, id: &str, code: &str, name: String) {
        self.joined += 1;
        self.members.insert(id.to_owned(), (code.to_owned(), name, self.joined));
    }

    pub(super) fn leave(&mut self, id: &str) {
        let Some((code, ..)) = self.members.remove(id) else {
            return;
        };
        self.votes.remove(id);
        self.ballots.remove(id);
        self.results.remove(id);
        if self.in_room(&code).is_empty() {
            self.rooms.remove(&code);
        }
    }

    pub(super) fn view(&self, me: &str) -> Option<RoomView> {
        let code = self.room_of(me).ok()?;
        let room = &self.rooms[&code];
        let ids = self.in_room(&code);
        Some(RoomView {
            code: code.clone(),
            deck_name: room.deck_name.clone(),
            round: room.round,
            revealed: room.revealed,
            members: ids
                .iter()
                .map(|id| Member {
                    id: id.clone(),
                    name: self.members[id].1.clone(),
                    voted: self.ballots.contains(id),
                    me: id == me,
                })
                .collect(),
            results: ids
                .iter()
                .filter_map(|id| {
                    let card = self.results.get(id)?;
                    Some(Shown {
                        id: id.clone(),
                        card: card.clone(),
                    })
                })
                .collect(),
        })
    }
}

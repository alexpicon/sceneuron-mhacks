// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A scripted shared room that records the requests the server passed on.

use std::sync::Mutex;

use sceneuron_server::{Room, RoomAction, RoomError, RoomRequest};
use serde_json::{Value, json};

/// Answers every request with a state that names its action; the code "WRONG1" is refused.
#[derive(Default)]
pub struct FakeRoom {
    pub requests: Mutex<Vec<RoomRequest>>,
}

impl FakeRoom {
    pub fn actions(&self) -> Vec<RoomAction> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| request.action)
            .collect()
    }
}

impl Room for FakeRoom {
    fn state(&self) -> Value {
        json!({ "joined": true, "code": "K7M2QX", "participants": [{ "name": "Ana", "voted": true }] })
    }

    fn act(&self, request: RoomRequest) -> Result<Value, RoomError> {
        let refused = request.code.as_deref() == Some("WRONG1");
        let action = request.action;
        self.requests.lock().unwrap().push(request);
        if refused {
            return Err(RoomError::new(400, "no room has the code WRONG1"));
        }
        Ok(json!({ "joined": true, "after": format!("{action:?}") }))
    }
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice requests in a Sceneuron hub: each request the engine makes goes to the assistant's voice
//! commands thread once, and its reply comes back to the engine. A request no longer waiting when
//! its turn comes, because it ran out of time or newer ones replaced it, costs no model call.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sceneuron_core::VoiceRequest;
use sceneuron_server::{Hub, HubEvent};

use crate::assistant::Assistant;

/// Hands every new voice request of the hub's engine to `assistant`, which the engine then knows
/// to wait for.
pub(crate) fn follow_requests(hub: &Arc<Hub>, assistant: &Arc<Assistant>) {
    hub.update(|engine, _| engine.set_voice_host(true));
    let (hub_ref, assistant_ref) = (Arc::downgrade(hub), Arc::downgrade(assistant));
    let sent = AtomicU64::new(0);
    hub.add_listener(Box::new(move |event| {
        if !matches!(event, HubEvent::StateChanged { .. }) {
            return;
        }
        let (Some(hub), Some(assistant)) = (hub_ref.upgrade(), assistant_ref.upgrade()) else {
            return;
        };
        let after = sent.load(Ordering::SeqCst);
        let new: Vec<VoiceRequest> = hub.read(|engine| {
            let requests = engine.voice_requests().iter();
            requests.filter(|request| request.nonce > after).cloned().collect()
        });
        for request in new {
            sent.fetch_max(request.nonce, Ordering::SeqCst);
            let (hub_ref, assistant_ref) = (Arc::downgrade(&hub), Arc::downgrade(&assistant));
            assistant.queue_voice(Box::new(move || {
                let (Some(hub), Some(assistant)) = (hub_ref.upgrade(), assistant_ref.upgrade()) else {
                    return;
                };
                let project = hub.read(|engine| {
                    let waiting = engine.voice_requests().iter().any(|known| known.nonce == request.nonce);
                    waiting.then(|| engine.project().clone())
                });
                let Some(project) = project else {
                    return;
                };
                let reply = assistant.carry_out(&project, &request.task);
                hub.update(|engine, now| engine.voice_replied(request.nonce, reply, now));
            }));
        }
    }));
}

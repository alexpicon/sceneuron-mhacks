// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Subtitle segments from the hub to the translator, each once.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sceneuron_server::{Hub, HubEvent};

use crate::assistant::Assistant;

/// Hands every new final subtitle segment that needs translating to the assistant's translator,
/// once, in the order they were said.
pub(crate) fn follow_subtitles(hub: &Arc<Hub>, assistant: &Arc<Assistant>) {
    let (hub_ref, assistant_ref) = (Arc::downgrade(hub), Arc::downgrade(assistant));
    let handed = AtomicU64::new(0);
    hub.add_listener(Box::new(move |event| {
        if !matches!(event, HubEvent::StateChanged { .. }) {
            return;
        }
        let (Some(hub), Some(assistant)) = (hub_ref.upgrade(), assistant_ref.upgrade()) else {
            return;
        };
        let jobs = hub.read(|engine| engine.translation_jobs(handed.load(Ordering::SeqCst)));
        for job in jobs {
            if handed.fetch_max(job.id, Ordering::SeqCst) < job.id {
                assistant.translate(job);
            }
        }
    }));
}

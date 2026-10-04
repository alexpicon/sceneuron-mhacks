// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A scripted AI assistant that records what the server asked of it.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use sceneuron_core::{Condition, EditOp, EditPlan, Project};
use sceneuron_server::{Assistant, AssistantError, PlanRequest, ProposedPlan, Speech};
use serde_json::{Value, json};

/// Plans add a "waving" condition; the request "fail" answers like a rejected API key.
pub struct FakeAssistant {
    pub settings: Mutex<Value>,
    pub requests: Mutex<Vec<PlanRequest>>,
    pub looks: AtomicUsize,
    /// Spoken cards by id, served as MP3 audio.
    pub speeches: Mutex<Vec<(String, Vec<u8>)>>,
}

impl Default for FakeAssistant {
    fn default() -> Self {
        Self {
            settings: Mutex::new(json!({ "available": true, "enabled": false })),
            requests: Mutex::new(Vec::new()),
            looks: AtomicUsize::new(0),
            speeches: Mutex::new(Vec::new()),
        }
    }
}

pub fn waving_plan() -> EditPlan {
    EditPlan {
        summary: "Watch for waving".to_owned(),
        operations: vec![EditOp::AddCondition {
            condition: Condition {
                id: "waving".to_owned(),
                name: "Waving".to_owned(),
                prompt: "The person waves at the camera.".to_owned(),
                detail: String::new(),
                enabled: true,
                min_confidence: 0.6,
                confirm: 1,
                release: 2,
            },
        }],
    }
}

impl Assistant for FakeAssistant {
    fn settings(&self) -> Value {
        self.settings.lock().unwrap().clone()
    }

    fn update_settings(&self, update: Value) -> Result<Value, AssistantError> {
        if update.get("enabled") == Some(&json!(true)) && update.get("consent") != Some(&json!(true)) {
            return Err(AssistantError::new(400, "confirm consent first"));
        }
        let mut settings = self.settings.lock().unwrap();
        for (key, value) in update.as_object().unwrap() {
            settings[key] = value.clone();
        }
        Ok(settings.clone())
    }

    fn plan(&self, project: &Project, request: &PlanRequest) -> Result<ProposedPlan, AssistantError> {
        self.requests.lock().unwrap().push(request.clone());
        if request.request == "fail" {
            return Err(AssistantError::new(502, "the API key was rejected"));
        }
        let plan = waving_plan();
        let project = plan.apply(project).map_err(|error| AssistantError::new(400, error.0))?;
        Ok(ProposedPlan {
            plan,
            project,
            model: "fake-model".to_owned(),
            cost_usd: 0.01,
        })
    }

    fn analyze_now(&self) -> Result<(), AssistantError> {
        self.looks.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn speech(&self, id: &str) -> Option<Speech> {
        let speeches = self.speeches.lock().unwrap();
        let (_, audio) = speeches.iter().find(|(known, _)| known == id)?;
        Some(Speech {
            audio: audio.clone(),
            content_type: "audio/mpeg",
        })
    }
}

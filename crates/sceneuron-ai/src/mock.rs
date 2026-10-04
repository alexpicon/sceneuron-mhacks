// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A provider that answers from a closure and records every request, for tests and for a demo
//! mode that works without an API key.

use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::provider::{AiError, Connector, ModelReply, ModelRequest, Provider, Usage};

type Respond = dyn Fn(&ModelRequest) -> Result<Value, AiError> + Send + Sync;

/// Answers each request with whatever the closure returns.
pub struct MockProvider {
    respond: Box<Respond>,
    usage: Usage,
    requests: Mutex<Vec<ModelRequest>>,
}

impl MockProvider {
    /// A provider whose answers come from `respond`, reporting no token usage.
    pub fn new(respond: impl Fn(&ModelRequest) -> Result<Value, AiError> + Send + Sync + 'static) -> MockProvider {
        MockProvider {
            respond: Box::new(respond),
            usage: Usage::default(),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// Reports `usage` for every successful answer, so cost accounting can be exercised.
    pub fn with_usage(mut self, usage: Usage) -> MockProvider {
        self.usage = usage;
        self
    }

    /// Every request so far, oldest first.
    pub fn requests(&self) -> Vec<ModelRequest> {
        self.lock().clone()
    }

    /// How many requests arrived so far.
    pub fn calls(&self) -> usize {
        self.lock().len()
    }

    /// A connector that hands out this provider whatever the settings say.
    pub fn connector(self: &Arc<Self>) -> Connector {
        let provider = Arc::clone(self);
        Arc::new(move |_settings| Ok(provider.clone() as Arc<dyn Provider>))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<ModelRequest>> {
        self.requests.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Provider for MockProvider {
    fn complete(&self, request: &ModelRequest) -> Result<ModelReply, AiError> {
        self.lock().push(request.clone());
        let json = (self.respond)(request)?;
        Ok(ModelReply {
            json,
            usage: self.usage,
            model: request.model.clone(),
            latency_ms: 0.0,
        })
    }

    fn kind(&self) -> &'static str {
        "mock"
    }
}

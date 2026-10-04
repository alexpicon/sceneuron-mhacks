// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Planning: the streamer's words become an edit plan that applies cleanly, with one retry
//! that sends back why the first answer did not.

use std::sync::Arc;

use sceneuron_core::{EditPlan, Project};
use serde_json::Value;

use super::{Assistant, PlanOutcome, lock};
use crate::cost::{cost_usd, prices_for};
use crate::image::encode_jpeg;
use crate::planner::{PlanMode, parse_plan, plan_request};
use crate::provider::{AiError, ModelRequest, Provider};
use crate::settings::AiSettings;

const MAX_REQUEST_CHARS: usize = 2000;

impl Assistant {
    /// Asks the planner model for an edit plan and checks that it applies to `project`.
    ///
    /// An answer that cannot be read as a plan, or a plan that fails validation, is sent back
    /// once with the reason for a corrected plan.
    /// With `use_camera`, the latest camera frame goes along so the model can see the scene.
    /// Every call counts toward the director's totals and hourly budget.
    pub fn plan(
        &self,
        project: &Project,
        request: &str,
        mode: PlanMode,
        use_camera: bool,
    ) -> Result<PlanOutcome, AiError> {
        let request = request.trim();
        if !(1..=MAX_REQUEST_CHARS).contains(&request.chars().count()) {
            let message = format!("describe what you want in 1 to {MAX_REQUEST_CHARS} characters");
            return Err(AiError::Config(message));
        }
        let settings = lock(&self.settings).with_credentials(&self.credentials);
        let jpeg = if use_camera {
            Some(self.camera_jpeg(&settings)?)
        } else {
            None
        };
        let provider = self.planner(&settings)?;
        let mut spent = 0.0;
        let first = plan_request(project, request, mode, jpeg.clone(), None, &settings);
        let (answer, model) = self.ask(provider.as_ref(), &first, &settings, &mut spent)?;
        let error = match checked(&answer, project) {
            Ok((plan, next)) => return Ok(outcome(plan, next, model, spent)),
            Err(error) => error,
        };
        let retry = plan_request(project, request, mode, jpeg, Some((&answer, &error)), &settings);
        let (answer, model) = self.ask(provider.as_ref(), &retry, &settings, &mut spent)?;
        let plan = parse_plan(&answer)?;
        match plan.apply(project) {
            Ok(next) => Ok(outcome(plan, next, model, spent)),
            Err(error) => Err(AiError::Invalid(format!("the plan did not pass validation: {error}"))),
        }
    }

    /// One planner call, counted toward the totals; returns the answer and the model that gave it.
    pub(super) fn ask(
        &self,
        provider: &dyn Provider,
        request: &ModelRequest,
        settings: &AiSettings,
        spent: &mut f64,
    ) -> Result<(Value, String), AiError> {
        let reply = provider.complete(request)?;
        let cost = prices_for(settings, &reply.model).map_or(0.0, |prices| cost_usd(&reply.usage, &prices));
        *spent += cost;
        self.director.record_usage(&reply.model, reply.usage, cost);
        Ok((reply.json, reply.model))
    }

    fn camera_jpeg(&self, settings: &AiSettings) -> Result<Vec<u8>, AiError> {
        if !settings.consent {
            let message = "confirm that camera images may be sent to the provider first".to_owned();
            return Err(AiError::Config(message));
        }
        let frame = self.director.latest_frame().ok_or_else(|| {
            AiError::Config("no camera image yet; add Sceneuron Camera Analysis to a camera source".to_owned())
        })?;
        encode_jpeg(&frame, settings.image_width)
    }

    pub(super) fn planner(&self, settings: &AiSettings) -> Result<Arc<dyn Provider>, AiError> {
        let mut cached = lock(&self.planner);
        if let Some((built_for, provider)) = cached.as_ref()
            && built_for == settings
        {
            return Ok(Arc::clone(provider));
        }
        let provider = (self.connector)(settings)?;
        *cached = Some((settings.clone(), Arc::clone(&provider)));
        Ok(provider)
    }
}

/// Reads an answer as a plan and applies it; the error is the reason to send back with it.
fn checked(answer: &Value, project: &Project) -> Result<(EditPlan, Project), String> {
    let plan = parse_plan(answer).map_err(|error| error.message().to_owned())?;
    let next = plan.apply(project).map_err(|error| error.0)?;
    Ok((plan, next))
}

fn outcome(plan: EditPlan, project: Project, model: String, cost_usd: f64) -> PlanOutcome {
    PlanOutcome {
        plan,
        project,
        model,
        cost_usd,
    }
}

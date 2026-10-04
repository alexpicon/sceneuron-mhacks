// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Bounded retries for rate limits and outages, within a deadline.

use std::thread;
use std::time::{Duration, Instant};

use super::Failure;
use crate::provider::AiError;

/// Waits before the first and second retry.
const BACKOFF: [Duration; 2] = [Duration::from_millis(500), Duration::from_millis(1500)];
/// A longer `retry-after` is not waited out; the director's own backoff takes over.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(10);

/// Runs `attempt` until it succeeds, fails permanently, or uses up its retries or the time
/// left before `deadline`. Each attempt receives the time it may take.
///
/// Rate limits and outages are retried at most twice, after 500 ms and 1500 ms or the server's
/// `retry-after` if longer. A retry that could not finish before the deadline is not made.
pub(crate) fn with_retries<T>(
    deadline: Instant,
    mut attempt: impl FnMut(Duration) -> Result<T, Failure>,
) -> Result<T, AiError> {
    let mut retries = 0;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(AiError::Unavailable("the request timed out".to_owned()));
        }
        let failure = match attempt(remaining) {
            Ok(value) => return Ok(value),
            Err(failure) => failure,
        };
        let retryable = matches!(failure.error, AiError::RateLimited(_) | AiError::Unavailable(_));
        let Some(backoff) = BACKOFF.get(retries).filter(|_| retryable) else {
            return Err(failure.error);
        };
        let delay = match failure.retry_after {
            Some(after) if after > MAX_RETRY_AFTER => return Err(failure.error),
            Some(after) => after.max(*backoff),
            None => *backoff,
        };
        if Instant::now() + delay >= deadline {
            return Err(failure.error);
        }
        thread::sleep(delay);
        retries += 1;
    }
}

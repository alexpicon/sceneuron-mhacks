// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! One-time editor links: a short-lived code that opens the editor without putting the
//! long-lived token on a command line, where other local users or audit logs could read it.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{generate_token, lock, tokens_match};

/// How long a link works after it was made.
const LINK_LIFETIME: Duration = Duration::from_secs(60);

/// Most links waiting to be used; making another drops the oldest.
const MAX_LINKS: usize = 8;

#[derive(Debug, Default)]
pub(crate) struct EditorLinks {
    codes: Mutex<Vec<(String, Instant)>>,
}

impl EditorLinks {
    /// A new single-use code.
    pub(crate) fn issue(&self) -> String {
        let code = generate_token();
        let mut codes = lock(&self.codes);
        codes.retain(|(_, made)| made.elapsed() < LINK_LIFETIME);
        if codes.len() >= MAX_LINKS {
            codes.remove(0);
        }
        codes.push((code.clone(), Instant::now()));
        code
    }

    /// Uses up `code`; true when it was issued, unused, and not expired.
    pub(crate) fn redeem(&self, code: &str) -> bool {
        let mut codes = lock(&self.codes);
        codes.retain(|(_, made)| made.elapsed() < LINK_LIFETIME);
        match codes.iter().position(|(known, _)| tokens_match(known, code)) {
            Some(index) => {
                codes.remove(index);
                true
            }
            None => false,
        }
    }
}

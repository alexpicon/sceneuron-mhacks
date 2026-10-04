// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reasons for an early call that have not been acted on yet.

/// Reasons waiting for the next call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Pending {
    pub asked: bool,
    pub first_look: bool,
    pub conditions: bool,
    pub presence: bool,
    pub novel: bool,
    pub retry: bool,
}

impl Pending {
    pub const FIRST_LOOK: Pending = Pending {
        asked: false,
        first_look: true,
        conditions: false,
        presence: false,
        novel: false,
        retry: false,
    };

    pub const RETRY: Pending = Pending {
        asked: false,
        first_look: false,
        conditions: false,
        presence: false,
        novel: false,
        retry: true,
    };

    /// The most important pending reason.
    pub fn reason(&self) -> Option<&'static str> {
        let reasons = [
            (self.asked, "the streamer asked"),
            (self.first_look, "first look"),
            (self.conditions, "conditions changed"),
            (self.presence, "presence changed"),
            (self.novel, "a local detector noticed something new"),
            (self.retry, "retrying after a failure"),
        ];
        reasons.into_iter().find(|(set, _)| *set).map(|(_, reason)| reason)
    }

    pub fn union(self, other: Pending) -> Pending {
        Pending {
            asked: self.asked || other.asked,
            first_look: self.first_look || other.first_look,
            conditions: self.conditions || other.conditions,
            presence: self.presence || other.presence,
            novel: self.novel || other.novel,
            retry: self.retry || other.retry,
        }
    }
}

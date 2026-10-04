// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The error returned when a function table cannot be resolved.

use core::fmt;

/// A required OBS function was not found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadError {
    symbol: &'static str,
}

impl LoadError {
    pub(crate) const fn missing(symbol: &'static str) -> Self {
        Self { symbol }
    }

    /// The C name of the first function that could not be resolved.
    pub fn symbol(&self) -> &'static str {
        self.symbol
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OBS function `{}` was not found in the running process", self.symbol)
    }
}

impl std::error::Error for LoadError {}

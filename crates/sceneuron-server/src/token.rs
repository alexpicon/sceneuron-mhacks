// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Access tokens that keep other local programs and web pages away from the API.

use std::fmt::Write;
use std::hint::black_box;

const TOKEN_BYTES: usize = 16;
const MIN_TOKEN_CHARS: usize = 16;
const MAX_TOKEN_CHARS: usize = 128;

/// Returns a new random token of 32 lowercase hexadecimal characters.
///
/// # Panics
///
/// Panics if the operating system cannot provide random bytes, because no other source
/// can protect the API.
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    if let Err(error) = getrandom::fill(&mut bytes) {
        panic!("the operating system cannot provide random bytes for the access token: {error}");
    }
    let mut token = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        let _ = write!(token, "{byte:02x}");
    }
    token
}

/// Compares a presented token with the expected one.
///
/// The running time depends only on the lengths, never on where the tokens differ.
pub fn tokens_match(expected: &str, given: &str) -> bool {
    let expected = expected.as_bytes();
    let given = given.as_bytes();
    let mut difference = expected.len() ^ given.len();
    for (index, byte) in expected.iter().enumerate() {
        let other = given.get(index).copied().unwrap_or(0);
        difference |= usize::from(black_box(byte ^ other));
    }
    black_box(difference) == 0
}

/// Accepts tokens long enough to resist guessing that need no escaping in a URL.
pub(crate) fn check_token(token: &str) -> Result<(), String> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~');
    let length_ok = (MIN_TOKEN_CHARS..=MAX_TOKEN_CHARS).contains(&token.len());
    if length_ok && token.chars().all(allowed) {
        Ok(())
    } else {
        Err(format!(
            "the token must be {MIN_TOKEN_CHARS}-{MAX_TOKEN_CHARS} characters of A-Z, a-z, 0-9, '-', '_', '.' or '~'"
        ))
    }
}

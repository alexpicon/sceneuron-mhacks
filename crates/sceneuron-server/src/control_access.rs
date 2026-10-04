// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The control token and the pairing codes that hand it out.
//!
//! A control surface such as a Stream Deck gets the control token by pairing: the editor asks
//! for a six-digit code, and the surface trades it once for the token. Revoking replaces the
//! token, which ends every pairing at once; the token's generation lets event streams opened
//! with the old token notice and end. A surface that traded a code or made a request with the
//! token shows that one is paired.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{generate_token, hello, lock, tokens_match};

/// How long a pairing code works after the editor asked for it.
pub(crate) const PAIRING_LIFETIME: Duration = Duration::from_secs(120);

/// Wrong codes that burn a pairing code, so guessing one of a million codes has five tries.
const WRONG_CODES_TO_BURN: u8 = 5;

/// The largest multiple of a million below 2^32. Random values at or above it are drawn again,
/// so every code from 000000 to 999999 is equally likely.
const CODE_DRAW_LIMIT: u32 = 4_294_000_000;

pub(crate) struct ControlAccess {
    state: Mutex<State>,
    /// Serializes revocations, so the host saves new tokens in the order they take effect.
    revoking: Mutex<()>,
}

struct State {
    token: String,
    /// Counts revocations.
    generation: u64,
    /// The one code that can be traded for the token; issuing another replaces it.
    code: Option<PairingCode>,
    /// Counts the codes traded for the token, so the editor can tell when its code was used.
    pairings: u64,
    /// A control surface holds the token: it traded a code for it or made a request with it.
    paired: bool,
}

struct PairingCode {
    code: String,
    expires: Instant,
    wrong: u8,
}

impl ControlAccess {
    pub(crate) fn new(token: String) -> Self {
        Self {
            state: Mutex::new(State {
                token,
                generation: 0,
                code: None,
                pairings: 0,
                paired: false,
            }),
            revoking: Mutex::new(()),
        }
    }

    /// The generation of the control token when `given` is it.
    pub(crate) fn check(&self, given: &str) -> Option<u64> {
        let state = lock(&self.state);
        tokens_match(&state.token, given).then_some(state.generation)
    }

    /// Like [`ControlAccess::check`] for a request that carries `given`, which, when it is the
    /// token, shows that a control surface holds it.
    pub(crate) fn accept(&self, given: &str) -> Option<u64> {
        let mut state = lock(&self.state);
        let matched = tokens_match(&state.token, given);
        state.paired |= matched;
        matched.then_some(state.generation)
    }

    /// Whether a control surface holds the current token: since the server started or the token
    /// was last revoked, one traded a code for it or made a request with it.
    pub(crate) fn paired(&self) -> bool {
        lock(&self.state).paired
    }

    /// Which control token is current; it changes with every revocation.
    pub(crate) fn generation(&self) -> u64 {
        lock(&self.state).generation
    }

    /// The hello proof for the current control token.
    pub(crate) fn proof(&self, message: &str) -> String {
        let token = lock(&self.state).token.clone();
        hello::proof(&token, message)
    }

    /// A new six-digit pairing code that works until [`PAIRING_LIFETIME`] after `now`,
    /// replacing any earlier one.
    pub(crate) fn issue_code(&self, now: Instant) -> String {
        let code = six_digits();
        lock(&self.state).code = Some(PairingCode {
            code: code.clone(),
            expires: now + PAIRING_LIFETIME,
            wrong: 0,
        });
        code
    }

    /// How many codes were traded for the token since the server started.
    pub(crate) fn pairings(&self) -> u64 {
        lock(&self.state).pairings
    }

    /// Trades a pairing code for the control token. A code works once and only until it
    /// expires, and the fifth wrong code burns it.
    pub(crate) fn pair(&self, given: &str, now: Instant) -> Option<String> {
        let mut state = lock(&self.state);
        let code = state.code.as_mut()?;
        if now >= code.expires {
            state.code = None;
            return None;
        }
        if tokens_match(&code.code, given) {
            state.code = None;
            state.pairings += 1;
            state.paired = true;
            return Some(state.token.clone());
        }
        code.wrong += 1;
        if code.wrong >= WRONG_CODES_TO_BURN {
            state.code = None;
        }
        None
    }

    /// Replaces the control token with a new one that differs from every token in `others`,
    /// and drops the pairing code. `save` must keep the new token first; when it fails, the
    /// old token stays in use and its error is returned.
    pub(crate) fn revoke(&self, others: &[&str], save: impl FnOnce(&str) -> Result<(), String>) -> Result<(), String> {
        let _serial = lock(&self.revoking);
        let token = loop {
            let token = generate_token();
            if !others.contains(&token.as_str()) && self.check(&token).is_none() {
                break token;
            }
        };
        save(&token)?;
        let mut state = lock(&self.state);
        state.token = token;
        state.generation += 1;
        state.code = None;
        state.paired = false;
        Ok(())
    }
}

/// A uniformly random code of six decimal digits.
///
/// # Panics
///
/// Panics if the operating system cannot provide random bytes, like [`generate_token`].
fn six_digits() -> String {
    loop {
        let mut bytes = [0u8; 4];
        if let Err(error) = getrandom::fill(&mut bytes) {
            panic!("the operating system cannot provide random bytes for a pairing code: {error}");
        }
        let value = u32::from_le_bytes(bytes);
        if value < CODE_DRAW_LIMIT {
            return format!("{:06}", value % 1_000_000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "control-token-0123456789";

    fn wrong_code(code: &str) -> String {
        let last = if code.ends_with('0') { '1' } else { '0' };
        format!("{}{last}", &code[..5])
    }

    #[test]
    fn codes_are_six_digits_and_replace_earlier_ones() {
        let access = ControlAccess::new(TOKEN.to_owned());
        let now = Instant::now();
        let first = access.issue_code(now);
        assert!(
            first.len() == 6 && first.bytes().all(|byte| byte.is_ascii_digit()),
            "{first}"
        );
        let second = loop {
            let code = access.issue_code(now);
            if code != first {
                break code;
            }
        };
        assert_eq!(access.pair(&first, now), None);
        assert_eq!(access.pair(&second, now).as_deref(), Some(TOKEN));
    }

    #[test]
    fn a_code_works_once() {
        let access = ControlAccess::new(TOKEN.to_owned());
        let now = Instant::now();
        let code = access.issue_code(now);
        assert_eq!(access.pair(&code, now).as_deref(), Some(TOKEN));
        assert_eq!(access.pair(&code, now), None);
    }

    #[test]
    fn a_code_expires_after_two_minutes() {
        let access = ControlAccess::new(TOKEN.to_owned());
        let issued = Instant::now();
        let code = access.issue_code(issued);
        let late = issued + PAIRING_LIFETIME;
        assert_eq!(access.pair(&code, late), None);
        assert_eq!(access.pair(&code, issued), None, "an expired code is dropped");
        let code = access.issue_code(issued);
        let just_in_time = late - Duration::from_millis(1);
        assert_eq!(access.pair(&code, just_in_time).as_deref(), Some(TOKEN));
    }

    #[test]
    fn the_fifth_wrong_code_burns_the_code() {
        let access = ControlAccess::new(TOKEN.to_owned());
        let now = Instant::now();
        let code = access.issue_code(now);
        for _ in 0..4 {
            assert_eq!(access.pair(&wrong_code(&code), now), None);
        }
        assert_eq!(
            access.pair(&code, now).as_deref(),
            Some(TOKEN),
            "four wrong codes leave it"
        );
        let code = access.issue_code(now);
        for _ in 0..5 {
            assert_eq!(access.pair(&wrong_code(&code), now), None);
        }
        assert_eq!(access.pair(&code, now), None, "the fifth wrong code burned it");
    }

    #[test]
    fn revoking_replaces_the_token_and_drops_the_code() {
        let access = ControlAccess::new(TOKEN.to_owned());
        let now = Instant::now();
        let code = access.issue_code(now);
        let mut saved = String::new();
        let revoked = access.revoke(&["editor-token-0123456789"], |token| {
            saved = token.to_owned();
            Ok(())
        });
        assert_eq!(revoked, Ok(()));
        assert_eq!(access.check(TOKEN), None);
        assert_eq!(access.check(&saved), Some(1));
        assert_eq!(access.generation(), 1);
        assert_eq!(access.pair(&code, now), None);
    }

    #[test]
    fn a_failed_save_keeps_the_old_token_and_code() {
        let access = ControlAccess::new(TOKEN.to_owned());
        let now = Instant::now();
        let code = access.issue_code(now);
        let refused = access.revoke(&[], |_| Err("disk full".to_owned()));
        assert_eq!(refused, Err("disk full".to_owned()));
        assert_eq!(access.check(TOKEN), Some(0));
        assert_eq!(access.pair(&code, now).as_deref(), Some(TOKEN));
    }
}

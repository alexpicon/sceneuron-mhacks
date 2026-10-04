// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The room's rules that need no database: room codes, names, and card ids.

/// Letters and digits for room codes, without the look-alikes 0, O, 1, I, and L.
pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Characters in a fresh room code.
pub const CODE_LENGTH: usize = 6;

/// Fewest and most characters of a room code someone chose, such as CHASKI.
pub const CHOSEN_CODE_CHARS: std::ops::RangeInclusive<usize> = 4..=12;

/// Longest participant name, deck name, or card id, in characters.
pub const MAX_CHARS: usize = 40;

/// A room code made from `CODE_LENGTH` draws of `draw`, each any `u32`.
pub fn make_code(mut draw: impl FnMut() -> u32) -> String {
    (0..CODE_LENGTH)
        .map(|_| {
            let index = draw() as usize % CODE_ALPHABET.len();
            char::from(CODE_ALPHABET[index])
        })
        .collect()
}

/// The code as typed, trimmed and in capitals, when it can be a room code: a fresh one, or one
/// someone chose. Fresh codes leave out look-alikes; chosen ones may use any letters and digits.
pub fn normalize_code(code: &str) -> Result<String, String> {
    let code = code.trim().to_ascii_uppercase();
    let valid = CHOSEN_CODE_CHARS.contains(&code.len()) && code.bytes().all(|byte| byte.is_ascii_alphanumeric());
    if valid {
        Ok(code)
    } else {
        Err(format!(
            "a room code is {} to {} letters and digits, such as K7M2QX or CHASKI; got {code:?}",
            CHOSEN_CODE_CHARS.start(),
            CHOSEN_CODE_CHARS.end()
        ))
    }
}

/// A participant's display name, trimmed: 1 to `MAX_CHARS` characters without control characters.
pub fn check_name(name: &str) -> Result<String, String> {
    check_text("name", name)
}

/// A deck's name, trimmed: 1 to `MAX_CHARS` characters without control characters.
pub fn check_deck_name(name: &str) -> Result<String, String> {
    check_text("deck name", name)
}

/// A card id as the voter's deck names it: 1 to `MAX_CHARS` characters without control characters.
pub fn check_card(card: &str) -> Result<String, String> {
    check_text("card", card)
}

fn check_text(what: &str, text: &str) -> Result<String, String> {
    let text = text.trim();
    let count = text.chars().count();
    if count == 0 || count > MAX_CHARS {
        return Err(format!("the {what} must be 1 to {MAX_CHARS} characters, got {count}"));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("the {what} must not contain control characters"));
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_use_only_the_alphabet_and_cover_all_of_it() {
        let mut next = 0u32;
        let code = make_code(|| {
            next += 1;
            next
        });
        assert_eq!(code, "BCDEFG");
        let mut seen = std::collections::BTreeSet::new();
        for start in 0..CODE_ALPHABET.len() as u32 {
            let mut value = start;
            let code = make_code(|| {
                value += 7;
                value
            });
            assert_eq!(code.len(), CODE_LENGTH);
            seen.extend(code.bytes());
        }
        assert_eq!(seen.len(), CODE_ALPHABET.len());
        for confusing in b"0O1IL" {
            assert!(!CODE_ALPHABET.contains(confusing));
        }
        assert_eq!(make_code(|| u32::MAX).len(), CODE_LENGTH);
    }

    #[test]
    fn codes_are_read_in_any_case_and_spacing() {
        assert_eq!(normalize_code(" k7m2qx ").unwrap(), "K7M2QX");
        assert_eq!(normalize_code("chaski").unwrap(), "CHASKI");
        assert_eq!(normalize_code("TEAM").unwrap(), "TEAM");
        for wrong in ["", "ABC", "ABCDEFGHJKMNP", "K7-2QX", "ÄBCDEF", "K7 2QX"] {
            assert!(normalize_code(wrong).is_err(), "{wrong:?}");
        }
    }

    #[test]
    fn names_and_cards_are_trimmed_and_bounded() {
        assert_eq!(check_name("  Ana ").unwrap(), "Ana");
        assert_eq!(check_card("p5").unwrap(), "p5");
        assert_eq!(check_deck_name("Planning poker").unwrap(), "Planning poker");
        assert!(check_name("   ").is_err());
        assert!(check_name(&"é".repeat(MAX_CHARS)).is_ok());
        assert!(check_name(&"é".repeat(MAX_CHARS + 1)).is_err());
        assert!(check_card("p\u{7}").is_err());
        assert!(check_deck_name("two\nlines").is_err());
    }
}

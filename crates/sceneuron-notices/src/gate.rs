// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The license gate: every crate that ships in Sceneuron must be under a license that lets the
//! plugin be distributed next to libobs, which is GPL-2.0-or-later. docs/development.md says why
//! each license is allowed.

use crate::graph::Package;

/// SPDX ids of the allowed licenses.
pub const ALLOWED: [&str; 12] = [
    "0BSD",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "CC0-1.0",
    "CDLA-Permissive-2.0",
    "IJG",
    "ISC",
    "MIT",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Zlib",
];

/// A package whose license does not pass, with why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub package: Package,
    pub reason: String,
}

/// The packages whose declared license does not pass the gate.
pub fn refusals(packages: &[Package]) -> Vec<Refusal> {
    packages
        .iter()
        .filter_map(|package| {
            let reason = match (package.license.trim(), &package.license_file) {
                ("", Some(file)) => format!("declares only a license file, {file}, which the gate cannot read"),
                ("", None) => "declares no license".to_owned(),
                (expression, _) => match allows(expression) {
                    Ok(true) => return None,
                    Ok(false) => format!("{expression} is not on the allowed list"),
                    Err(error) => format!("cannot read the license {expression:?}: {error}"),
                },
            };
            Some(Refusal {
                package: package.clone(),
                reason,
            })
        })
        .collect()
}

/// Whether an SPDX license expression lets a package ship: `OR` needs one side allowed, `AND`
/// both, and a license `WITH` an exception is not allowed. A `/` between licenses means `OR`, as
/// older manifests write it.
pub fn allows(expression: &str) -> Result<bool, String> {
    let tokens = tokens(expression);
    let mut parser = Parser { tokens, at: 0 };
    let allowed = parser.any()?;
    match parser.tokens.get(parser.at) {
        None => Ok(allowed),
        Some(token) => Err(format!("unexpected {token:?}")),
    }
}

fn tokens(expression: &str) -> Vec<String> {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ").replace('/', " OR ");
    spaced
        .split_whitespace()
        .map(|token| match token.to_ascii_uppercase().as_str() {
            operator @ ("OR" | "AND" | "WITH") => operator.to_owned(),
            _ => token.to_owned(),
        })
        .collect()
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn next_is(&self, token: &str) -> bool {
        self.tokens.get(self.at).is_some_and(|next| next == token)
    }

    /// `all (OR all)*`
    fn any(&mut self) -> Result<bool, String> {
        let mut allowed = self.all()?;
        while self.next_is("OR") {
            self.at += 1;
            allowed |= self.all()?;
        }
        Ok(allowed)
    }

    /// `one (AND one)*`
    fn all(&mut self) -> Result<bool, String> {
        let mut allowed = self.one()?;
        while self.next_is("AND") {
            self.at += 1;
            allowed &= self.one()?;
        }
        Ok(allowed)
    }

    /// `( any )`, or a license id with an optional `WITH exception`.
    fn one(&mut self) -> Result<bool, String> {
        let token = self.tokens.get(self.at).cloned().ok_or("the expression ends early")?;
        self.at += 1;
        let allowed = match token.as_str() {
            "(" => {
                let inner = self.any()?;
                if !self.next_is(")") {
                    return Err("a parenthesis is not closed".to_owned());
                }
                self.at += 1;
                inner
            }
            ")" | "OR" | "AND" | "WITH" => return Err(format!("unexpected {token:?}")),
            id => ALLOWED.contains(&id),
        };
        if self.next_is("WITH") {
            self.at += 2;
            if self.at > self.tokens.len() {
                return Err("an exception is missing after WITH".to_owned());
            }
            return Ok(false);
        }
        Ok(allowed)
    }
}

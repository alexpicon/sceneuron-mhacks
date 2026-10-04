// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Editable element properties and partial patches over them.

use serde::{Deserialize, Serialize};

use crate::model::ValidationError;

/// Horizontal text alignment inside an element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// A looping animation that runs while the element is visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdleAnimation {
    #[default]
    None,
    Pulse,
    Glow,
    Float,
}

/// The animation played when an element becomes visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnterAnimation {
    None,
    #[default]
    Fade,
    SlideUp,
    Pop,
}

macro_rules! element_props {
    ($($field:ident: $ty:ty = $default:expr),+ $(,)?) => {
        /// Every editable property of an overlay element.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct ElementProps {
            $(pub $field: $ty,)+
        }

        impl Default for ElementProps {
            fn default() -> Self {
                Self { $($field: $default,)+ }
            }
        }

        impl ElementProps {
            /// Overwrites every field that `patch` sets.
            pub fn apply(&mut self, patch: &PropsPatch) {
                $(if let Some(value) = &patch.$field {
                    self.$field = value.clone();
                })+
            }
        }

        /// A partial set of element properties; unset fields leave the target unchanged.
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
        #[serde(default, deny_unknown_fields)]
        pub struct PropsPatch {
            $(#[serde(skip_serializing_if = "Option::is_none")]
            pub $field: Option<$ty>,)+
        }

        impl PropsPatch {
            /// Returns true when the patch sets no field.
            pub fn is_empty(&self) -> bool {
                true $(&& self.$field.is_none())+
            }

            /// Copies every field that `other` sets over this patch.
            pub fn merge(&mut self, other: &PropsPatch) {
                $(if let Some(value) = &other.$field {
                    self.$field = Some(value.clone());
                })+
            }

            /// Names of the fields this patch sets, as they appear in JSON.
            pub fn field_names(&self) -> Vec<&'static str> {
                let mut names = Vec::new();
                $(if self.$field.is_some() {
                    names.push(stringify!($field));
                })+
                names
            }
        }
    };
}

element_props! {
    x: f64 = 0.0,
    y: f64 = 0.0,
    width: f64 = 480.0,
    height: f64 = 120.0,
    visible: bool = true,
    text: String = String::new(),
    font_size: f64 = 48.0,
    bold: bool = true,
    color: String = "#ffffff".to_owned(),
    background: String = "#00000000".to_owned(),
    border_color: String = "#00000000".to_owned(),
    border_width: f64 = 0.0,
    corner_radius: f64 = 0.0,
    opacity: f64 = 1.0,
    align: TextAlign = TextAlign::Center,
    idle: IdleAnimation = IdleAnimation::None,
    enter: EnterAnimation = EnterAnimation::Fade,
}

const MAX_TEXT_CHARS: usize = 1000;

impl ElementProps {
    /// Checks that every field holds a value the overlay can render safely.
    pub fn validate(&self) -> Result<(), ValidationError> {
        check_range("x", self.x, -10_000.0, 10_000.0)?;
        check_range("y", self.y, -10_000.0, 10_000.0)?;
        check_range("width", self.width, 1.0, 10_000.0)?;
        check_range("height", self.height, 1.0, 10_000.0)?;
        check_range("font_size", self.font_size, 1.0, 1_000.0)?;
        check_range("border_width", self.border_width, 0.0, 200.0)?;
        check_range("corner_radius", self.corner_radius, 0.0, 5_000.0)?;
        check_range("opacity", self.opacity, 0.0, 1.0)?;
        check_color("color", &self.color)?;
        check_color("background", &self.background)?;
        check_color("border_color", &self.border_color)?;
        check_text(&self.text)
    }
}

impl PropsPatch {
    /// Checks every field the patch sets, using the same limits as [`ElementProps`].
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut props = ElementProps::default();
        props.apply(self);
        props.validate()
    }
}

fn check_range(field: &str, value: f64, min: f64, max: f64) -> Result<(), ValidationError> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        let message = format!("{field} must be between {min} and {max}, got {value}");
        Err(ValidationError(message))
    }
}

/// Accepts CSS hex colors: `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`.
fn check_color(field: &str, value: &str) -> Result<(), ValidationError> {
    let digits = value.strip_prefix('#').unwrap_or("");
    let valid_length = matches!(digits.len(), 3 | 4 | 6 | 8);
    if valid_length && digits.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        let message = format!("{field} must be a hex color such as #ffcc00, got {value:?}");
        Err(ValidationError(message))
    }
}

fn check_text(text: &str) -> Result<(), ValidationError> {
    if text.chars().count() > MAX_TEXT_CHARS {
        let message = format!("text must be at most {MAX_TEXT_CHARS} characters");
        return Err(ValidationError(message));
    }
    if text.chars().any(|c| c.is_control() && c != '\n') {
        let message = "text must not contain control characters other than newlines".to_owned();
        return Err(ValidationError(message));
    }
    Ok(())
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! BT.601 YUV to RGB conversion.

/// Limited-range luma scale, 255 / 219.
const LIMITED_LUMA: f32 = 1.164_383_6;
/// Limited-range chroma scale, 255 / 224.
const LIMITED_CHROMA: f32 = 1.138_392_9;

/// BT.601 chroma-to-RGB weights for full-range chroma centered on zero.
const V_TO_R: f32 = 1.402;
const U_TO_G: f32 = 0.344_136;
const V_TO_G: f32 = 0.714_136;
const U_TO_B: f32 = 1.772;

/// How averaged channel values become RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    /// Channels are already R, G, B.
    Rgb,
    /// Channels are Y, U, V with BT.601 coefficients.
    Yuv { full_range: bool },
}

impl Encoding {
    /// Converts one averaged pixel to clamped 8-bit RGB.
    #[inline]
    pub(crate) fn to_rgb(self, channels: [f32; 3]) -> [u8; 3] {
        match self {
            Encoding::Rgb => channels.map(to_byte),
            Encoding::Yuv { full_range } => yuv_to_rgb(channels, full_range),
        }
    }
}

fn yuv_to_rgb([y, u, v]: [f32; 3], full_range: bool) -> [u8; 3] {
    let (luma, chroma_scale) = if full_range {
        (y, 1.0)
    } else {
        ((y - 16.0) * LIMITED_LUMA, LIMITED_CHROMA)
    };
    let u = (u - 128.0) * chroma_scale;
    let v = (v - 128.0) * chroma_scale;
    [
        to_byte(luma + V_TO_R * v),
        to_byte(luma - U_TO_G * u - V_TO_G * v),
        to_byte(luma + U_TO_B * u),
    ]
}

#[inline]
fn to_byte(value: f32) -> u8 {
    // Float-to-int `as` casts saturate, and NaN becomes 0.
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_range_extremes_map_to_black_and_white() {
        let yuv = Encoding::Yuv { full_range: false };
        assert_eq!(yuv.to_rgb([16.0, 128.0, 128.0]), [0, 0, 0]);
        assert_eq!(yuv.to_rgb([235.0, 128.0, 128.0]), [255, 255, 255]);
    }

    #[test]
    fn full_range_gray_is_identity() {
        let yuv = Encoding::Yuv { full_range: true };
        for level in [0.0, 77.0, 255.0] {
            let byte = level as u8;
            assert_eq!(yuv.to_rgb([level, 128.0, 128.0]), [byte; 3]);
        }
    }

    #[test]
    fn out_of_gamut_values_clamp() {
        let yuv = Encoding::Yuv { full_range: true };
        assert_eq!(yuv.to_rgb([255.0, 255.0, 255.0])[0], 255);
        assert_eq!(yuv.to_rgb([0.0, 0.0, 0.0])[0], 0);
        assert_eq!(Encoding::Rgb.to_rgb([f32::NAN, -4.0, 300.0]), [0, 0, 255]);
    }
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Wraps packed RGB24 frames in CoreGraphics images for Vision.

use std::ptr;

use objc2_core_foundation::{CFData, CFRetained};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo, kCGColorSpaceSRGB,
};
use sceneuron_vision::{DetectError, check_rgb};

/// The sRGB color space camera frames are tagged with.
pub(crate) fn srgb() -> Result<CFRetained<CGColorSpace>, DetectError> {
    // SAFETY: `kCGColorSpaceSRGB` is an immutable CFString constant exported by CoreGraphics.
    let name = unsafe { kCGColorSpaceSRGB };
    CGColorSpace::with_name(Some(name)).ok_or_else(|| DetectError::Load("CoreGraphics has no sRGB color space".into()))
}

/// Copies one `width` x `height` RGB24 frame into a new image.
pub(crate) fn rgb_image(
    rgb: &[u8],
    width: u32,
    height: u32,
    space: &CGColorSpace,
) -> Result<CFRetained<CGImage>, DetectError> {
    check_rgb(rgb, width, height)?;
    let failed =
        |what: &str| DetectError::Inference(format!("could not create the {what} for a {width}x{height} frame"));
    let data = CFData::from_bytes(rgb);
    let provider = CGDataProvider::with_cf_data(Some(&data)).ok_or_else(|| failed("image data provider"))?;
    let (width, height) = (width as usize, height as usize);
    // SAFETY: `check_rgb` proved the provider holds exactly `height` tightly packed rows of
    // `width * 3` bytes, which matches 8 bits per component, 24 bits per pixel, and no
    // alpha; `decode` may be null.
    let image = unsafe {
        CGImage::new(
            width,
            height,
            8,
            24,
            width * 3,
            Some(space),
            CGBitmapInfo(CGImageAlphaInfo::None.0),
            Some(&provider),
            ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    };
    image.ok_or_else(|| failed("image"))
}

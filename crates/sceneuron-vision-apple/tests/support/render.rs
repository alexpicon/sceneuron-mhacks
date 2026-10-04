// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Renders test frames with CoreGraphics and CoreText, so text fixtures are reproducible.

use std::ptr;

use objc2_core_foundation::{CFAttributedString, CFDictionary, CFString, CFType};
use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGImageAlphaInfo, kCGColorSpaceSRGB};
use objc2_core_text::{CTFont, CTLine, kCTFontAttributeName};

/// A white `width` x `height` RGB24 frame with each `(text, x, y)` drawn in black bold
/// Helvetica `size` points tall; `x` and `y` place the baseline, from the bottom left.
pub fn text_frame(width: u32, height: u32, size: f64, lines: &[(&str, f64, f64)]) -> Vec<u8> {
    let (width, height) = (width as usize, height as usize);
    let mut rgbx = vec![255u8; width * height * 4];
    // SAFETY: `kCGColorSpaceSRGB` is an immutable CFString constant exported by CoreGraphics.
    let space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB })).expect("sRGB exists");
    // SAFETY: `rgbx` holds `height` rows of `width * 4` bytes, matching 8 bits per component
    // with a skipped fourth byte, and it outlives the context, which is dropped below.
    let context = unsafe {
        CGBitmapContextCreate(
            rgbx.as_mut_ptr().cast(),
            width,
            height,
            8,
            width * 4,
            Some(&space),
            CGImageAlphaInfo::NoneSkipLast.0,
        )
    }
    .expect("bitmap context");
    let name = CFString::from_static_str("Helvetica-Bold");
    // SAFETY: a null matrix asks for the identity transform.
    let font = unsafe { CTFont::with_name(&name, size, ptr::null()) };
    // SAFETY: `kCTFontAttributeName` is an immutable CFString constant exported by CoreText.
    let key = unsafe { kCTFontAttributeName };
    let attributes = CFDictionary::<CFString, CFType>::from_slices(&[key], &[&**font]);
    for &(text, x, y) in lines {
        let string = CFString::from_str(text);
        // SAFETY: the attributes map the font attribute name to a CTFont, as CoreText expects.
        let attributed = unsafe { CFAttributedString::new(None, Some(&string), Some(attributes.as_opaque())) }
            .expect("attributed string");
        // SAFETY: the attributed string is valid for the duration of the call.
        let line = unsafe { CTLine::with_attributed_string(&attributed) };
        objc2_core_graphics::CGContext::set_text_position(Some(&context), x, y);
        // SAFETY: the context draws into `rgbx`, which is still alive.
        unsafe { line.draw(&context) };
    }
    drop(context);
    let (pixels, _) = rgbx.as_chunks::<4>();
    pixels
        .iter()
        .flat_map(|&[red, green, blue, _]| [red, green, blue])
        .collect()
}

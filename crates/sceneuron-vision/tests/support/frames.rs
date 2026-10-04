// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Builds raw frames of every pixel format from an RGB image, for conversion tests.

use sceneuron_vision::{FrameRef, PixelFormat};

/// Every format the converter supports.
pub const ALL_FORMATS: [PixelFormat; 12] = [
    PixelFormat::I420,
    PixelFormat::Nv12,
    PixelFormat::Yvyu,
    PixelFormat::Yuy2,
    PixelFormat::Uyvy,
    PixelFormat::Rgba,
    PixelFormat::Bgra,
    PixelFormat::Bgrx,
    PixelFormat::Y800,
    PixelFormat::I444,
    PixelFormat::Bgr3,
    PixelFormat::I422,
];

/// Byte written into row padding and unused alpha bytes; the converter must ignore it.
const JUNK: u8 = 0xA5;

#[derive(Debug, Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 3]>,
}

impl Image {
    pub fn from_fn(width: usize, height: usize, color: impl Fn(usize, usize) -> [u8; 3]) -> Image {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| color(x, y))
            .collect();
        Image { width, height, pixels }
    }

    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        self.pixels[y * self.width + x]
    }
}

/// Owned planes of an encoded frame; each plane is exactly as long as its rows require.
#[derive(Debug, Clone)]
pub struct Encoded {
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub planes: Vec<Vec<u8>>,
    pub strides: Vec<usize>,
    pub full_range: bool,
}

impl Encoded {
    pub fn frame(&self) -> FrameRef<'_> {
        let mut planes: [&[u8]; 4] = [&[]; 4];
        let mut strides = [0; 4];
        for (index, plane) in self.planes.iter().enumerate() {
            planes[index] = plane;
            strides[index] = self.strides[index];
        }
        FrameRef {
            format: self.format,
            width: self.width,
            height: self.height,
            planes,
            strides,
            full_range: self.full_range,
        }
    }
}

/// BT.601 forward transform, rounded to bytes.
pub fn rgb_to_yuv([r, g, b]: [u8; 3], full_range: bool) -> [u8; 3] {
    let (r, g, b) = (f64::from(r), f64::from(g), f64::from(b));
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = (b - y) / 1.772;
    let v = (r - y) / 1.402;
    let (y, u, v) = if full_range {
        (y, u, v)
    } else {
        (16.0 + y * 219.0 / 255.0, u * 224.0 / 255.0, v * 224.0 / 255.0)
    };
    [y, u + 128.0, v + 128.0].map(|value| value.round().clamp(0.0, 255.0) as u8)
}

/// Produces the bytes of one unit (a pixel, chroma sample, or macropixel) of a plane
/// from its column and row.
type UnitWriter<'a> = Box<dyn Fn(usize, usize) -> Vec<u8> + 'a>;

/// Encodes `image` as `format`, adding `pad` junk bytes to every row's stride.
///
/// Chroma for subsampled formats comes from the top-left pixel of each block, so images
/// whose colors are constant over aligned 2x2 blocks encode without chroma loss.
pub fn encode(image: &Image, format: PixelFormat, full_range: bool, pad: usize) -> Encoded {
    let (w, h) = (image.width, image.height);
    let yuv = |x: usize, y: usize| rgb_to_yuv(image.at(x, y), full_range);
    let (half_w, half_h) = (w.div_ceil(2), h.div_ceil(2));
    let chroma = |channel: usize, x_step: usize, y_step: usize| {
        move |x: usize, y: usize| vec![yuv(x * x_step, y * y_step)[channel]]
    };
    let rgb = |offsets: [usize; 3], bytes: usize| {
        move |x: usize, y: usize| {
            let mut pixel = vec![JUNK; bytes];
            for (channel, offset) in offsets.into_iter().enumerate() {
                pixel[offset] = image.at(x, y)[channel];
            }
            pixel
        }
    };
    let packed = |order: [usize; 4]| {
        // `order` names the channel of each byte: 0 = Y0, 1 = U, 2 = Y1, 3 = V.
        move |m: usize, y: usize| {
            let (left, right) = (yuv(2 * m, y), yuv((2 * m + 1).min(w - 1), y));
            order.map(|slot| [left[0], left[1], right[0], left[2]][slot]).to_vec()
        }
    };
    // Each plane: units per row, rows, and the writer for one unit.
    let planes: Vec<(usize, usize, UnitWriter)> = match format {
        PixelFormat::I420 => vec![
            (w, h, Box::new(move |x, y| vec![yuv(x, y)[0]])),
            (half_w, half_h, Box::new(chroma(1, 2, 2))),
            (half_w, half_h, Box::new(chroma(2, 2, 2))),
        ],
        PixelFormat::I422 => vec![
            (w, h, Box::new(move |x, y| vec![yuv(x, y)[0]])),
            (half_w, h, Box::new(chroma(1, 2, 1))),
            (half_w, h, Box::new(chroma(2, 2, 1))),
        ],
        PixelFormat::I444 => vec![
            (w, h, Box::new(chroma(0, 1, 1))),
            (w, h, Box::new(chroma(1, 1, 1))),
            (w, h, Box::new(chroma(2, 1, 1))),
        ],
        PixelFormat::Nv12 => vec![
            (w, h, Box::new(move |x, y| vec![yuv(x, y)[0]])),
            (half_w, half_h, Box::new(move |x, y| yuv(2 * x, 2 * y)[1..].to_vec())),
        ],
        PixelFormat::Yuy2 => vec![(half_w, h, Box::new(packed([0, 1, 2, 3])))],
        PixelFormat::Yvyu => vec![(half_w, h, Box::new(packed([0, 3, 2, 1])))],
        PixelFormat::Uyvy => vec![(half_w, h, Box::new(packed([1, 0, 3, 2])))],
        PixelFormat::Y800 => vec![(w, h, Box::new(move |x, y| vec![yuv(x, y)[0]]))],
        PixelFormat::Rgba => vec![(w, h, Box::new(rgb([0, 1, 2], 4)))],
        PixelFormat::Bgra | PixelFormat::Bgrx => vec![(w, h, Box::new(rgb([2, 1, 0], 4)))],
        PixelFormat::Bgr3 => vec![(w, h, Box::new(rgb([2, 1, 0], 3)))],
    };
    let mut encoded = Encoded {
        format,
        width: w as u32,
        height: h as u32,
        planes: Vec::new(),
        strides: Vec::new(),
        full_range,
    };
    for (units, rows, unit) in planes {
        let row_bytes = units * unit(0, 0).len();
        let stride = row_bytes + pad;
        let mut data = Vec::with_capacity(stride * rows);
        for y in 0..rows {
            (0..units).for_each(|x| data.extend(unit(x, y)));
            if y + 1 < rows {
                data.resize(data.len() + pad, JUNK);
            }
        }
        encoded.planes.push(data);
        encoded.strides.push(stride);
    }
    encoded
}

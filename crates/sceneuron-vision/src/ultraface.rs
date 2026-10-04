// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The bundled UltraFace RFB-320 face detector, run on the CPU with tract.

use std::fmt;
use std::io::Cursor;

use tract_onnx::prelude::*;

use crate::detect::{DetectError, InputSize, Perceiver};
use crate::nms::decode;
use crate::perception::{FaceBox, Perception};
use crate::{INPUT_HEIGHT, INPUT_WIDTH};

static MODEL: &[u8] = include_bytes!("../models/ultraface-rfb-320.onnx");

const WIDTH: usize = INPUT_WIDTH as usize;
const HEIGHT: usize = INPUT_HEIGHT as usize;
const PIXELS: usize = WIDTH * HEIGHT;
const INPUT_SHAPE: [usize; 4] = [1, 3, HEIGHT, WIDTH];

/// Detects faces in [`INPUT_WIDTH`] x [`INPUT_HEIGHT`] RGB frames. It fills only
/// [`Perception::faces`] and [`Perception::best_face`].
pub struct UltraFace {
    plan: Arc<TypedRunnableModel>,
    /// Reused CHW input buffer, normalized as the model was trained.
    input: Vec<f32>,
}

impl UltraFace {
    /// Loads and optimizes the embedded model. This takes tens of milliseconds, so pass
    /// `UltraFace::new` to [`crate::VisionWorker::spawn`] to run it off the caller's thread.
    pub fn new() -> Result<Self, DetectError> {
        let plan = load().map_err(|error| DetectError::Load(format!("{error:#}")))?;
        Ok(UltraFace {
            plan,
            input: vec![0.0; 3 * PIXELS],
        })
    }
}

impl fmt::Debug for UltraFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UltraFace").finish_non_exhaustive()
    }
}

fn load() -> TractResult<Arc<TypedRunnableModel>> {
    tract_onnx::onnx()
        .model_for_read(&mut Cursor::new(MODEL))?
        .with_input_fact(0, f32::fact(INPUT_SHAPE).into())?
        .into_optimized()?
        .into_runnable()
}

impl Perceiver for UltraFace {
    fn name(&self) -> &'static str {
        "UltraFace"
    }

    fn input_size(&self) -> InputSize {
        InputSize::Fixed {
            width: INPUT_WIDTH,
            height: INPUT_HEIGHT,
        }
    }

    fn perceive(&mut self, rgb: &[u8], width: u32, height: u32) -> Result<Perception, DetectError> {
        if width != INPUT_WIDTH || height != INPUT_HEIGHT || rgb.len() != 3 * PIXELS {
            return Err(DetectError::InputSize {
                expected_width: INPUT_WIDTH,
                expected_height: INPUT_HEIGHT,
                width,
                height,
                len: rgb.len(),
            });
        }
        let (red, rest) = self.input.split_at_mut(PIXELS);
        let (green, blue) = rest.split_at_mut(PIXELS);
        let (pixels, _) = rgb.as_chunks::<3>();
        for (index, [r, g, b]) in pixels.iter().enumerate() {
            red[index] = normalize(*r);
            green[index] = normalize(*g);
            blue[index] = normalize(*b);
        }
        let input = Tensor::from_shape(&INPUT_SHAPE, &self.input).map_err(inference)?;
        let outputs = self.plan.run(tvec!(input.into())).map_err(inference)?;
        let [scores, boxes, ..] = outputs.as_slice() else {
            return Err(DetectError::Inference(format!(
                "model returned {} outputs",
                outputs.len()
            )));
        };
        let scores = scores.to_plain_array_view::<f32>().map_err(inference)?;
        let boxes = boxes.to_plain_array_view::<f32>().map_err(inference)?;
        let anchors = match (scores.shape(), boxes.shape()) {
            (&[1, n, 2], &[1, m, 4]) if n == m => n,
            (scores, boxes) => {
                return Err(DetectError::Inference(format!(
                    "unexpected output shapes {scores:?} and {boxes:?}"
                )));
            }
        };
        Ok(decode((0..anchors).map(|i| FaceBox {
            x1: boxes[[0, i, 0]],
            y1: boxes[[0, i, 1]],
            x2: boxes[[0, i, 2]],
            y2: boxes[[0, i, 3]],
            score: scores[[0, i, 1]],
        })))
    }
}

#[inline]
fn normalize(value: u8) -> f32 {
    (f32::from(value) - 127.0) / 128.0
}

fn inference(error: TractError) -> DetectError {
    DetectError::Inference(format!("{error:#}"))
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Runs the Apple Vision perceiver on one raw RGB24 frame, prints what it found, and times
//! every analysis.
//!
//! ```sh
//! cargo run --release -p sceneuron-vision-apple --example perceive -- frame.rgb 640 360 [runs]
//! ```
//!
//! Frames that also run image labels and text are timed separately from the others.

use std::time::Instant;

use sceneuron_vision::{Perceiver, facts};
use sceneuron_vision_apple::AppleVision;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [path, width, height, rest @ ..] = args.as_slice() else {
        return Err("usage: perceive FRAME.rgb WIDTH HEIGHT [RUNS]".into());
    };
    let (width, height): (u32, u32) = (width.parse()?, height.parse()?);
    let runs: u32 = rest.first().map_or(Ok(20), |runs| runs.parse())?;
    let rgb = std::fs::read(path)?;

    let started = Instant::now();
    let mut vision = AppleVision::new()?;
    println!("started in {:.0} ms with {:?}", ms(started), vision.active());
    for reason in vision.skipped() {
        println!("skipped: {reason}");
    }
    let first = Instant::now();
    let perception = vision.perceive(&rgb, width, height)?;
    println!("first frame in {:.1} ms: {perception:#?}", ms(first));
    for fact in facts(&perception) {
        println!("fact: {fact}");
    }

    // Means of image, Vision, and reading time, and the frame count, for fast and slow frames.
    let mut sums = [[0.0f64; 4]; 2];
    let started = Instant::now();
    for _ in 0..runs {
        vision.perceive(&rgb, width, height)?;
        let timing = vision.last_timing().ok_or("no timing after a frame")?;
        let sum = &mut sums[usize::from(timing.slow)];
        sum[0] += timing.image_ms;
        sum[1] += timing.vision_ms;
        sum[2] += timing.read_ms;
        sum[3] += 1.0;
    }
    let per_frame = ms(started) / f64::from(runs);
    println!("{width}x{height}, {runs} frames (image + Vision + reading results):");
    for (sum, kind) in sums.iter().zip(["without labels and text", "with labels and text"]) {
        if sum[3] > 0.0 {
            let [image, vision, read, count] = sum.map(|value| value / sum[3]);
            println!(
                "  {:3.0} frames {kind}: {image:.1} + {vision:.1} + {read:.1} ms",
                count * sum[3]
            );
        }
    }
    println!("  mean per frame: {per_frame:.1} ms");
    Ok(())
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the view of OBS filter audio as float planes for the voice input: the output's
//! channels are borrowed as they are, and blocks with missing or misaligned planes are refused.

use sceneuron::audio_block::planes;
use sceneuron_obs_sys as sys;

/// A block whose planes point into owned float buffers.
struct TestBlock {
    block: sys::obs_audio_data,
    _planes: Vec<Vec<f32>>,
}

fn test_block(planes: Vec<Vec<f32>>) -> TestBlock {
    let mut block = sys::obs_audio_data::default();
    let mut planes = planes;
    block.frames = planes.first().map_or(0, Vec::len) as u32;
    for (index, plane) in planes.iter_mut().enumerate() {
        block.data[index] = plane.as_mut_ptr().cast::<u8>();
    }
    TestBlock { block, _planes: planes }
}

#[test]
fn the_output_channels_are_borrowed_as_they_are() {
    let test = test_block(vec![vec![1.0, 0.5, 0.0], vec![0.0, 0.5, -1.0]]);
    // SAFETY: both planes hold `frames` floats for as long as `test` lives.
    let stereo = unsafe { planes(&test.block, 2) }.unwrap();
    assert_eq!(stereo.as_slice(), [&[1.0, 0.5, 0.0][..], &[0.0, 0.5, -1.0][..]]);
    // SAFETY: the first plane holds `frames` floats for as long as `test` lives.
    let mono = unsafe { planes(&test.block, 1) }.unwrap();
    assert_eq!(mono.as_slice(), [&[1.0, 0.5, 0.0][..]]);
    // SAFETY: no plane is read for zero channels.
    assert!(unsafe { planes(&test.block, 0) }.unwrap().as_slice().is_empty());
}

#[test]
fn a_missing_or_misaligned_plane_is_refused() {
    let test = test_block(vec![vec![0.5; 4]]);
    // SAFETY: the second plane is null, which the function checks before reading.
    assert!(unsafe { planes(&test.block, 2) }.is_none());
    let mut misaligned = test_block(vec![vec![0.5; 4]]);
    // SAFETY: one byte into a live buffer; the function refuses it before reading.
    misaligned.block.data[0] = unsafe { misaligned.block.data[0].add(1) };
    // SAFETY: the plane pointer is checked for alignment before any read.
    assert!(unsafe { planes(&misaligned.block, 1) }.is_none());
}

#[test]
fn more_channels_than_planes_count_as_the_planes_there_are() {
    let owned: Vec<Vec<f32>> = (0..sys::MAX_AV_PLANES).map(|index| vec![index as f32; 2]).collect();
    let test = test_block(owned);
    // SAFETY: every plane holds `frames` floats for as long as `test` lives.
    let all = unsafe { planes(&test.block, 64) }.unwrap();
    assert_eq!(all.as_slice().len(), sys::MAX_AV_PLANES);
    assert_eq!(all.as_slice()[7], [7.0, 7.0]);
}

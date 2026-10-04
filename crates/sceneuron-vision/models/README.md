# Bundled model

`ultraface-rfb-320.onnx` is the Ultra-Light-Fast-Generic-Face-Detector-1MB
RFB-320 face detector, copied unmodified from
<https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB/blob/master/models/onnx/version-RFB-320.onnx>.

- License: MIT, Copyright (c) 2019 linzai. The full text is in
  [`LICENSE-ultraface.txt`](LICENSE-ultraface.txt).
- SHA-256: `34cd7e60aeff28744c657de7a3dc64e872d506741de66987f3426f2b79f88017`

The crate embeds the file at compile time, so replacing it changes the shipped
detector. Verify the checksum after any update:

```sh
shasum -a 256 crates/sceneuron-vision/models/ultraface-rfb-320.onnx
```

Model contract relied on by `src/ultraface.rs`:

- Input: `[1, 3, 240, 320]` float RGB in CHW order, each value `(v - 127) / 128`.
- Output 0: scores `[1, 4420, 2]`; index 1 of the last axis is the face probability.
- Output 1: boxes `[1, 4420, 4]`; normalized `x1, y1, x2, y2`.

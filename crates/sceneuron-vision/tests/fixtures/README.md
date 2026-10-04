# Test fixtures

Raw packed RGB24 frames, 320x240, row-major with no padding (230,400 bytes each).
They were derived with ffmpeg from the public-domain NASA portrait of Neil
Armstrong, <https://commons.wikimedia.org/wiki/File:Neil_Armstrong_pose.jpg>
(NASA Photo ID S69-31741).

| File | Content | Detector expectation |
| --- | --- | --- |
| `face-close-320x240.rgb` | Webcam-style crop around the head | one face |
| `face-320x240.rgb` | Full portrait, letterboxed to 4:3 | one face |
| `empty-320x240.rgb` | Lower crop of the portrait without a face | no face |

To inspect one, convert it back to an image:

```sh
ffmpeg -f rawvideo -pixel_format rgb24 -video_size 320x240 -i face-320x240.rgb face.png
```

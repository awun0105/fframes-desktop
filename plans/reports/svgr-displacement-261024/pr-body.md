`feDisplacementMap` draws garbage. A uniform map that should move an element by a few pixels
moves it by a hundred, or out of the filter region. fframes renders the same frame correctly
with its Skia backend, so the CPU and Skia renders of a video disagree.

This is the same fix as linebender/resvg#1157 (upstream issues linebender/resvg#1160 and
linebender/resvg#1019), ported to svgr.

### Two bugs

1. **`scale` is applied twice.** `apply_displacement_map` converts the scale to pixels with
   `scale_coordinates(fe.scale(), fe.scale(), ts)`, and `displacement_map::apply` then
   multiplied the offset by `fe.scale()` again. Pixels moved by `scale² × (C − 0.5)`: at
   `scale="20"` that is 400 × (C − 0.5) px instead of 20 × (C − 0.5).
2. **The map is read premultiplied.** Filter Effects 1 says "The calculations using the pixel
   values from `in2` are performed using non-premultiplied color values", and
   `displacement_map::apply`'s doc comment asks for the same, but the caller passed the
   premultiplied pixmap. Semi-transparent maps, such as `feTurbulence type="turbulence"`
   output, then pulled every offset toward the top left.

### Before and after

The new tests draw the expected position as a dashed red outline. Box of the blue pixels on a
200 × 200 canvas:

| Test | Expected | Before | After |
| --- | --- | --- | --- |
| `uniform-map` (`#c0c0c0`, scale 20) | x 45..=144, y 45..=144 | x 0..=48, y 0..=48 | x 45..=144, y 45..=144 |
| `semi-transparent-map` (white at 50%, scale 20) | x 40..=139, y 40..=139 | x 49..=148, y 49..=148 | x 40..=139, y 40..=139 |

Through fframes: a 960 × 540 frame with a uniform map, a `feTurbulence type="turbulence"` map and
an opaque `fractalNoise` map at scale 29, compared with fframes' Skia-CPU render (odiff,
threshold 0.15):

| fframes `--renderer cpu` | Pixels that differ from `--renderer skia-cpu` |
| --- | --- |
| svgr 0.50.2 | 57.27% |
| svgr 0.50.2 with this fix | 0.50% |

(Comparison image below.)

### Tests

- New tests `filters/feDisplacementMap/uniform-map` and `semi-transparent-map`, references made
  by the harness (`MAKE_REF=1`) and compressed with `oxipng -o 6 -Z`.
- `cargo test -p svgr --release`: 1,707 passed, 0 failed.
- The two new tests come without `-actual.png` files; the harness writes those on every run.

### Not in this PR

After the fix, 0.50% of the pixels of the fframes frame still differ from Skia-CPU: thin lines
along the edges of displaced shapes, almost all in the turbulence panel. svgr samples the
nearest whole pixel (the `TODO` about anti-aliasing in `displacement_map.rs`), which is the
likely cause. resvg#1157 also adds `scale=0`, `turbulence-map` and
`primitiveUnits=objectBoundingBox` tests; they can follow separately.

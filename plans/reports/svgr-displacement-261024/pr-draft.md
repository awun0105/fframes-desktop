# Draft: fix `feDisplacementMap` in svgr

Where to file it:

- **The PR goes to `dmtrKovalenko/svgr`, branch `main`.** That repository has issues turned
  off, so the PR description below is also the bug report.
- `svgr` 0.50.2 (the version the root `Cargo.toml` pins) and svgr's `main` (`48043661`,
  2026-10-16) both have the bug.
- The same bug is reported upstream in resvg: issue
  [linebender/resvg#1160](https://github.com/linebender/resvg/issues/1160) and the older
  [#1019](https://github.com/linebender/resvg/issues/1019). The fix is open as
  [linebender/resvg#1157](https://github.com/linebender/resvg/pull/1157), not merged on
  2026-10-24. svgr is a fork of resvg, so this PR ports that fix.
- After a svgr release, fframes needs `svgr` and `usvgr` bumped in its root `Cargo.toml`.
  The optional issue text for `dmtrKovalenko/fframes` is at the end.

Files next to this draft:

- `uniform-map.svg`, `semi-transparent-map.svg`: the two test cases, each with an expected
  outline;
- `fframes-repro.rs`: an fframes video that shows the bug with `--renderer cpu` against
  `--renderer skia-cpu`;
- `fix.patch`: the minimal change, made against svgr 0.50.2 (the same lines on svgr's `main`).
  This is the version that was tested. The diff in the PR text adds the parameter renames and
  comments of resvg#1157; either can go into the PR;
- `before-after.png`: the fframes CPU render before and after the fix, with Skia below.

---

## PR title

```
fix(svgr): apply the feDisplacementMap scale once and read the map unpremultiplied
```

## PR description

`feDisplacementMap` draws garbage on the CPU renderer. A uniform map that should move an
element by a few pixels moves it by a hundred, or off the filter region entirely. fframes
renders the same frame correctly with its Skia backend, so the CPU and Skia renders of a video
disagree.

### Two bugs

Both are in `crates/svgr/src/filter/`, inherited from resvg.

1. **`scale` is applied twice.**
   - `apply_displacement_map` (`mod.rs`) converts the scale to pixels with
     `scale_coordinates(fe.scale(), fe.scale(), ts)`, which already returns
     `fe.scale() * ts_scale`.
   - `displacement_map::apply` then computes `dx * sx * fe.scale()`, so the shift grows with
     `scale²`. At `scale="20"` a pixel moves 400 × (C − 0.5) px instead of 20 × (C − 0.5).
2. **The map is read premultiplied.**
   - `displacement_map::apply` says in its doc comment that the map must be unpremultiplied,
     and Filter Effects 1 says the same: "The calculations using the pixel values from `in2`
     are performed using non-premultiplied color values."
   - The caller passes the premultiplied pixmap. Maps with alpha below 1, such as the output
     of `feTurbulence type="turbulence"`, then pull every offset toward the top left.

### How to reproduce

Render `uniform-map.svg` (below). The blue square should sit on the dashed red outline, moved
by 20 × (0.753 − 0.5) ≈ 5 px up and left.

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
    <filter id="f" x="0" y="0" width="200" height="200" filterUnits="userSpaceOnUse"
            color-interpolation-filters="sRGB">
        <feFlood flood-color="#c0c0c0" result="map"/>
        <feDisplacementMap in="SourceGraphic" in2="map" scale="20"
                           xChannelSelector="R" yChannelSelector="G"/>
    </filter>
    <rect x="50" y="50" width="100" height="100" fill="blue" filter="url(#f)"/>
    <rect x="45" y="45" width="100" height="100" fill="none" stroke="red" stroke-dasharray="4"/>
</svg>
```

`semi-transparent-map.svg` is the same with `flood-color="#ffffff" flood-opacity="0.5"`. Read
unpremultiplied, it moves pixels exactly like an opaque white map: 10 px. Its outline is at 40.

Where the blue square lands (box of the blue pixels, 200 × 200 canvas):

| Case | Expected | Before | After |
| --- | --- | --- | --- |
| uniform map, scale 20 | x 45..=144, y 45..=144 | x 0..=48, y 0..=48 | x 45..=144, y 45..=144 |
| half-transparent map, scale 20 | x 40..=139, y 40..=139 | x 49..=148, y 49..=148 | x 40..=139, y 40..=139 |

The boxes were measured with resvg 0.48.1 before and after this patch; svgr 0.50.2 has the same
two lines. resvg 0.45.1, 0.46.0 and 0.48.1 render the fframes repro frame below pixel for pixel
the same as fframes' CPU renderer, so the bug is old and shared.

Other uniform maps fit `scale²` too: at scale 4 the square moves 4 px (16 × 0.253) instead of
1 px, and a `#404040` map at scale 20 moves it 100 px down and right, out of the canvas.

**In fframes**, the repro video (`fframes-repro.rs`) renders three filtered groups: a uniform
map, a `feTurbulence type="turbulence"` map, and a `fractalNoise` map with its alpha forced to
1. At frame 29 (scale 29), compared with odiff at threshold 0.15:

| Render | Pixels that differ from `--renderer skia-cpu` |
| --- | --- |
| `--renderer cpu`, svgr 0.50.2 | 57.27% |
| `--renderer cpu`, svgr 0.50.2 with this patch (`[patch.crates-io]`) | 0.50% (2,573 pixels) |

### The fix

The same change as linebender/resvg#1157:

```diff
--- a/crates/svgr/src/filter/displacement_map.rs
+++ b/crates/svgr/src/filter/displacement_map.rs
@@
-/// `sx` and `sy` indicate canvas scale.
+/// `scale_x` and `scale_y` are the `scale` attribute already converted
+/// into the canvas coordinate system, i.e. the maximum displacement in pixels.
@@
 pub fn apply(
     fe: &DisplacementMap,
-    sx: f32,
-    sy: f32,
+    scale_x: f32,
+    scale_y: f32,
     src: ImageRef,
     map: ImageRef,
     dest: ImageRefMut,
 ) {
@@
-        let ox = (x as f32 + dx * sx * fe.scale()).round() as i32;
-        let oy = (y as f32 + dy * sy * fe.scale()).round() as i32;
+        let ox = (x as f32 + dx * scale_x).round() as i32;
+        let oy = (y as f32 + dy * scale_y).round() as i32;
--- a/crates/svgr/src/filter/mod.rs
+++ b/crates/svgr/src/filter/mod.rs
@@ fn apply_displacement_map(
     let pixmap1 = input1.into_color_space(cs)?.take()?;
-    let pixmap2 = input2.into_color_space(cs)?.take()?;
+    let mut pixmap2 = input2.into_color_space(cs)?.take()?;
+
+    // The displacement map must be read using non-premultiplied color values.
+    demultiply_alpha(pixmap2.data_mut().as_rgba_mut());
 
     let mut pixmap = pixmap_pool
         .take_or_allocate(region.width(), region.height())
         .ok_or(Error::NoResults)?;
 
+    // `scale` is in user units, convert it into pixels.
     let (sx, sy) = match scale_coordinates(fe.scale(), fe.scale(), ts) {
```

`pixmap2` is a private copy at that point (`take()` clones a shared result), so demultiplying it
in place does not change the result that other primitives read.

### Tests

- New tests `filters/feDisplacementMap/uniform-map` and `semi-transparent-map`. Each draws the
  expected position as a dashed red outline, and the blue rect fills it.
- Reference PNGs made by the test harness (`MAKE_REF=1`) and compressed with
  `oxipng -o 6 -Z`, as `crates/svgr/tests/README.md` asks.
- The two lines were added to `integration/render.rs` by hand. Running `gen-tests.py` on
  Windows writes backslash paths and adds six SVGs that the checked-in file does not list.
- `cargo test -p svgr --release`: 1,707 passed, 0 failed.

resvg#1157 also adds `scale=0`, `turbulence-map` and `primitiveUnits=objectBoundingBox`; they
can follow in another PR. svgr's `CHANGELOG.md` has had no entries since 0.40.0, so this PR
adds none.

### Not in this PR

After the fix, 0.50% of the pixels of the repro frame still differ from Skia-CPU. In the odiff
image they are thin lines along the edges of displaced shapes, almost all in the
`feTurbulence` panel. svgr still samples the nearest whole pixel (the `TODO` about
anti-aliasing in `displacement_map.rs`), which is the likely cause. Smoother sampling is a
separate change.

---

## Optional issue for `dmtrKovalenko/fframes`

Title: `CPU renderer draws feDisplacementMap wrongly (fixed in svgr, needs a bump)`

> `--renderer cpu` and `--renderer skia-cpu` disagree on any frame that uses
> `feDisplacementMap`: 57% of the pixels differ on the attached repro, where Skia is correct.
> The cause is in svgr (scale applied twice, map read premultiplied), fixed in
> dmtrKovalenko/svgr#PR_NUMBER. Once that is released, bump `svgr` and `usvgr` in the root
> `Cargo.toml`. The repro video and the numbers are in the svgr PR.

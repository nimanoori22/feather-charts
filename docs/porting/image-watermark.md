# Image watermark port

Implemented and verified on 2026-10-17 from baseline `f4f5f63`.
Source identity: [manifest](image-watermark-source.json).
Plan/checklist: [plan](../../tasks/image-watermark-plan.md),
[checklist](../../tasks/image-watermark-todo.md).

## Source component and consumers

The complete built-in `src/plugins/image-watermark/{options,primitive,pane-view,
pane-renderer}.ts` files were read, together with the pane wrapper and draw order
in PaneWidget. The factory constructs a pane primitive; browser attachment loads
an Image and publishes natural dimensions on completion. Detach releases the
image/update callback. Options updates preserve omitted fields. The renderer
centers the image and fits it to padded pane dimensions and optional maxima.
Its view explicitly selects the bottom layer: after background, before grid and
ordinary series. There is no hit testing, autoscale contribution or animation.

Graph traces identify the advanced watermark tutorial and watermark type tests.
The type test changes alpha while retaining padding/limits; the graphics test uses
a transparent SVG above an Area background; the coverage fixture resizes to
300x300. These impose pane ownership, source-over alpha, aspect fitting, update
and resize requirements. The coverage subtree is excluded from the graph and was
read directly. The separate `packages/lwc-plugin-image-watermark` package is not
the behavioral reference for this built-in feature.

## Behavioral contract

Defaults: alpha 1, padding 0, no maxima. Center coordinates are rounded before
subtracting half the drawn image dimensions. Fitting preserves aspect ratio and
allows enlargement. Each active positive maximum constrains available width or
height; None and zero mean inactive. Text and image watermarks intentionally use
different primitive layers: normal and bottom respectively.

The image stays in logical pane coordinates during pan/zoom, with no price/time
reference. Existing owner keys preserve attachment identity through pane index
changes and preserved empty panes. Replacement keeps its PrimitiveId/order.
Detached/replaced images remain visible in retained old frames until those frames
are dropped. Alpha 0 hides output without changing the attachment.

Native validation rejects nonfinite/out-of-range alpha, nonfinite/negative padding
or maxima, zero dimensions, overflow and mismatched RGBA byte length before chart
mutation. Nonpositive pane/padded areas produce no image instead of the source's
incidental negative-scale geometry. None/zero maxima and valid source geometry
remain equivalent. These deliberate policies are independently tested.

## Rust design and ownership

Public `ImageWatermarkOptions`, `ImageWatermarkError` and `ImageWatermark` are
exported through `api`. `from_rgba(width,height,pixels,options)` consumes decoded
RGBA bytes and constructs one native shared handle. `with_options(options)`
returns a validated watermark sharing that same image, without copying pixels.
The host keeps its options/value and uses existing add_pane_primitive,
replace_primitive and remove_primitive operations. No second registry or handle.

```text
Host -> watermark/options
Chart -> existing Rc attachment -> watermark -> immutable shared native RGBA handle
Retained frame -> Rc frozen renderer -> shared image + fixed placement/opacity
```

One prepared viewport renderer is cached. Unchanged preparation and DPR-only
changes reuse it; resize recomputes constant-size placement without scanning data.
Options changes create a new renderer, sharing the image. No mutable pixel alias,
back-reference, ownership cycle, load callback, thread or engine task is introduced.
The native renderer draws the prepared image through the existing Frame API.

A Chart-owned image registry was rejected because primitive ownership already
provides identity, replacement and cleanup. Public encoded/path handles were not
accepted as validated inputs because they introduce IO/decode failures in drawing.
Browser URL strings, HTMLImageElement, onload identity and manual detach callbacks
are host integration details, replaced by owned decoded resources and RAII.

## Why the renderer dependency patches are necessary

Both unpatched Iced backends failed the overlap reproduction: an image covered a
green vector rectangle drawn after it. Frame and renderer layers batch by type,
so splitting chart geometry alone cannot establish bottom ordering. A chart-local
segment API would also spread feature-specific drawing interfaces through all
existing frame consumers while leaving software damage work at risk.

The selected alternative patches the exact registry versions, vendored under
`vendor/`: iced_tiny_skia 0.14.1 and iced_wgpu 0.14.0. The effective patch is
[159 lines across six files](../../vendor/image-ordering.patch), with original
and patched hashes in [provenance](../../vendor/image-ordering-sources.json).
MIT licenses and the unchanged source files needed for reproducible dependency
builds are retained. No edits to the external Iced checkout or registry cache.

TinySkia inserts canvas images into the ordered primitive list, clips each image
and draws it inside the existing cached group. Its shared raster pipeline now keeps
image placement in the floating-point transform: converting bounds/scale to an
integer pixmap offset shifted small scaled images. A final image/vector/image
regression reproduced that defect and now checks both ordering and placement. The chart retains one software
damage boundary. WGPU splits only image-containing geometry at raster/vector
boundaries, stores cached phases and submits them through ordered clipped layers.
Cached images can be removed and restored without stale phases. The compatibility
regression also caught early text flushing at an image boundary; canvas fill_text
now stays in the final overlay phase, including pasted subframes. Image-absent
geometry follows its existing path. Existing Canvas overlay-text semantics remain;
chart text is already prepared as glyph paths.

This preserves all current Chart/Frame drawing entry points and existing custom
primitive interfaces. No full-chart raster fallback, per-pixel vector conversion
or synchronous codec work. The Cargo feature is `image-without-codecs`; it brings
Iced's image-buffer/color/EXIF support but no default image codec suite. Cargo.lock
records the dependency changes. Remove the local patches when an upstream version
provides equivalent ordered canvas image rendering. General rotated/SVG canvas
behavior is not separately certified by this watermark milestone.

## Tests and evidence

All **636 all-target tests** pass. `cargo check --all-targets`, strict Clippy,
formatting and diff checks pass. Image, text, last-price-animation and conflation
actual-source oracles pass.

- Image oracle executes the pinned actual TS factory/defaults/view/renderer:
  **112 valid placement states**, load completion, default bottom layer, option
  preservation and detach behavior. Eight nonpositive drawable cases follow the
  documented native skip policy. Injected image dimensions isolate placement
  from browser decoding and platform filtering.
- Public tests prove visible output after an initial compiling visible-output
  RED, alpha hide/restore, image replacement/removal, invalid inputs preserving
  a working attachment, frozen old frames, multiple image/pane ownership, empty
  panes, index shifts, actual pane removal and normal/text occlusion of bottom
  images. Watermarks leave hits, layout/autoscale and scheduling unchanged.
- Resource tests prove identical native image IDs/pixel addresses across options
  changes, unchanged renderer Rc reuse for 100 preparations, DPR independence,
  resize transforms and retained-renderer release. Pulse-only ticks prepare no
  image watermark.
- Software damage tests pass with one redraw region at 941x1030, 420x320 and
  300x180 while navigation changes pixels. The original image-absent test passes.
- `image_ordering` runs on both native backends, direct/cached geometry, image/vector/image
  interleaving, scaled placement, cached removal/restoration, image clip bounds and analytic intrinsic-alpha times opacity
  blending. A black pixel of alpha 128 at opacity .5 over white produces ~191,
  within one quantization level; alpha 0 and 1 are checked too. Comparing image
  blending to native vector blending was rejected as an oracle because WGPU's
  vector color path uses different gamma handling. Raster source-over is tested
  against its own analytic expected values, not loosened to match vector output.
- **120 image captures** pass: two backends x four DPRs x three sizes x five scenes.
  Frozen frames draw twice with identical output. Scenes cover transparent logo,
  aspect/limits, mixed overlays/text, empty pane and multiple images.
- All **24 default-off images** match the retained pre-change images byte for byte.
  Source/binary/fixture/vendor hashes are frozen in the protocols. Representative
  software empty-pane, GPU capped-image and GPU mixed-occlusion images were viewed.

Raw images stay in `target/image-watermark/`. Compact protocols/hashes, native
regression logs, original failures and results are in
[evidence/image-watermark](evidence/image-watermark/validation.json).

The first capture run is retained in `target/image-watermark/after/`. It exposed
two fixture assumptions: a capped image can be wholly occluded by the dense series;
a short multi-pane scene can have less drawable height than the primary single-pane
frame reports. The limits fixture now hides its price series, leaving fitting
visible; the mixed scene still exercises actual overlap. Short-height assertions
use the drawn multi-pane region and account for padding. No renderer threshold or
paint-order assertion was weakened. The fixture-corrected run is `after-fixed/`. The final software placement fix
was verified with the unchanged fixture matrix in `after-placement-fixed/`; the
original placement failure is retained in the compact evidence. The accepted build
with the canvas text compatibility fix is captured under `final/`; compact
`capture-protocol.json` and `capture-results.json` refer to that run.

## Native acceptance and limits

The user initially reported “All checks passed; window closed” for both software and GPU
on 2026-10-17. Each run covered image opacity/hidden state, padding, size limits
and replacement controls; wheel zoom, diagonal pan, price-axis wheel/drag and box
moving/resizing at regular, narrow and earlier problem sizes; short-height shrink
and recovery. Text watermark, live feed and price pulse were running together.
The image stayed centered under chart content. The user repeated the software checks after the final fractional-placement fix
and again reported “All checks passed; window closed.”
Touchpad, touchscreen and physical
high-DPI checks remain untested; simulated DPR captures are not those checks.
Historical timing acceptance remains OPEN/deferred with unchanged thresholds.

Only decoded RGBA input is supported. The host can fetch/decode first, then attach
or replace on success; failed loads leave the current image intact. SVG, encoded
PNG/JPEG/data URLs, automatic downloads and animated images are outside scope.
Filtering/edge pixels can differ between native backends and browsers. No engine
component is missing for the implemented decoded-image behavior.

## Files and demo

- Public API: `src/api/image_watermark.rs`, `src/api/mod.rs`.
- Placement/rendering: `src/renderers/image_watermark_renderer.rs`, renderer exports.
- Native order: six patched files in the two vendored backends, vendor provenance,
  Cargo.toml/Cargo.lock; `examples/image_ordering.rs` is the native regression.
- Public/lifecycle tests: `tests/image_watermark.rs`; software damage test extended.
- Demo: `examples/public_chart.rs`, `examples/public_chart/image_watermark.rs`.
- Tools: `scripts/check_image_watermark_reference.cjs`, `scripts/capture_image_watermark.py`.
- Documentation: README, source manifest, this report/evidence and feature plan/checklist.

```sh
cargo run --example public_chart -- --line --image-watermark --text-watermark --last-price-animation-continuous --live-feed --primitives
# Append --wgpu for the GPU backend.
cargo run --example image_ordering -- --wgpu
```

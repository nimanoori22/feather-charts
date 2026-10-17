# Image watermark implementation plan

Date: 2026-10-17. Status: implemented; automated and software/GPU mouse acceptance pass.
Starting checkpoint: `f4f5f63` (text watermarks complete).
Execution checklist: [image-watermark-todo.md](image-watermark-todo.md).

## Outcome and boundary

Add a pane-owned image watermark with centered, aspect-preserving fitting,
opacity, padding, optional maximum dimensions, replacement and removal. Preserve
TradingView's bottom-layer order and pane anchoring through navigation and resize.
Reuse the primitive lifecycle and keep image loading outside the synchronous engine.

The first supported input is validated, decoded RGBA pixels. Applications may
fetch/decode their own images before attachment. The engine must not open files,
fetch URLs, decode images or start tasks while preparing/drawing. Encoded PNG/JPEG,
SVG/data-URL parsing, automatic downloads and animated images are outside this
milestone. The upstream tutorial/graphics tests use browser-loaded SVGs; their
placement/alpha/lifecycle intent must be preserved, but browser image-format and
loading API parity is not claimed. Use a small owned deterministic RGBA fixture
for native tests and the demo, without a new codec dependency.

Existing timing acceptance stays OPEN/deferred; device exceptions remain separate.
No old plan/checklist is overwritten. Feature-specific files follow this repository's
existing naming convention. No delegation or extra approval gates are needed.

## Source investigation and consumers

Read all four upstream components under `src/plugins/image-watermark/`:
`options.ts`, `primitive.ts`, `pane-view.ts`, `pane-renderer.ts`.
The public factory constructs a pane primitive and attaches through
`PanePrimitiveWrapper`. Its view explicitly returns `bottom`. Browser attachment
loads an Image; completion publishes natural dimensions and requests redraw.
Detach clears the image and update callback. Option updates preserve omitted
fields; returned renderers contain the current image/options snapshot.

Graph tracing of `createImageWatermark` identifies the advanced watermark tutorial
and watermark type tests. Direct source reads also cover the graphics fixture
(Area series, SVG, alpha .5, padding 20) and coverage resize fixture (300x300).
These require pane ownership independent of a series, transparency, preserved
aspect ratio, resize handling and options updates. No hit test, autoscale or timer.
Coverage metadata matches the four source files, type test and tutorial/graphics
fixture; the coverage-test subtree is excluded, so its source was read directly.
Freeze revision and working-tree hashes in implementation; the index is supporting
evidence, not the source identity. Inspect factory/wrapper/pane-widget ordering and
all imports again before modifying the shared drawing path.

The standalone `packages/lwc-plugin-image-watermark` implementation also exists.
The built-in `src/plugins/image-watermark` implementation is this milestone's
reference; do not accidentally substitute the standalone package's API/behavior.

## Required contract and explicit native validation

- Defaults: alpha 1, padding 0, no maximum width/height.
- Center is `Math.round(pane_width/2)` and `Math.round(pane_height/2)` in logical
  coordinates. Preserve rounding before positioning, including odd dimensions.
- Available dimensions subtract twice the padding, then apply each active maximum.
  Scale is min(available_width/image_width, available_height/image_height).
  Preserve aspect ratio; **upscaling is allowed** by the source.
- A maximum of zero is inactive in the source (truthiness); express None/Some(0)
  equivalently. Positive maxima cap drawing dimensions, not intrinsic pixel size.
- Native alpha must be finite and within [0,1]; padding and maxima must be finite
  and nonnegative. Reject invalid options before attachment/replacement. Do not
  emulate Canvas's incidental invalid-globalAlpha state retention.
- Require positive intrinsic pixel dimensions and checked `width * height * 4`
  matching the supplied buffer length. Reject arithmetic overflow/malformed input.
  Keep rendering conversions finite. Skip drawing when the pane/padded drawable
  area is nonpositive; record this deliberate handling of source negative-scale
  edge cases and compare ordinary valid geometry separately.
- Draw on the source bottom layer, clipped to its pane; preserve ordering relative
  to background, bottom primitives, grid, series and overlays. Trace PaneWidget's
  actual bottom passes before choosing insertion points; the label alone is not
  enough. Alpha 0 produces no visible contribution.
- Options/image replacement keeps PrimitiveId and attachment order. Removal and
  actual pane removal release ownership; retained old frames keep old pixels.
  Preserve empty-pane/index-shift behavior and multiple watermark attachment order.
- No hit target, axis labels, autoscale contribution, animation timer or history
  scan. Keep data navigation, editable primitives, text watermark and pulse intact.

## Renderer feasibility: the first implementation gate

Iced's local source and actual compiled registry sources were inspected. Canvas
`Frame::draw_image` is feature-gated. Both TinySkia and WGPU store images separately
from vector geometry and submit geometry first, then images in `draw_geometry`.
A plain draw_image call inside today's single chart Frame therefore cannot prove
bottom-layer ordering. Nested clips do not solve that category separation.
The current widget deliberately emits one cached geometry for software damage;
text-watermark verification already demonstrated a separate WGPU clip-flush trap.

Compare two real designs before choosing:

1. **Chart-local ordered rendering segments** using native image resources between
   vector phases, with the existing one-geometry path when images are absent.
   Prefer this if public render entry points, bottom attachment interleaving and
   software damage behavior can all be preserved with a small additive adapter.
   Images are shared resources, not per-pixel tessellation. Determine how renderer
   snapshots and custom primitive drawing contribute images without downcasting
   or a second ownership registry. Any new trait method must have a default.
2. **A renderer geometry-ordering fix** preserving images/vector insertion order
   within one geometry. This could retain the current chart damage boundary but
   spans Iced backends, increases maintenance and dependency scope. Do not select
   it merely because it appears cleaner; establish the smallest reproducible
   backend defect and compare scope against the chart-local design first.

Direct canvas drawing in one geometry is insufficient; converting every raster
pixel to a vector rectangle and full-chart CPU rasterization are rejected for
unbounded work or loss of normal backend rendering. Do not silently ship images
above the chart, weaken paint-order assertions, break existing draw callers or
accept software interaction regressions. If the preferred design cannot meet
these gates, write the concrete blocker and revised design before broad edits.

## Proposed public API and ownership

Provisional surface; confirm against the feasibility gate before implementation:

```rust
pub struct ImageWatermarkOptions {
    pub alpha: f32,
    pub padding: f32,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}
pub struct ImageWatermark { /* validated options, shared immutable image */ }
impl ImageWatermark {
    pub fn from_rgba(
        width: u32, height: u32, pixels: impl Into<Vec<u8>>,
        options: ImageWatermarkOptions,
    ) -> Result<Self, ImageWatermarkError>;
    pub fn with_options(&self, options: ImageWatermarkOptions)
        -> Result<Self, ImageWatermarkError>;
}
```

with_options returns a new value sharing the image resource, avoiding a pixel copy
for opacity/padding changes. Freeze error variants for invalid dimensions/buffer
length/options. Reuse native image handles internally; do not expose unchecked path
or encoded-byte handles as validated image input. Enable only the necessary Iced
image capability (evaluate `image-without-codecs` against the locked version).
Audit Cargo.lock changes; no broad codec/runtime dependencies just for RGBA.

```text
Host -> options + optional original watermark/image resource
Chart -> existing attachment Rc -> watermark -> shared immutable RGBA/native handle
Prepared frame -> frozen image payload -> same immutable resource + placement/alpha
```

No back-reference, Rc cycle, mutable pixel alias, request callback or load task in
Chart. Host image-load success maps to add/replace; load failure leaves the previous
attachment intact. Browser load callbacks/URL strings/manual destroy are incidental
host integration, not synchronous engine mechanisms. Old frames legitimately retain
image memory until dropped. Verify resource reuse/drop using observable work/weak
ownership hooks where needed, without a general image-manager framework.

## Delivery order and checkpoints

0. **Freeze contract and expose ordering failure.** Pin source hashes, settle valid
   input/error policy, make a minimal image/vector overlap fixture on both backends,
   and capture default-off baselines before changing rendering. Write a measurable
   software damage/work comparison using existing diagnostics at the laggy sizes.
1. **Prove ordered rendering.** Implement only the smallest adapter selected above;
   test background/image/grid/filled-series ordering, pane clips and repeated frames
   on both backends. Cover multiple bottom attachments interleaved with vector
   primitives. Preserve default-off output and public drawing call sites.
   **Checkpoint A:** ordering and damage evidence supports the chosen design.
2. **Ship one visible image.** Add validated RGBA construction, defaults, centered
   bottom attachment and frozen output through the public API. Record a compiling
   visible-output RED, then make it pass. Include transparent pixel blending.
3. **Complete fit and updates.** Add alpha/padding/maxima, source geometry oracle,
   with_options resource reuse and image replacement. Test tall/wide/square images,
   odd pane sizes, upscaling, zero maxima, fully transparent and oversized padding.
   **Checkpoint B:** visible core behavior matches the source and invalid values
   cannot replace a working image.
4. **Close lifecycle and work bounds.** Verify remove/hide-via-alpha/replace, old
   frames, multiple images/panes, preserved empty panes, pane removal/index shifts,
   text watermark coexistence, no hit/autoscale effects and pulse-only ticks.
   Prove options changes do not decode/copy pixels and unchanged layout does not
   recreate resources; resize computes placement without visiting history.
5. **Add the opt-in demo.** `--image-watermark`, a small generated RGBA fixture and
   controls for opacity, padding/size limits and image replacement. Use host-owned
   options/existing IDs. Demonstrate live feed, text, pulse and editable box.
   **Checkpoint C:** public lifecycle and demo work at regular/narrow/short sizes.
6. **Run finite native acceptance.** Freeze fixture/binary/source identity and
   capture 2 backends x 4 DPR values (1/1.25/1.5/2) x 3 sizes (800x500,300x300,300x90)
   x 5 scenes (transparent logo, aspect/limits, mixed occlusion, empty pane,
   multiple attachments) = 120 feature captures. Retain all failures; compare
   repeat draws of frozen frames and 24 default-off before/after images. Test
   alpha blending analytically with simple interior pixels; do not require
   backend edge-filtering pixels to match each other exactly.
7. **Review and report.** Run fmt, focused/full tests, all-target check, strict
   Clippy and affected text/animation/conflation source oracles. Review ownership,
   bounds, draw ordering, API compatibility and dependency changes. Record native
   mouse results on both backends at regular/narrow/problem sizes and short-height
   recovery. Publish source contract, alternatives, exact files, work/capture
   evidence and the decoded-RGBA scope/host-loading example.
   **Checkpoint D:** all gates pass; unavailable physical devices explicitly remain
   untested. Historical timing thresholds/status are unchanged.

Each slice lists dependencies, likely files and verification in the checklist.
Rendering adapters, resource lifetime and shared drawing paths must be sequential.
Do not make large cross-backend edits before the ordering proof.

## Completion record

Implemented the renderer-fix alternative (159 effective patch lines in the exact
Iced backend versions), preserving all existing draw entry points and one software
damage region. 636 tests, strict Clippy/check/fmt, all affected source oracles,
120 image captures and 24 unchanged default-off comparisons pass. Both physical
mouse runs passed at all three sizes and short-height recovery. See the
[port report](../docs/porting/image-watermark.md) for the retained failures, exact
scope and dependency-patch maintenance requirement. Other devices remain untested;
the historical timing exception is unchanged.

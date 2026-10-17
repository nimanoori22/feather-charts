# Text watermark implementation plan

Date: 2026-10-17. Status: implemented; automated and software/GPU mouse checks pass.
Rust starting checkpoint: `61bb52b`. Execution checklist:
[text-watermark-todo.md](text-watermark-todo.md).

## Outcome and scope

Add a built-in pane-owned text watermark using the existing drawing-primitive
API. Support multiple independently styled lines, all nine alignment combinations,
source width fitting, visibility changes, replacement and removal. Keep its
position relative to the pane viewport while chart data scrolls or zooms.

Ship one working centered line first, then full layout/style behavior, lifecycle
coverage, an opt-in demo and verification on TinySkia/WGPU. Image watermarks are
a later feature. No new animation, event subscription, async runtime, plugin
registry, editing handles, word wrapping or data-coordinate anchoring is needed.
The existing timing exception remains OPEN/deferred with unchanged thresholds.

## Investigated source and consumers

The complete upstream text-watermark files have been read:
`plugins/text-watermark/{options,primitive,pane-view,pane-renderer}.ts`.
The pane wrapper, primitive interfaces, public export, tutorial, watermark type
checks and initial/options-update/coverage fixtures were also inspected.
Graph traces identify the public factory/tutorial and the view/renderer chain;
direct searches supplement the graph for test and exported API consumers.
Coverage metadata matches the primary source files; freeze actual working-tree
revision/hashes before implementation rather than relying only on index freshness.

The factory attaches a pane primitive, not a series drawing. The wrapper exposes
option updates, pane lookup and detach; mutation requests a normal chart update.
The renderer consumes pane dimensions and font metrics only. Numeric/custom
horizontal scales in the upstream type fixtures confirm no time-domain dependency.
There is no price range, hit-test or timer contribution.

In Rust, `Chart::add_pane_primitive`, `replace_primitive`, `remove_primitive` and
stable pane-owner keys already implement attachment lifecycle. Prepared renderers
own immutable geometry. `AxisFontResolver` has bounded width/glyph caches, and
`PrimitiveRenderer` can draw prepared paths on both backends. `PrimitiveContext`
does not currently expose a font service; its `prepare` method is infallible.
Do not assume a shared font service can be injected without API consequences.

Exact Iced `graphics/src/geometry/text.rs` was inspected: Canvas text generates
paths from a shaped Paragraph and supports top/center/bottom anchors. Existing
axis glyph helpers use center anchoring. Font/line-box adaptation is therefore
an early verification task, not a reason to change axis behavior speculatively.

## Required behavioral contract

- Default visible=true, centered horizontally/vertically, no lines. Per-line
  defaults: text empty, black at alpha .5, size 48, native equivalent of the
  source default font; normal style. Default line height is 1.2 × font size.
- Each array element is one line. Lines have independent color, font family,
  weight/style, size and optional line height. No automatic wrapping.
- Each nonempty line independently uses `zoom = min(1, pane_width / text_width)`.
  Never enlarge a short line. Total block height sums scaled line heights.
- Top block offset is 0; center/bottom use max((height-total)/2,0) or
  max(height-total,0). Tall blocks clip; there is no vertical shrink-to-fit.
- Horizontal anchors are line_height/2 for left, pane_width/2 for center, and
  pane_width-1-line_height/2 for right. The source uses unscaled line height for
  this padding. Preserve that detail when a long line shrinks.
- Empty strings contribute no visible text or height in the source. Exercise
  blank versus whitespace-only strings and source line-height fallback for zero;
  resolve embedded line separators and invalid numeric values in the source audit.
- Text uses logical/media coordinates, top anchoring, alpha and plot clipping.
  The source view has no zOrder override: it uses **normal**, not bottom/top.
  Pane primitives precede series in the source group; grid/background remain
  governed by existing paint order. Watermarks can be occluded by filled series.
- Changing alignment/visibility preserves other options. Replacing lines uses
  the new array with per-line defaults, not an index-wise merge with old lines.
  Rust may express this through owned options and replacement rather than patches.
- Pane ownership is independent of series visibility/removal. A surviving empty
  pane can still display a watermark. Actual pane removal drops its attachment;
  numeric pane-index shifts must not move it to another owner.
- Watermarks do not affect autoscale, axis allocation, hit winners, pointer
  payloads, navigation, crosshair, markers, top primitives or animation scheduling.
  Retained frames stay frozen after updates/detach/removal.

## Design and alternatives

Preferred: `TextWatermark` implements the existing `DrawingPrimitive`. The host
owns/clones options and uses existing primitive operations; Chart owns the attached
value. A small pure layout helper consumes measured widths; a native preparation
adapter resolves fonts/shapes paths before drawing. Reuse existing bounded text
helpers; store a bounded cache with the watermark if that avoids public context
changes. Cache mutation is synchronous and renderer snapshots must not retain it.

Alternative: a Chart-owned watermark registry and dedicated Chart watermark
methods. This centralizes font access but duplicates primitive IDs/lifecycle,
preparation and removal. Reject unless the font audit establishes a concrete
advantage that the existing primitive path cannot supply cleanly.

Compare a local cache using house text helpers with a narrow internal shared-font
handoff in Task 0. Do not add a public service framework or widen every application
primitive's context just for this feature. Any shared glyph-helper adjustment must
preserve existing axis anchors, font cache keys and snapshots. Native typed fonts
are preferred over leaking dynamic strings to satisfy Iced's static font names.
Document the supported family/fallback mapping and font-loading expectations.

```text
Host -> owned TextWatermarkOptions
Chart -> existing Attachment -> Rc<dyn DrawingPrimitive> -> TextWatermark
                                                        -> options + bounded cache
Prepared frame -> existing prepared attachment -> Rc<frozen renderer>
                                               -> owned paths/colors
```

No back-reference to Chart, captured live options, Rc cycle or new cleanup handle.
Replacement preserves the existing PrimitiveId and attachment order; removal
uses existing owner checks. Old frames own their old renderer output.

### Proposed API (confirm in Task 0)

```rust
pub struct TextWatermarkOptions {
    pub visible: bool,
    pub horizontal_alignment: iced::alignment::Horizontal,
    pub vertical_alignment: iced::alignment::Vertical,
    pub lines: Vec<TextWatermarkLine>,
}
pub struct TextWatermarkLine {
    pub text: String,
    pub color: String,
    pub font: iced::Font, // family, weight and style; native font selection
    pub font_size: f32,
    pub line_height: Option<f32>,
}
pub struct TextWatermark { /* validated options + private cache */ }
impl TextWatermark {
    pub fn new(options: TextWatermarkOptions) -> Result<Self, TextWatermarkError>;
}
```

Use `Default` for source defaults. Validate colors/numeric geometry before an
attachment/replacement can alter the chart. Record the policy for degenerate
sizes/line heights; invalid replacement must preserve the previous attachment.

```rust
let id = chart.add_pane_primitive(pane, TextWatermark::new(options.clone())?)?;
options.visible = false;
chart.replace_primitive(id, TextWatermark::new(options.clone())?)?;
chart.remove_primitive(id)?;
```

This maps source create/applyOptions/detach onto existing Rust ownership. The host
modifies only intended fields on its options value. A separate partial-update API,
watermark-specific handle, automatic symbol/timeframe binding and getPane wrapper
are unnecessary for the requested behavior.

## Delivery order

0. Freeze source/consumer contract, font strategy and API ownership.
1. Build an actual-source layout/options oracle and define public assertions;
   record the public behavioral RED when the API first compiles in step 2.
2. Deliver one centered line through public pane attachment and frozen drawing.
3. Complete multiline fitting, alignment, styles and validation.
4. Close update/removal/pane lifecycle and mixed-rendering regressions.
5. Prove bounded shaping work; add the public demo controls.
6. Verify native captures/default-off parity and physical mouse coexistence.
7. Run final checks/review and publish the port report.

The checklist splits work into small slices and lists likely files/dependencies.
Refine file locations before implementation; preserve source correspondence where
practical without introducing a new plugin framework. Keep shared font/primitive
changes sequential. Checkpoints after 0–1, 2–3, 4–5 and 6–7 are verification gates,
not additional permission prompts. No delegation is needed.

## Verification and completion

Use the actual TS options/view/renderer with an injected recording canvas and
fixed measureText widths. Compare layout transforms, alignment, fillText ordering,
visibility/defaults and update behavior to Rust. Freeze fixture counts and hashes.
Do not claim browser/native pixel equality from synthetic font metrics.

Test native metrics and glyph output together: proportional/monospace, bold/italic,
Unicode/fallback, empty/whitespace, long lines and custom line height. Shape at the
base font size and transform prepared paths when fitting where feasible. Preserve
line-box/top semantics; never center each glyph's ink box as a substitute.

Counters must show no repeated shaping on unchanged preparation, pan/live updates
with unchanged viewport/style, or last-price-animation-only ticks. Resize/fitting
can recompute transforms without scanning series history. Caches must be bounded,
keys must include all shaping-affecting fields, and replacement/removal must release
unused state. No new wall-time benchmark matrix or historical timing reruns.

Before changing output, capture default-off references. Run a finite native
capture matrix: 2 backends × DPR 1/1.25/1.5/2 × 3 sizes (800×500, 300×300,
300×90) × 5 scenes (multiline, edge alignments, long-line fit, mixed overlaps,
preserved empty pane) = 120 watermark captures. Also compare 24 default-off
before/after images (backends × DPR × sizes). Freeze commands/scenes/font identity
before capture, retain failures, check frozen-frame repeatability, and inspect
representative images. Layout oracle/tests cover all nine alignment combinations.

Run fmt, focused/full tests, all-target check and strict Clippy; rerun affected
primitive/axis/animation source oracles. Test mouse zoom, diagonal pan, price-axis
wheel/drag and box editing with watermark + live feed on both backends at regular
and narrow sizes. Record unavailable touch/high-DPI hardware separately.

Publish `docs/porting/text-watermark.md`, a pinned source manifest, README demo
command and completed checklist. Report source consumers, required/incidental
behavior, API/ownership, alternatives, exact files, tests/captures/work evidence
and any font/platform limits. Completion requires implementation, automated parity
and recorded native acceptance; it does not close old timing/device exceptions.

## Completion record

See [port report](../docs/porting/text-watermark.md) and
[evidence](../docs/porting/evidence/text-watermark/validation.json).
626 tests, all-target check, strict Clippy, formatting, three source oracles,
120 watermark captures and 24 unchanged default-off comparisons pass.
Both native mouse checks passed at regular/narrow sizes. Physical touchpad,
touchscreen and high-DPI checks remain untested; old timing acceptance is unchanged.

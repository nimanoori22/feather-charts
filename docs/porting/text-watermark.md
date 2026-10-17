# Text watermark

Implemented and verified on 2026-10-17; baseline `61bb52b`. Source working-tree identity is
frozen in [text-watermark-source.json](text-watermark-source.json).

## Contract and consumers

The four complete upstream text-watermark components, pane wrapper and primitive
interfaces were inspected. Public factory/tutorial/type tests require pane-owned
creation, independently styled lines, option updates and detach; generic horizontal
scale type tests impose no time dependency. Initial/options-update/coverage fixtures
exercise family/style changes, alignment and detach. PaneWidget draws pane
primitives before series in the normal group. The watermark has no zOrder override,
hit test, autoscale or timer. The renderer independently shrinks lines to width,
sums scaled line heights, and clamps center/bottom block offsets to zero. It uses
unscaled line-height padding at the horizontal edges. Empty strings add no height.

The source view uses `lineHeight || fontSize * 1.2`: None and zero therefore map
to the default. Native options reject nonfinite/negative heights and nonpositive or
nonfinite font sizes before attachment; no real consumer requires invalid CSS
geometry. Native fonts use Iced Font (family, weight, style) rather than dynamic
CSS font strings. Default is the native sans-serif fallback, not guaranteed to be
the same installed font as a browser's CSS fallback list.

Canvas replaces ASCII whitespace with spaces during text preparation, including
embedded tabs/newlines; normalize those characters once so Iced cannot introduce
extra lines. See [HTML text preparation](https://html.spec.whatwg.org/multipage/canvas.html#text-preparation-algorithm).
Other Unicode line-breaking details use the native paragraph implementation;
exact browser equivalence for those controls is not asserted.

## Design and ownership

Preferred: TextWatermark implements DrawingPrimitive and uses add_pane_primitive,
replace_primitive and remove_primitive with existing owner checks. The application
keeps its options value. A Chart-owned registry was rejected because it duplicates
ownership/IDs/removal. No new public context fields or font-service framework.

TextWatermark owns validated lines, a local instance of the existing bounded
AxisFontResolver cache, and at most one prepared viewport renderer. Shared Chart
font injection would widen the primitive preparation interface; local house-cache
reuse avoids that API change. A private glyph-helper variant for top anchoring
includes the vertical anchor in cache keys; existing axis callers retain Center.
A viewport cache can return a frozen renderer without any reshaping on unchanged
preparation. It cannot retain a Chart/model reference.

Chart -> existing Rc attachment -> TextWatermark -> options, bounded caches
Prepared frame -> Rc frozen renderer -> owned paths/colors

There are no back-references/cycles or timer handles. Removing a primitive releases
its caches; retained frames keep only their old rendering payload. Pane-owner keys
keep identity through index shifts. Browser callback identity, mutable renderer
objects, getPane wrappers and CSS string parsing are incidental implementation
choices. Visible layout, colors, ownership and lifecycle are required.

## API

Public TextWatermarkOptions owns visibility, native horizontal/vertical alignment
and Vec<TextWatermarkLine>. Each line owns text/color, Font, font_size and optional
line_height. Defaults match source geometry/color. TextWatermark::new(options)
returns Result and validates before mutation. Modifying a host-owned options clone
and replacing the primitive preserves untouched fields and stable PrimitiveId.
No separate patch registry, unsafe code, thread or asynchronous engine is needed.

## Validation

Automated/source/native rendering checks pass; mouse responses and the final
command report are recorded below. Historical timing acceptance stays OPEN/deferred.

## Implementation

Public types are exported from `api`: `TextWatermark`, `TextWatermarkOptions`,
`TextWatermarkLine` and `TextWatermarkError`. Colors and numeric style values are
validated by construction before Chart mutation. Size must be positive/finite and
its native line box representable; line height must be finite/nonnegative. Zero
line height uses source fallback. No options registry or context extension was
introduced. Host-owned options implement deliberate full replacement through the
existing primitive API, preserving IDs and attachment order.

Logical layout is a small pure function. Native preparation measures base-size
text, shapes owned glyph paths, then applies source alignment/scale transforms.
`AxisFontResolver` keeps its existing 256-entry width and glyph bounds. The private
cache key now includes vertical anchoring; old axis calls retain Center and the
watermark uses Top. One prepared viewport renderer is retained by the watermark;
unchanged preparation reuses it by Rc. Resize recomputes transforms using cached
base text. Neither glyph preparation nor drawing reads canonical series history.

The frozen renderer contains paths/colors only. It inherits the plot's existing
clip and does not sample clocks, shape text, inspect a model or retain mutable
options while drawing. Retained old frames own their old paths after replacement,
hide, detach, pane removal and index changes. Multiple watermarks use ordinary
primitive attachment ordering and existing pane-owner keys.

## Root fixes found during verification

Two new checks failed before correction; their evidence is retained.

1. The native Arabic/fallback test overflowed `cosmic-text::SubpixelBin::new` while
   generating glyph paths. The actual compiled Iced 0.14 text path can leave a
   single RTL run positioned against an infinite width: its alignment branch can
   overwrite the RTL relayout flag. The shared glyph adapter now passes the
   measured finite text width to Canvas path generation. The Arabic/CJK/fallback
   regression passes with overflow checks enabled. No dependency profile, panic
   suppression or overflow-check setting was changed.
2. The first WGPU capture had no visible watermark. The extra nested clipping
   draft pasted glyph meshes before the parent's unflushed background meshes.
   Iced WGPU flush ordering then covered the text. Removing that redundant draft
   keeps glyphs in the plot's existing clip/buffer and preserves normal ordering.
   The capture fixture asserts visible output on both backends; all cases now
   pass. The existing single chart geometry/damage boundary remains intact.

The original run is under `target/text-watermark/after/`; the corrected run is
under `target/text-watermark/after-fixed/`. This was a root-fix rerun with unchanged
scenes/assertions, not a timing retry or threshold adjustment.

## Automated evidence

- Actual-source oracle: **270** combinations of alignment, pane dimensions and
  line scenarios match the pinned TS defaults/view/renderer. The real factory,
  wrapper, options replacement, visibility, detach and normal-layer defaults are
  exercised as well. Metrics are injected to compare layout independently of
  platform font rasterization.
- A public pixel RED demonstrated an attached watermark drawing nothing before
  rendering was implemented; the same visible-output test now passes.
- Focused tests cover default visible behavior, full replacement/hide/removal,
  rejected styles/invalid pane IDs, frozen old frames, preserved empty panes,
  owner identity through index shifts, actual pane removal, normal paint order,
  hit/autoscale/layout independence and animation-only Tick coexistence.
- Font tests cover top/center anchor separation against native Canvas paths,
  ASCII whitespace normalization, native Arabic/CJK/fallback shaping, and bounded
  cache reuse. One hundred unchanged preparations reuse the same renderer; a
  resize and DPR change do not reshape unchanged base text. Pulse-only Ticks make
  zero watermark prepare calls. The existing axis/cache tests pass.
- Both backends pass **120 watermark captures**: five scenes × three sizes ×
  four DPR values × two backends. Every frozen frame is drawn twice and compared.
  The scenes include multiline, edge alignment, long-line fit, mixed filled/
  primitive/marker content and an empty preserved pane.
- All **24 default-off images** are byte-identical to the pre-feature captures.
  The protocol freezes binary/source/fixture hashes and fontconfig selections;
  actual fallback is provided by Iced/cosmic-text. Representative software and GPU
  images, including the preserved empty-pane watermark, were inspected visually.

`scripts/capture_text_watermark.py` runs the fixed capture matrix and compares
baseline pixels. The first baseline was taken with a capture-fixture-only change
on `61bb52b`, before production text/glyph code changed. Raw RGBA files stay under
`target/text-watermark/`. Compact protocols, hashes and status are recorded under
`docs/porting/evidence/text-watermark/`.

## Native acceptance and limits

The user reported “All checks passed; window closed” on both software and GPU
on 2026-10-17. Both runs covered watermark visibility/alignment/text/style controls
and wheel zoom, diagonal dragging, price-axis wheel/drag and box moving/resizing
at regular and narrow sizes, with live feed and price pulse running. The watermark
stayed anchored to its pane. Physical touchpad, touchscreen and high-DPI device
checks remain untested and separate from simulated DPR captures.

Native `iced::Font` selects family, weight/style and fallback through Iced's font
system. Load fonts in the host before preparation; replace the watermark after
changing font availability. Exact browser/native glyph pixels and font fallback
identity are not claimed. Other Unicode line-breaking details follow the native
paragraph implementation; ordinary embedded ASCII tabs/newlines are normalized
per Canvas preparation. The watermark does not implement automatic word wrapping.

Image watermarks, dynamic symbol/timeframe binding, watermark editing handles and
a general JavaScript plugin wrapper remain outside scope. No engine component is
missing for the implemented text watermark. The historical timing exception stays
OPEN/deferred, with its original acceptance thresholds unchanged.

## Files and verification

Core/public API: `src/api/text_watermark.rs`, `src/api/mod.rs`.
Layout: `src/renderers/text_watermark_renderer.rs`, its tests/module export.
Native glyph preparation: `src/ui/text_watermark.rs`, `src/ui/mod.rs`,
`src/ui/price_axis.rs` (private anchor-aware cache and finite-width glyph fix).
Public behavior tests: `tests/text_watermark.rs`.
Demo/captures: `examples/public_chart.rs`, `examples/public_chart/watermark.rs`.
Tools: source oracle and native capture runner under `scripts/`.
Documentation: README, this source/ownership/report/evidence and feature checklist.

Final verification passed: `cargo fmt --check`, **626 tests** from
`cargo test --all-targets`, `cargo check --all-targets`, and
`cargo clippy --all-targets -- -D warnings`. The watermark, last-price-animation
and conflation source oracles all pass. No old timing screens were rerun.
See [compact evidence](evidence/text-watermark/validation.json) for the recorded
results, source/capture protocols and retained failure diagnostics.

Run the demo with `cargo run --example public_chart -- --line --text-watermark
--last-price-animation-continuous --live-feed --primitives`; append `--wgpu` for
the GPU backend. Image watermarks are the separate follow-up dependency/feature;
text watermarks need no additional engine component.

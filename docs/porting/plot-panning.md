# Free plot dragging

## Source and contract

Investigated Lightweight Charts `gui/pane-widget.ts` pressed-mouse movement and
end-scroll consumers, `model/chart-model.ts` price/time scroll entry points,
`model/pane.ts`, and `PriceScale.startScroll/scrollTo/endScroll`.
PaneWidget owns one drag, starts time and default-price-scale scrolling, passes
pane-local X/Y independently, and ends both on release. Price scrolling shifts
the original range by pixel displacement × range length / usable height;
inverted scales reverse the sign. Range length is preserved. Other panes share
time navigation but keep independent price ranges. Crosshair/views refresh after
movement. Graph coverage is best-effort and direct reads establish behavior.

The original engine rejects price scrolling with autoscale on. To fulfill the
requested free dragging immediately in our demos, meaningful vertical movement
explicitly enters manual price mode. Horizontal-only dragging preserves
autoscale. This is a deliberate application behavior extension; existing
low-level Pane/PriceScale source algorithms remain unchanged. Axis double-click
restores autoscale. Existing 5px activation, wheel behavior, gesture ownership and
stationary-event suppression remain in effect; kinetic/touch gestures are deferred.

## Alternatives and ownership/API

1. Add independent horizontal/vertical input handlers and gestures.
2. Emit a single two-coordinate pan action from ChartInput and route both
   dimensions synchronously through the current application/model path.

Choose 2: one event, one gesture owner, no duplicate state machines or allocations
per movement. Retain the existing horizontal `Navigation::Scroll` variant for
callers; add `Pan { start: Option<iced::Point>, position: iced::Point }` for native
plot input. Multi-pane routing retains the original pane even when the pointer
crosses another pane or an axis. First activation uses the same current-point
anchor as the established horizontal path; subsequent movement pans both axes.

Application owns an optional plot-pan session with pane, axis side, anchor Y and
whether price scrolling started. Canvas owns the existing input state; output
owns only coordinates. Model borrows the selected pane/scale synchronously.
No callbacks, mutable JS identity, manual destroy, async state or ownership cycles.
Release, focus/layout/membership changes, wheel, axis/divider gestures and data
lifecycle changes end both model gestures; drawing stays read-only.

Public model API:

```rust
// start_scroll_price(pane: usize, side: PriceAxisSide, y: f64)
// scroll_price_to(pane: usize, side: PriceAxisSide, y: f64)
// end_scroll_price(pane: usize, side: PriceAxisSide)
// Each returns Result<(), ChartModelError>.
```

The start method explicitly enters manual price mode for supported nonempty
scales. All methods validate pane/finite input and reuse existing pane effects,
source price-scroll geometry and crosshair refresh. Current demos select the
pane's default visible price axis, matching source consumers; multi-series
composition is now supplied by the [public chart API](public-chart-api.md).

## Implementation and verification

Implemented in `src/ui/chart_input.rs`, `src/model/chart_model.rs`,
`src/model/price_scale.rs`, `examples/support/chart_demo.rs`, and their existing
input/model/demo tests. Source fixtures extend `examples/interaction_reference.rs`
and `scripts/check_interaction_reference.cjs`. No dependencies were added.

The native diagonal-drag regression first failed because the price range did not
move, then passed. Connected tests cover line, candle, histogram and price/volume
panes, diagonal and pure vertical movement, horizontal-only autoscale, pane
isolation, crossing between panes, release/focus cancellation and removal/reload.
A geometry test exposed a pre-existing projection mismatch: projection used
`internal_height`, whereas source scrolling uses `internal_height - 1`. Projection
and inverse conversion now use the source's exact pixel span and inversion-aware
bottom margin. Normal/inverted and logarithmic model tests verify one-to-one
pointer displacement, unchanged range length, reversal, end/reset and invalid or
unsupported input.

Growing axis labels must not cancel a pan. ChartInput retains the gesture's
original coordinate origin through width changes; hover/crosshair coordinates
use the current layout. Chart-wide bounds changes still cancel input in the
Canvas caller, and pane height/origin changes cancel through ChartInput. The
existing multi-pane viewport guard retains global bounds/pane-count checks.
Standalone ChartInput consumers must reset their input state on chart-wide
viewport changes, as the shared Canvas does.

Validation commands: `cargo fmt --check`, `cargo test --all-targets --quiet`,
`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --examples`. Tests pass: 328 library tests plus 16 tests in each of
four demo example targets (392 executions). The source interaction oracle now
contains 401 cases: the previous 131 plus 270 actual TypeScript PriceScale
scroll/projection cases covering fractional heights, negative/large ranges,
up/down displacement and inversion. Formatter globals are stubbed because
these source methods do not use them; PriceScale and PriceRangeImpl algorithms
are executed from the original source.

All nine source comparison scripts pass (19,233 cases in total). Fresh
price/volume captures pass on TinySkia and WGPU: 28 each at ratios 1, 1.25, 1.5
and 2, with finite geometry, clipping/seam and repeatability assertions. Outputs
are in `/tmp/feather-free-pan-volume-{tiny,wgpu}`; inspected the default 1.25x GPU
capture. These are automated rendering checks, not real-device input acceptance.
Original PriceRangeImpl metadata was stale in the graph; its complete current
source was read directly and the oracle transpiles that source at run time.
Native ChartInput now emits the additional Pan variant; downstream exhaustive
navigation matches must handle it. The existing Scroll variant remains usable.
On 2026-10-12 the user reported that physical mouse smoothness checks felt fine.
Touchpad acceptance has not been separately reported. Time navigation is shared
and only the origin pane's default price scale moves; double-click that axis to
restore autoscale. Touch and kinetic scrolling remain subsequent work.

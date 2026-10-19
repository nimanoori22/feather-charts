# Runtime default price-axis selection

Baseline: `fb096eb`. Source: local Lightweight Charts 5.2.1,
`8a7c94ff8ddffde68955db61ef8b58098f239de0`.
Plan: [native capabilities](../../tasks/native-capabilities-plan.md).
Only runtime selection is implemented in this change. End-to-end Display-P3 and
broader Intl expansion are **not implemented**; their planned tasks stay unchecked.
Existing sRGB drawing and H's verified locale profiles remain implemented.

## Source investigation and real consumers

Read complete `src/model/chart-model.ts`, `src/model/pane.ts` and
`src/model/default-price-scale.ts`, plus chart API/widget forwarding, defaults,
source coverage/graphics tests, magnet/grid views and relevant crosshair/input
consumers. Graph discovery and both-direction traces identify shared model options
as the source of the preference. Source coverage is best effort with matching
metadata; the excluded coverage-test subtree was read directly.

`ChartApi.applyOptions` forwards to the widget, then `ChartModel.applyOptions`
merges model options and invalidates the chart. Each pane dynamically reads that
shared preference. Selection prefers a populated visible primary side, then a
populated visible secondary side, then the first attached source; without a source
it uses a visible side or the preferred empty scale. The visible-side helper does
not require a populated series. Grid marks, crosshair coordinate conversion and
magnet snapping use the default price scale. Overlay labels use the default visible
side. Plot scrolling starts from the default scale; explicit axis gestures retain
their own named target. New panes see current options. Existing series are not moved.

The source also uses the preference when it implicitly selects a destination for
new series. Native creation already requires an explicit `PriceScalePosition`;
that established typed API continues to win. This change adds no implicit-series
creation overload or JS-shaped retained wrapper.

The source enum/default is right. It is a preference, not a force-left operation:
hidden/empty scales still trigger the source fallback chain. It does not show a
hidden axis, move a series, reset a manual range or select another pane.

## Rust design, alternatives and ownership

Chosen: extend `ChartModelOptionsPatch` with an optional existing enum, validated
through the ordinary atomic patch path. Readback uses the existing owned
`RuntimeModelOptions.pane`. Preview merges the preference, compares before/after,
and classifies an actual preference change as a viewport interaction-target change.
The owner cancels gestures through its existing policy before commit. Invalid and
equal patches preserve gestures and queued events. Existing pane-option propagation,
full invalidation and crosshair refresh perform the remaining work.

Alternative: a dedicated per-pane setter. That duplicates validation/invalidation
and would introduce per-pane runtime state unlike source's chart-wide option. It
has no demonstrated consumer need here, so the existing patch path is used.

Chart owns model/defaults and panes; each pane keeps a copied preference enum.
Future panes copy current defaults. Prepared frames/screenshots own resolved
output. There are no new references, callbacks, allocations on draw, Rc cycles,
Drop requirements, async tasks or threads.

Source mutable shared option identity and DeepPartial string merging are
incidental. Rust uses a typed optional field and detached readback. An active
viewport gesture is cancelled when its possible primary scale changes; this native
ownership policy prevents continuation on an obsolete target. JS does not expose
an equivalent owned gesture token. Explicit series positions and stricter atomic
validation remain documented native adaptations.

## Public API

```rust
use feather_charts::api::{
    Chart, ChartOptionsPatch, ChartModelOptionsPatch, DefaultVisiblePriceScale,
};
let mut chart = Chart::default();
chart.apply_chart_options(ChartOptionsPatch {
    model: ChartModelOptionsPatch {
        default_visible_price_scale: Some(DefaultVisiblePriceScale::Left),
        ..Default::default()
    },
    ..Default::default()
}).unwrap();
assert_eq!(
    chart.runtime_options().model.pane.default_visible_price_scale,
    DefaultVisiblePriceScale::Left,
);
```

`None` retains current configuration. The existing yield-curve forwarding method
and generic calendar/strike/session chart implementation share this path. Existing
callers using `..Default::default()` need no migration; exhaustive
`ChartModelOptionsPatch` literals must add `default_visible_price_scale: None`.
Ordinary patch defaults behave as before. The public enum admits Left/Right and cannot accidentally name an overlay.

## Tests and desktop demonstration

`tests/runtime_primary_axis.rs` covers existing/future/reordered panes, original
readback lifetime, preserved series IDs/data, visibility/empty fallbacks, atomic
invalid/equal patches and pending data events, grid marks, overlay label side,
retained frames, pan cancellation, custom series/manual ranges and pointer price
conversion. The source oracle compares ten ordered states from actual Chromium
source execution against the public Rust owner. Preserve the missing-API and
semantic red logs in `target/primary-axis/`.

The existing configuration demo adds Compare axes, Primary left and Primary right.
Compare axes puts blue/custom on different visible sides with distinct margins;
primary controls then change the grid reference without changing assignments.
The footer reports the configured preference. All four domain hosts are exercised
by the example test, including their existing forwarding and Reset behavior.

```sh
cargo run --offline --example pane_management -- --price-scale-configuration --screenshot-export --live-feed
```

Add `--options`, `--yield-curve` or `--custom-domain` for other domains. Select
`ICED_BACKEND=tiny-skia` or `ICED_BACKEND=wgpu` for a normal desktop acceptance run.

## Validation and delivery

Implementation and acceptance are complete. Formatting, all-target build/check,
strict Clippy, 892 tests across 91 suites and two compile-fail doctests pass. All
47 source comparison suites pass. Both renderer paths pass 576 new screen captures
and 576 retained exports; 2208 affected compatibility images remain unchanged.
Software and GPU mouse acceptance passed in all four modes, including previews
and exports. GPU confirmation was received after the user accidentally closed Codex.
See [the evidence manifest](runtime-primary-axis-evidence.json) for exact hashes
and [the measured score](public-parity-score.md) for the bounded API inventory.
The focused matrix freezes four domains, regular
800×500/narrow 300×300/short 300×90, DPRs 1/1.25/1.5/2 and both asserted backends.
Scenes: right, left, hidden-side fallback, moved, recovered and rejected. Export
snapshots survive preference replacement, Reset and owner drop. A rejected mixed
patch must retain identical pixels; left/right must visibly differ at regular size.
Affected unchanged scale-assignment/axis/helper protocols retain their old counts
and accepted image hashes. Z0 history remains intact; it is not reclassified as
newly executed evidence.

Files: `src/model/chart_configuration.rs`,
`src/model/chart_model/configuration.rs`, `src/api/configuration.rs`,
`tests/runtime_primary_axis.rs`, configuration/primary-axis capture modules in
`examples/pane_management`, the main example, source oracle/capture tools, public
inventory/score tools and documentation. Final evidence lists exact current hashes.
Remaining dependencies: none for the requested runtime preference. Display-P3,
broader Intl, earlier timing/drawing-order and unavailable-device checks remain open.

# Price-based horizontal charts: implementation plan

Status: proposed; implementation has not started. Baseline: `53ed228`.
Checklist: [options-chart-todo.md](options-chart-todo.md).

## Outcome and scope

Deliver the Rust equivalent of TradingView `createOptionsChart`: horizontal items
represent prices or strikes, with decimal labels and price-based tick priorities.
A chart can plot option volume against strike without treating strikes as dates.
Reuse the existing behavior-generic Chart, data layer, preparation, widget,
interaction and extension paths delivered for yield curves.

Include all six existing built-in families (Line, Area, Baseline, Histogram, Bar,
Candlestick), multiple series/panes, replacement/latest/historical updates,
whitespace, typed pointer events, managed up/down markers, existing drawing
primitives, watermarks and compatible last-price animation. Exercise ordinary
chart options and vertical price formatting independently of horizontal precision.

Exclude option valuation, Greeks, feeds, trading/execution, a continuous numeric
projection, numeric custom-series renderers and a new chart-plugin framework.
Do not copy the yield scaffold or its Line/Area restriction. No new runtime,
threads, async mechanisms or dependencies are expected.

## Source investigation and consumer requirements

Primary files in the sibling lightweight-charts checkout:

- src/api/create-options-chart.ts: constructs ordinary ChartApi through createChartEx.
- src/model/horz-scale-behavior-price/{horz-scale-behaviour-price.ts,options.ts,types.ts}:
  numeric identity conversion, fixed decimals and numeric tick priorities.
- src/api/create-chart.ts and chart-api.ts: construction, default merging and the
  behavior/data/widget ownership boundary.
- src/api/options/chart-options-defaults.ts: ordinary chart defaults; localization
  has no precision default, so missing precision reaches JS toFixed as undefined.
- src/api/ichart-api.ts: generic addSeries accepts the existing series families.
- tests/e2e/graphics/test-cases/options-chart.js: actual factory, precision 1,
  1,000 quarter-unit Line items and fitContent.
- website/tutorials/how_to/horizontal-price-scale.js: custom behavior constructor,
  precision 2, 5,000 quarter-unit Line items; supporting behavior example rather
  than proof that the built-in factory has different defaults.
- src/model/time-scale.ts: coordinates derive from logical indices/bar spacing.

Factory consumers require ordinary chart defaults and common series operations.
Data/model consumers require stable ordered keys, multi-series union positions and
incremental weights. Axis/crosshair consumers require the same formatter before
and after options changes, including cache invalidation. Public events/extensions
must retain numeric items rather than calendar conversions. No source consumer
here requires the yield wrapper's private support series or deferred settlement.

Graph search and outbound tracing confirmed the factory -> createChartEx -> ChartApi
path. Exact-path coverage was checked for these code files and the Rust integration
points; metadata matched except upstream time-scale.ts, whose relevant coordinate
implementation was read directly. Graph coverage is best effort, not exhaustive
proof; the contract checkpoint must trace remaining formatter/data consumers and
read any further coverage gaps before production edits.

## Behavioral contract to freeze before coding

| Requirement | Evidence and checks |
| --- | --- |
| Numeric identity | Ordered finite prices retain their values in keys, events, markers and round trips. Test negative/fractional values and signed zero; reject NaN/infinity at the native boundary. |
| Index-based positions | Irregular strikes such as 90,100,150 occupy successive logical positions. Compare multi-series unions and explicit whitespace against source. Do not interpolate absent strikes or generate support points. |
| Fixed decimal labels | Both item/crosshair and tick labels use toFixed(precision). Omitted precision yields zero decimal places. Verify zero through 100 places, rounding ties, negative near-zero labels and the JS exponential fallback at magnitude >= 1e21. Rust formatting is not assumed equivalent. |
| Precision changes | Update data/model behavior coherently and invalidate labels/measurements. Horizontal precision must not overwrite vertical series price formatting. |
| Priority weights | Multiples of 100,50,25,10,5,1,0.5 receive 8,7,6,5,4,3,1; other values receive 0, in that order. Preserve exact source floating-point predicates, including negative values and zero. |
| Incremental work | Source fills weights starting at startIndex. The native append hook can calculate this item-local weight without rescanning history; replacement and append outputs must agree. |
| Ordinary chart defaults | Preserve ordinary axes/series defaults and explicit overrides. Do not inherit yield percent labels, hidden price lines, left-axis defaults or month/year labels. |
| Shared mutations and lifecycle | Preserve ordering, duplicate/historical update policy, whitespace, clear/remove, empty recovery, marker deadlines and retained-frame independence using the existing core. |

Execute actual TS files in the source oracle; do not duplicate the algorithm in a
JS expected-value implementation. Record source revision plus working-tree hashes,
runner identity, cases and expected counts before production edits. Preserve the
existing modified upstream checkout. Characterize invalid/fractional precision
coercion separately: prefer a native integer precision option validated to 0..=100,
with explicit errors and no JS coercion API. Label native validation differences
rather than presenting them as source parity. Source non-finite acceptance is not
permission to place invalid coordinates into the renderer.

## Architecture, alternatives and proposed API

Preferred: a specialized constructor on the existing generic chart, plus a typed
alias. This exposes the existing built-in mutators without another forwarding
wrapper. Use a distinct constructor name to preserve calendar Chart::new inference.

```rust,ignore
pub struct HorizontalPrice(f64);
impl HorizontalPrice {
    pub fn new(value: f64) -> Result<Self, OptionsChartError>;
    pub fn value(self) -> f64;
}
pub struct OptionsChartOptions { pub precision: u8 }
pub type OptionsChart = Chart<HorzScaleBehaviorPrice>;
impl Chart<HorzScaleBehaviorPrice> {
    pub fn new_options(options: OptionsChartOptions) -> Result<Self, ChartError>;
    // Provide a constructor accepting existing chart/localization overrides.
    pub fn replace_options_chart_options(&mut self, options: OptionsChartOptions)
        -> Result<(), ChartError>;
}
```

Names/signatures are provisional until the compatibility spike. Default precision
is 0. Validate the whole options replacement before mutation. Keep normal full
chart options separate from the narrow horizontal precision option. Reuse existing
BuiltInSeriesDataItem<HorizontalPrice>, SeriesOptionsMap, IDs and errors; do not
invent a second row hierarchy. Give the behavior its own finite, signed-zero-
normalized ordered/hashable price key; do not call a strike Maturity or add prices
to calendar Time. Compare local newtype implementation with any existing suitable
finite-number helper before duplicating utilities.

Alternative: a small OptionsChart wrapper with new()/with_chart_options() and
forwarded operations. It provides constructor symmetry with YieldCurveChart but
adds forwarding with no scaffold invariant to protect. Choose it only if the alias
spike reveals a concrete public API or ownership problem. Neither design needs
runtime dispatch or a generic refactor of the already shared chart core.

```text
OptionsChart = Chart<price behavior>
  -> owns model behavior + data-layer behavior (same validated options)
  -> owns series, markers, primitive attachments and existing widget state
Prepared frame -> owns labels/geometry; no reference back to mutable chart
```

Existing formatter/attachment ownership remains unchanged. No new callbacks,
strong owner captures or Rc cycles are needed. Construction maps the source
factory to the specialization; addSeries/setData/updateData map directly to shared
Rust operations; event subscribers receive HorizontalPrice. Review these mappings
against axis caches, primitive contexts and marker keys before implementing.

## Delivery sequence

1. **Freeze contract and compatibility baselines.** Actual-source oracle for keys,
   labels, weights, defaults, updates and index spacing; pin source hashes and old
   calendar/yield images. Record native validation corrections separately.
2. **Implement pure price behavior.** Finite price type, precision options, identity
   converter, formatter and weights. Start with failing behavioral tests. Resolve
   toFixed parity before exposing the public chart; use installed helpers only if
   they match the actual source cases.
3. **Expose one end-to-end public chart.** Prove alias versus wrapper, construct a
   Line, update precision, fit/zoom/prepare and receive typed pointer data. Then
   cover all built-in families, multi-series/panes, updates and shared extensions.
   Gate: calendar/yield constructors still compile; no duplicated engine/widget.
4. **Deliver a strike-price demo and backend fixtures.** Add isolated options_chart
   example with a Line and Histogram on suitable scales, irregular strikes, live
   updates, precision controls, markers, watermarks/pulse and editable box. Reuse
   existing example support instead of adding a second runtime or profiler.
5. **Validate and record acceptance.** Source comparisons, regression suite,
   software/GPU captures and physical mouse acceptance; write the port report and
   usage documentation. Keep device and historical timing exceptions separate.

The checklist gives dependencies, likely files and per-slice checks. Keep each
compiling slice near 3–5 files; split exports/error plumbing from behavior/API tests
if needed. No user approval is needed between ordinary implementation checkpoints.

## Verification and completion gates

Freeze fixture scenes/counts at checkpoint 1 before production edits. Proposed
feature matrix: 2 backends x 4 DPR values (1/1.25/1.5/2) x 3 sizes
(800x500,300x300,300x90) x 5 scenes = 120 images. Scenes: sparse/irregular Line,
Line+Histogram, other built-in families, precision/crosshair/overlays, empty/reset
recovery. Semantic tests cover every family independently. Compare 24 unchanged
calendar and 24 representative yield baseline images using pinned pre-change
binary/fixture/font identity. Freeze clocks/feeds for deterministic captures.
Browser/native text pixels need not match; native unchanged pixels must match.

Test formatting/weight boundaries, invalid writes preserving state, precision
cache changes, coordinate round trips, missing points and logical union spacing.
Check managed marker seed comparisons, historical updates and idle expiry using
explicit clocks; manual markers persist. Check frozen frames after owner mutation
and drop. Verify hover/zoom/precision updates do not rebuild all history using
focused work assertions, without reopening the deferred broad timing investigation.

Native mouse checks use explicit ICED_BACKEND=tiny-skia and ICED_BACKEND=wgpu:
regular, narrow and earlier problem size; wheel zoom, diagonal pan, price-axis
wheel/drag, numeric crosshair labels, precision controls, live updates, box
move/resize/Escape, idle marker expiry, offscreen pulse and short-height recovery.
Record actual user results per backend; never infer physical acceptance from images.

Final checks: cargo fmt --check; cargo test --all-targets; cargo check --all-targets;
cargo clippy --all-targets -- -D warnings; new actual-source oracle plus affected
existing calendar/yield/axis/marker/primitive/overlay oracles. Inspect representative
captures. Record commands, counts and checked revision in docs/porting/options-chart.md.
No push is part of implementation unless separately requested.

Main risk: JS toFixed and Rust formatting differ at some rounding boundaries and
large magnitudes. Resolve this with actual-source cases, not visual approximation.
Touchpad, touchscreen and physical high-DPI acceptance remain untested; earlier
open timing exceptions retain their status independently of this feature.

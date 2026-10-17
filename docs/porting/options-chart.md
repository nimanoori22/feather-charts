# Price-based horizontal charts

Status: in development. Baseline 53ed228; plan in tasks/options-chart-plan.md.

## Source and contract

Investigated complete create-options-chart.ts and the horz-scale-behavior-price
component (behavior, options and numeric item type), construction/default merging
in create-chart.ts/chart-api.ts, the generic series API and live TimeScale formatter
consumers. Graph tracing has incomplete dynamic interface edges: direct source
reads verify formatDateTime, _formatLabelImpl and _updateDateTimeFormatter.
Pinned revision and actual working-tree hashes: options-chart-source.json.

The factory uses the ordinary chart API, all built-in series and ordinary defaults.
Numeric identity keys select shared logical indices; irregular strikes are equally
spaced at successive indices, with no continuous projection or generated scaffold.
Axis ticks use price-specific priorities and decimal labels. Crosshair formatDateTime
first honors localization.timeFormatter; ticks do not. This distinction is in the
shared TimeScale, not the behavior itself. Precision is independent of vertical
series price formats. Upstream precision omitted at runtime means zero decimals;
invalid precision throws during formatting and fractional precision is truncated.

Native policy: finite immutable HorizontalPrice values, normalized signed zero,
Eq/Ord/Hash consistent with numeric identity; integer precision 0..=100 validated
before construction/replacement. Nonfinite keys and JS precision coercions are
intentionally excluded. Negative and fractional strikes remain valid. No async,
manual destroy or shared mutable options object is introduced.

## Design and ownership

Chosen API: OptionsChart is Chart<HorzScaleBehaviorPrice>, with a distinct
new_options constructor to retain calendar Chart::new inference. The generic owner
already supports typed rows/events/markers/primitives and all built-in preparation.
A separate wrapper was considered but adds forwarding without an invariant to
protect. YieldCurveChart retains its own scaffold; prices do not reuse Maturity.

The chart owns model/data behavior instances with identical validated options,
series and extension attachments. Prepared frames own strings and drawing payloads.
No new callbacks, back references, Rc cycles or destruction order are introduced.
Source addSeries/setData/updateData map to existing generic Rust operations.

## Formatting evidence

scripts/check_options_chart_reference.cjs executes the actual pinned TS behavior,
checks factory delegation with a narrow createChartEx observation boundary, and
compares 6,578 frozen item/tick/weight cases to options-chart-cases.tsv. This is not
a browser/DOM integration test. Cases include every precision 0..100, signed values,
subnormals, large magnitudes, exact halfway cases and deterministic random f64 bits.
Invalid/fractional precision and omitted precision are independently characterized.

Behavioral RED: ordinary Rust formatting emitted -2 for upstream -2.5 -> -3 at
precision 0. Native formatting now corrects only exact binary halfway cases where
ties-to-even rounded down, without scaling the float or adding an epsilon. Values
>=1e21 use shortest scientific notation with the source positive exponent sign.
The retained pure source comparison passes all 6,578 cases.

## Validation and acceptance

Pure tests pass; integration, backend captures and physical acceptance pending.
Retained pre-change artifacts: target/options-chart/before, 24 calendar and 24 yield
images, with original capture protocols. These retained images must be compared
against freshly built old-family fixtures before acceptance.

The historical timing exception and untested touchpad/touchscreen/physical high-DPI
checks remain separate and open. No options valuation/feed/custom renderer is added.

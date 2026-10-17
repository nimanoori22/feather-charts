# Price-based horizontal charts

Status: planned scope complete; automated and software/GPU mouse acceptance passed.
Baseline 53ed228; implementation checkpoints 207ac46, af9dde2 and 0019701.
Plan: [options-chart-plan.md](../../tasks/options-chart-plan.md).

## Source and contract

Investigated complete create-options-chart.ts and the horz-scale-behavior-price
component (behavior, options and numeric item type), construction/default merging
in create-chart.ts/chart-api.ts, the generic series API and live TimeScale formatter
consumers. Graph tracing has incomplete dynamic interface edges: direct source
reads verify formatDateTime, _formatLabelImpl and _updateDateTimeFormatter. Coverage
metadata was stale for upstream TimeScale/DataLayer and shared row/helper files;
relevant methods were read directly and actual working-tree code was executed.
Rust evidence paths matched index metadata. Iced scrollable direction/enclosure
and Flowsurface's scrollable-content example were inspected for the narrow toolbar;
engine and interaction logic needed no new runtime APIs.
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

## Files changed

Production Rust:

- src/model/horz_scale_behavior_price.rs: finite item, options/errors, conversion,
  numeric keys, fixed-decimal formatting, tick/append weights and pure tests.
- src/model/mod.rs: behavior export.
- src/api/options_chart.rs: typed alias, constructors, precision replacement,
  crosshair formatter and ChartBehavior adapter.
- src/api/chart.rs: price-specific conversion/data error variants.
- src/api/mod.rs: public exports; src/api/behavior.rs: family description.

Examples and verification:

- examples/options_chart.rs: native strike Line + volume Histogram demo, two
  explicit vertical axes, horizontal toolbar and live update/precision controls.
  Volume uses a demo-specific lower scale margin; chart API defaults remain ordinary.
- examples/options_chart/region.rs: example-owned top-layer editable box and real
  widget move/resize/Escape tests. Top-layer placement prevents the Histogram from
  covering its hit target; this is a demo choice, not a change to core arbitration.
- examples/options_chart/captures.rs and scripts/capture_options_chart.py: frozen
  five-scene, two-renderer matrix and exact old-family pixel comparisons.
- tests/options_chart.rs: public behavior/lifecycle, all families and extensions.
- scripts/check_options_chart_reference.cjs, options-chart-source.json,
  options-chart-cases.tsv and options-chart-states.tsv: pinned actual-source oracle.
- README.md, this report and tasks/options-chart-{plan,todo}.md: API/use/evidence.

No Cargo dependency, Iced vendor or other chart engine module was changed.

## Behavioral tests

Pure tests cover the 6,578 frozen source cases, partial weight filling, empty max
weight, invalid prices/options and signed-zero order/hash consistency. Native
empty weight selection returns the existing default rather than reproducing a JS
empty-array exception, since empty charts must prepare safely.

Eight public integration tests cover constructor inference alongside calendar/yield,
irregular index spacing, transactionally rejected options/writes, label cache
refresh, unchanged vertical labels and canonical row storage, typed numeric click
and whitespace payloads, history/append/remove/clear, every built-in's visible
geometry and short-height recovery, nine upstream real DataLayer state sequences,
seed-relative update markers, explicit-clock expiry/manual persistence, multiple
panes, annotation markers, both watermarks and offscreen pulse suppression. The
existing shared mutation/prepare/widget code is reused without a history traversal
in the precision setter; the behavior's append weight needs only the current item.

Demo widget tests distinguish box moving/right-edge resizing and verify both paths
restore their original geometry on Escape. Capture assertions exercise empty/reset
recovery, manual marker visibility and immutable old frames after owner removal.

## Validation and acceptance (2026-10-17)

- cargo test --offline --all-targets: **680 passed**, 64 suites, zero failed/ignored.
  Full log: target/options-chart/all-targets-tests.log. After the final demo layout
  adjustment and stronger review assertions, the affected eight integration tests
  and two example tests were rerun successfully; other production code was unchanged.
- cargo fmt --check; cargo check --offline --all-targets;
  cargo clippy --offline --all-targets -- -D warnings: all pass on final code.
- **28 source oracle scripts pass**, including all existing family, axis, layout,
  interaction, primitive, marker, watermark, animation, conflation and yield checks.
  Logs/counts: target/options-chart/oracles/{results.json,check_*_reference.log}.
  New oracle additionally exercises actual TimeScale coordinate and custom-crosshair
  methods and nine actual DataLayer union/update/whitespace/remove/clear states.
- **120 final feature captures** across tiny-skia/wgpu, DPR 1/1.25/1.5/2 and
  800x500/300x300/300x90: all pass. **24 calendar + 24 yield** images match retained
  pre-change pixels exactly. Browser/native font pixels are not compared.
  Final evidence: target/options-chart/final-build/{protocol.json,results.json}.
  Binary, source, fixture and vendor identities were checked against that protocol.
  Representative large/narrow/short captures were inspected; the revised volume
  scale leaves the line and watermark visible. Earlier captures remain retained.
- Final local review covered correctness, ownership, validation, compatibility,
  unnecessary dependencies and hot-path work; no unresolved implementation defect.
- ICED_BACKEND=tiny-skia native run: user reported **all checks passed; idle expiry
  passed; window closed**. ICED_BACKEND=wgpu run: the same result. Both native
  processes exited normally; logs at target/options-chart/mouse-{tiny-skia,wgpu}.log.
  Checks covered controls, wheel zoom, diagonal pan, both vertical scale wheel/drag,
  numeric crosshair, box move/resize/Escape, short-height recovery at regular,
  narrow and the earlier problem size, idle expiration and persistent manual markers.

The source factory oracle observes delegation rather than constructing a browser
DOM; native interactive charts and source behavior/data/scale methods are exercised
separately. This is not a claim of pixel-identical browser text rendering.

## Remaining dependencies and limits

Numeric custom-series renderers and arbitrary custom horizontal behavior factories
remain outside this scope. Option valuation, Greeks and market-data feeds are not
part of createOptionsChart or this port. No additional upstream component blocks
the delivered built-in price-chart path.

The historical timing exception and untested touchpad/touchscreen/physical high-DPI
checks remain separate and open. This feature's mouse acceptance does not close
those gaps. Commits are local; no push was requested.

# Yield-curve chart port

Implemented; automated and software/GPU mouse acceptance pass. Baseline `24bd3ba`; plan commit `1112050`.
Source working-tree hashes: [yield-curve-source.json](yield-curve-source.json).

## Contract checkpoint

Source files, consumers, alternatives and provisional API are documented in
[tasks/yield-curve-plan.md](../../tasks/yield-curve-plan.md). The source oracle
executes the original behavior, whitespace helpers and DataLayer with explicit
microtasks. Twenty default/custom labels and weights, eight scaffold states and
nine real DataLayer lifecycle states are frozen before production edits. It writes
`target/yield-curve/source-contract.json` for native comparison. Extent remains
420 months after shorter replacement, longest-series removal and clearing public
data; native settlement preserves this source scaffold feedback behavior.

Finite negative/fractional maturities are accepted; signed zero is normalized.
The scaffold clamps start/minimum to zero and resolution to at least one. Its
ordered union with real maturities defines coordinates; off-grid points are not
reprojected onto an invented continuously linear axis. Default formatting reads
localization.timeFormatter, not the declared but unused yieldCurve.formatTime.

Native validation rejects nonfinite options/maturities, invalid row/value/order
and scaffold counts above 100,000 points before mutation. This explicit resource
limit is a native correction, not source parity. The budget allows over 8,000 years
at monthly resolution while bounding support memory/work. Explicit option
replacement will settle the scaffold coherently rather than depending on a later
browser microtask/data mutation. Public iteration/events hide the private support
series. Numeric custom series and arbitrary chart-family plugins are deferred.

## Chosen design and ownership

A shared behavior-generic chart core with calendar defaults is preferred over a
separate numeric frontend: it reuses mutation, preparation, event and widget code
without duplicating an engine or navigation loop. A typed yield frontend limits
public built-in additions to Line/Area and owns private scaffold state. A Time-or-
Number enum was rejected: it would admit mixed invalid data and branch hot paths.

Chart owns model/data plus attachments and marker controllers. Numeric scaffold
state is owned by the yield chart; behaviors never reference their owners. Shared
primitive state has no chart back-reference. Preparation freezes coordinates,
colors and labels; old frames retain no live scaffold/controller state. Formatter
closures use existing Rc callbacks; hosts must avoid strong owner captures.

The calendar API and third-party calendar DrawingPrimitive implementations must
remain source-compatible. Numeric event payloads and primitive contexts carry
numeric items. Time-specific custom-series methods remain specialized. No async
engine, locks, new dependency or renderer patch is planned.

Validation, exact files, API, capture and native acceptance results will be added
as each gate completes. Historical timing acceptance remains OPEN/deferred;
touchpad, touchscreen and physical high-DPI checks remain separate.

## Implemented API and consumer mapping

`Chart<B = HorzScaleBehaviorTime>` now carries the behavior through its existing
owner methods. Calendar `Chart::new`, `Chart::default`, events, marker items and
`DrawingPrimitive` implementations retain calendar defaults. Time-specific custom
series stay specialized. `ChartBehavior` supplies validated converters and error
mapping; it is an owner adapter, not a runtime plugin registry.

`YieldCurveChart` wraps `Chart<YieldCurveHorzScaleBehavior>`. Read-only dereferencing
reuses inspection/frame/widget methods; there is no mutable core escape. Numeric
mutations and preparation go through explicit frontend methods, keeping scaffold
settlement under one owner. Line/Area additions accept existing series option
patches, allowing explicit overrides of yield defaults. `with_chart_options` takes
fully owned chart/localization overrides; `new` supplies yield axis defaults.
Errors reuse ChartError with YieldCurve/YieldData variants. `Maturity::from_months`
is the finite-number boundary; `months()` exposes its value. Numeric items reuse
BuiltInSeriesDataItem<Maturity>, not a second row hierarchy.

`set_data`, `update_data` and managed marker writes mark settlement dirty after
successful canonical mutation. The next prepare/update computes effective scaffold
state from the current shared domain, including retained support. Batched transient
extensions that are replaced before settlement do not grow support. Equal scaffold
state does not regenerate keys. The support owner is absent from public series IDs,
pointer data and rendered views; normal fulfilled membership excludes its rows.
It still participates in horizontal indices and source-compatible retained extent.

`set_maturity_formatter` invalidates horizontal label caches and keeps model/data
behaviors consistent. Formatter callbacks retain the existing localization/series
ownership pattern. Three-decimal percent formatting is applied to the support owner
as well as public curves: that owner can lead the price axis even without values.
Price values are already percentages; 5.378 formats as 5.378%, never 537.8%.

Numeric up/down commands keep seeded comparisons, explicit clocks, manual markers,
error atomicity and shared idle scheduling. `prepare_snapshot` now advances marker
expiration before preparing, just like full preparation and Tick, closing a shared
headless gap. Prepared attachment entries may remain after expiry, but their views
are empty; tests check drawing payload rather than attachment-list length.

Source browser microtasks become a dirty-state settlement at existing synchronous
boundaries. Callback identity, linked owners, DOM creation and explicit destroy
methods are discarded. RAII owns the support state and attachments. No behavior
callback points back to Chart, and frozen frames own no live scaffold data.

## Tests and validation

- Actual-source comparison passes 20 default labels, 20 weights, 20 custom labels,
  eight scaffold parameter cases and nine DataLayer/microtask lifecycle states.
  The real public factory/YieldChartApi constructor/default/restriction boundary
  executes with a narrow base-chart stub; this is not a browser DOM/render test.
- The startup harness was corrected to set its initial scaffold hash before the
  queued callback, matching the source constructor and avoiding a fixture-only
  duplicate rebuild. Contract keys/case counts were preserved.
- Compiling REDs cover maturity month/year formatting, headless snapshot expiry
  and support-owner percent formatting. The expiry assertion was refined to test
  empty renderer views; it also fails without the fix. Compiler diagnostics are
  not counted as behavioral REDs.
- Public tests cover sparse visible curves, numeric pointer payloads, exclusion of
  support-only selections, transactional errors, exact/manual expiry, pulse fast-path
  reuse, mutable formatter caches and one-region software damage at three sizes.
- A private 25-update work check proves no scaffold regeneration on same-domain
  changes, ticks or panning and no growth for an unsettled transient long curve.
  Source timeline tests cover Area, history, extension, whitespace, reset and removal.
- Demo tests exercise box hit routing, move/right-edge resize and Escape rollback
  through the actual shared widget/ChartMessage path. The box is application-owned,
  uses logical horizontal anchors and demonstrates the existing edit router.
- Final all-target suite: **668 passed, zero failed/ignored, 62 suites**. All-target
  check, strict Clippy and formatting pass. The final demo status text uses the
  existing nonwrapping pattern to avoid hover-induced layout changes; its focused
  tests/check/Clippy pass after that UI-only change.
- Calendar source oracles pass: 192 axis cases, 401 interaction cases, 5,898 primitive
  cases, 980 annotation placements/84,700 hits, plus up/down marker, pulse, conflation,
  text watermark and image watermark comparisons. Native reference binaries were
  rebuilt before executing those that depend on examples.

The final capture matrix passes **120 yield images and 24 unchanged calendar
images**, actual tiny-skia/wgpu at DPR 1/1.25/1.5/2 and sizes 800x500/300x300/300x90.
Five scenes cover sparse Line, Line+Area, changed domain/formatter, numeric markers
with mixed primitives/watermarks, and cleared data after reset/recovery. Nonempty
scenes assert visible curve geometry; markers differ from cleared output whenever
plot height is nondegenerate. Frozen frames draw identically twice and after owner
removal. Representative labels/spacing/percent axes/clipping/mixed scenes were viewed.

Raw images: `target/yield-curve/final`; calendar comparisons:
`target/yield-curve/calendar-final`. Baselines reuse verified exact-binary retained
images from the preceding feature, with provenance in the baseline protocol. Source,
fixture, binary/vendor/font hashes and logs are retained in
[evidence/yield-curve](evidence/yield-curve). Final source hashes match the captures.

The first capture used the multi-pane-only accessor on a single-pane chart and
failed on both backends. Its logs remain under `target/yield-curve/after`. The next
matrix passed and visual review exposed decimal-only yield-axis formatting; that
real formatter defect received a failing test and fix, then the final captures
were rerun. No acceptance assertion or timing threshold was weakened.

## Files changed and dependencies

- `src/api/behavior.rs`, `chart.rs`, `mod.rs`: generic owner boundary, errors and
  calendar defaults; shared snapshot expiration.
- `src/api/primitives.rs`, `primitive_editing.rs`, `interaction.rs`, `widget.rs`,
  `touch.rs`, `conflation.rs`: reuse ownership, typed event/primitive contexts and
  current input/preparation/scheduling through both families.
- `src/api/markers.rs`, `up_down_markers.rs`, `text_watermark.rs`, `image_watermark.rs`:
  item-generic annotation/marker state and shared watermark preparation.
- `src/api/defaults.rs`: Default for existing series option patches.
- `src/api/yield_curve.rs` and private tests: typed frontend, source defaults,
  bounded/coalesced scaffold, lifecycle/reference/work checks.
- `src/model/yield_curve_horz_scale_behavior.rs`, `model/mod.rs`: maturity types,
  numeric keys, options, formatting/weighting and scaffold calculations.
- `src/model/chart_model.rs`, `time_scale.rs`: existing behavior/localization update
  path, label-cache invalidation and crate-local last-domain lookup.
- `tests/yield_curve.rs`: public numeric integration, deadline/format regressions,
  software damage and pulse reuse.
- `examples/yield_curve.rs`, `examples/yield_curve/captures.rs`, `region.rs`: demo,
  frozen captures and example-owned editable box.
- `scripts/check_yield_curve_reference.cjs`, `scripts/capture_yield_curve.py`, README,
  source manifest, evidence, feature plan and checklist: reference verification,
  native acceptance and public documentation.

The existing generic model/data/scale, Line/Area preparation, Iced widget, primitive
router and renderers supply the dependencies. No new package, runtime or vendored
renderer patch was added. Generic type/storage slices were checked separately;
propagating owner methods then required a small coordinated change across dependent
impl blocks to keep the crate compiling. Those methods remain one shared path.
Numeric custom series, price-axis/options charts and an arbitrary custom horizontal
chart API remain separate milestones. This feature does not close the older timing
exception or supply missing physical devices.

## Physical acceptance

On 2026-10-17 the user reported all checks passed on both explicitly selected
software and GPU windows, including curve/domain/label controls, mouse wheel zoom,
diagonal pan, left price-axis wheel/drag, numeric crosshair, box move/right-edge
resize/Escape and short-height recovery at regular, narrow and earlier problem
sizes. Five-second expiration with no mouse movement after pausing the feed and
indefinite manual marker persistence both passed. Both windows closed normally
(exit 0). Results are retained in native-acceptance.json.

Touchpad, touchscreen and physical high-DPI checks remain untested. Historical
timing acceptance stays OPEN/deferred. No push was requested or performed.

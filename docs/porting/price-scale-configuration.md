# Price-scale controls and runtime configuration

Status: complete, 2026-10-18. Automated/source/capture checks and available software/GPU mouse acceptance passed in all three chart modes.
Plan: [implementation slices](../../tasks/price-scale-configuration-plan.md).

## Source and consumers

Investigated the complete PriceScaleApi/IPriceScaleApi, custom range and mode
state in PriceScale, ChartModel chart/scale patch routing, Pane.applyScaleOptions,
ChartApi.applyOptions and ChartWidget.applyOptions. Dependencies include range
conversion, minimum-move precision and lazy range calculation. Actual consumers
include public range/auto graphics tests and coverage cases, the logarithmic
price-scale unit test, and customization/accessibility/Vue callers of chart option
updates. Graph traces link public options through widget/model/panes to scales;
CSS listeners/sizing remain widget concerns. Graph coverage is best effort; direct
source reads are authoritative, particularly sparse Rust public call edges.

Required behavior: source manual ranges disable autoscale and remain after live
updates; auto-on permits recalculation. Source setters use internal range units:
in log mode setting 4..5 reads as 1..10, rounded to min-move precision. Percentage
and indexed ranges are scale units. The real-browser fixture pins these independent
setter/getter observations in all four modes. Do not silently convert setter inputs
to prices in log mode. Queries return owned optional ranges. Chart-wide scale patches
reach current and future panes; per-pane left/right patches also update construction
defaults, without applying that change to unrelated current panes. Overlay-specific
patches affect the existing scale, without creating one or altering defaults.

Source evidence: `price-scale-configuration-source.json` pins bundled input hashes;
`price-scale-configuration-states.json` records browser results. Run
`node scripts/check_price_scale_configuration_reference.cjs --source-only`.

Rust deliberately rejects nonfinite, reversed, zero-width and overflowing ranges
before mutation. Log ranges whose inverse conversion overflows are also invalid.
Invalid pane/overlay and invalid mixed patches are errors without partial mutation.
These strengthen the permissive source boundary. Source manual destroy, shared
mutable options, JS partial-object merging and retained widget references are
replaced by owned typed patches and chart methods with existing selectors.

## Design, ownership and API

Use Chart methods with `(pane, &PriceScalePosition)` and the existing public
PriceRange. Chart owns model, panes and scales; query results/patches are owned.
Existing Rc series and immutable frames retain their lifetime behavior. No new
backreferences, callbacks, threads or dependencies. A long-lived scale handle would
require identity/lifetime semantics through pane removal; full construction-option
replacement would reset unspecified settings. Neither is needed. Aggregate patch
validation followed by one model application is preferred over repeated public
setters that can partially commit and flush each field.

The planned signatures are retained. Add `apply_price_scale_options` for named
overlay targeting as well as axes; existing side-specific entry points remain.
YieldCurveChart explicitly forwards mutations. Read-only methods use its existing
Deref. Numeric horizontal formatters and retained maturity scaffold stay intact.

## Field inventory

| Source/native section | Public update/readback | Validation and effects |
|---|---|---|
| layout.background | LayoutOptionsPatch.background / runtime options | CSS colors; plot gradient and axis fills |
| layout.textColor, fontSize, fontFamily | layout patch / runtime options | colors, finite positive size/nonempty family; refresh typography/axis measurements |
| layout.panes | typed pane-layout patch / runtime options | separator colors; widget resize flag and divider rendering |
| grid.vertLines, horzLines | GridOptionsPatch of GridLineOptionsPatch / runtime options | color; visibility/style; invalidate pane grids |
| leftPriceScale, rightPriceScale, overlayPriceScales | existing PriceScaleOptionsPatch / defaults and per-scale options | margins, colors, finite dimensions/density; scale modes, marks, axes; current and future panes |
| timeScale | existing HorzScaleOptionsPatch / time-scale options | finite geometry, nonnegative dimensions, spacing bounds, color; behavior sync and time/price invalidation |
| crosshair | existing CrosshairOptions replacement / runtime options | existing color validation; preserve pointer origin and rebuild lines/labels |
| handleScroll/handleScale native subset | MouseNavigationOptions, TouchNavigationOptions, KineticScrollOptions / runtime options | typed booleans/enums; widget input routing and gesture cancellation |
| pane stretch/preserve | existing per-pane methods | existing finite validation and layout invalidation |

No successful no-op fields are added. Color-space/parsers, DOM sizing and attribution
drawing remain outside the patch type. Crosshair/navigation use explicit typed section
replacement, matching existing setters; layout/grid/scale/time use partial fields.

Appearance-only patches (colors/grid/crosshair) preserve active edit/drag state.
Typography, scale/time geometry, pane resizing and navigation changes cancel stale
viewport captures using existing policy. Equal/empty patches do not cancel. Manual
range state belongs to a scale; series transfers adopt destination state. Invalid
operations preserve gestures. Every caller resolves the current pane index.

## Verification and remaining work

Source probe passed before production changes. Implementation/test/capture/native
acceptance results will be recorded here as slices complete. Independent previous
drawing-order mouse acceptance, timing exceptions and untested physical devices
remain open. No acceptance threshold or source fixture is weakened by this work.

## Implementation notes

`ChartOptionsPatch` separates backend-independent `model: ChartModelOptionsPatch`
from mouse/touch/kinetic component settings. The model stays independent of Iced
input types and the API layer. `RuntimeChartOptions` mirrors that structure with
owned readbacks. A single preflight resolves layout/grid/defaults/time/crosshair
and validates every affected live scale before any input or model mutation. Model
application batches the normal invalidation/effect path; no canonical series rows
are copied or replaced.

The source option-mode transition differs from direct PriceScale.setMode:
applyOptions merges mode before the transition call. The public patch path now
preserves that source ordering; direct engine mode transitions retain their existing
conversion semantics. Additive source probes pin both log/normal manual transitions
and percentage/indexed autoscale forcing. Default options retain merged autoScale,
while current and newly constructed special-mode scales force auto-on. Manual
left/right range setting also updates future side defaults through auto-off.

The edit viewport compares coordinate-affecting scale settings, so changing text
or border colors no longer cancels an edit or prevents restoring its temporary
autoscale policy. A regression failed before this correction and passes afterward.
Geometry/input mutations advance the existing interaction revision; invalid and
empty patches preserve active state. Mouse settings reach both Canvas input paths;
axis input remains its separate existing control (the plot-mouse toggle does not
implicitly disable price-axis interactions).

Exact Iced Canvas Program source confirms borrowed program snapshots and separate
retained State: pass new input settings through each program snapshot, reset stale
captures through the existing revision, and retain event-driven redraw behavior.
Flowsurface ViewConfig is a reference for owned chart-view state, not a reason to
copy its autoscale or chart architecture.

Verification artifacts are under `target/price-scale-configuration/`. Failing
range/config API tests, color-edit regression and mode-transition regression are
retained alongside the passing results. Initial test setup errors (a nonexistent
SeriesOptionsMap default, field-name typo, macro ambiguity and unavailable test-only
JSON crate) were corrected using existing constructors, imports and dependency-free
JSON diagnostic formatting; they are not claimed as engine defects.

The final-source-removal regression exposed an existing native gap: source
PriceScale.removeDataSource enables auto and clears its range after the last owner
leaves. Rust now does the same; pop-to-empty remains a different operation with an
attached owner and keeps its manual policy.

The first capture run caught a second lifecycle defect on both renderers:
`live-manual` updated data before the first frame established a horizontal range.
Pane invalidation cleared a manual range when the horizontal range was absent.
Source Pane._recalculatePriceScaleImpl skips recalculation in that state. The native
lazy and eager paths now preserve manual ranges until auto is enabled; final source
removal still clears them through its own lifecycle. A focused API regression fails
with the pre-fix code after a latest update before first prepare, then passes after
the correction. Failed matrix results remain in `accepted/`; the final replay uses
a separate directory and unchanged scene expectations/baselines.

## Files changed

Production:
- `src/api/price_scale.rs`: new range/readback/auto/overlay patch API.
- `src/api/configuration.rs`: aggregate runtime patch and owned readback/input APIs.
- `src/api/mod.rs`, `chart.rs`: exports, error and component state, existing axis setter routing.
- `src/api/yield_curve.rs`: explicit mutation forwarding.
- `src/api/widget.rs`: mouse options reach single/multi-pane Canvas input.
- `src/api/primitive_editing.rs`: style-safe edit viewport comparison and policy restoration.
- `src/model/chart_configuration.rs`: owned model patches/readbacks and shared validation.
- `src/model/chart_model/configuration.rs`: resolved preflight, defaults, scale lookup/mutation and batched model application.
- `src/model/chart_model.rs`, `mod.rs`: register modules/error.
- `src/model/pane.rs`: existing/all-scale configuration, special-mode construction and manual-range preservation without horizontal layout.
- `src/model/price_scale.rs`: log range readback, source patch ordering and last-source cleanup.
- `src/ui/chart_input.rs`: equality for existing mouse option values.

Tests/examples/proof:
- `tests/price_scale_configuration.rs`, `chart_configuration.rs`: new public behavioral suites.
- `tests/pane_management_interaction.rs`, `primitive_editing.rs`: real Canvas gesture/edit regressions.
- `examples/pane_management.rs`, `pane_management/{configuration,host,captures}.rs`: shared opt-in controls, forwarding, demo tests and immutable-frame matrix.
- `scripts/check_price_scale_configuration_reference.cjs`: real source bundle/browser and native sequence comparisons.
- `scripts/capture_price_scale_configuration.py`, `capture_series_order.py`: reuse existing runner with additional frozen scenes; old modes unchanged.
- README, source/state JSON, this report and feature plan/checklist retain the contract/evidence.

## Acceptance protocol

Final feature matrix: calendar/strike/maturity × tiny-skia/wgpu × 800x500, 300x300,
300x90 × DPR 1, 1.25, 1.5, 2 × automatic/manual/live-manual/restored/appearance/log-manual
= **432 feature images**. The runner asserts the actual renderer, expected manual/auto
state, and fixed manual ranges after live updates. Retained frames must redraw
byte-identically after options, swap, transfer, removal and chart drop. It also
compares **96 existing compatibility images** byte-for-byte against the reused last
accepted baseline; the first failed run already passed all those comparisons.

New source comparison checks 12 range/update/restore states plus four mode
transitions against native results, and preserves source defaults observations.
All 32 prior comparison scripts run unchanged alongside it. Formatting, all-target
tests, doctests, all-target check and strict Clippy are mandatory final gates.

| Native backend | Calendar / strike / maturity | Result |
|---|---|---|
| tiny-skia | Range/auto/options controls, live mouse/box gestures at regular/narrow/problem sizes and short-height recovery | User reported all checks passed in all three modes; all three windows closed normally (exit 0) |
| wgpu | Same checks | User reported all checks passed in all three modes; all three windows closed normally (exit 0) |

Physical acceptance is recorded only from actual user responses. Existing unrelated
acceptance records stay open independently. No touchpad/touchscreen/high-DPI physical
result is inferred from DPR captures.

Final mode review added a rendered-marks regression: after normal manual marks have
been cached, switching mode must yield the same marks as a freshly configured log
scale. It reproduced stale 4.00/4.50/5.00 labels in place of the log scale's 10.00
mark. Mode patches now rebuild/invalidate marks explicitly after source-style mode
merging. Direct setMode conversion semantics remain unchanged. Evidence:
`mode-marks-cache-red.log` and `mode-marks-green.log`; the initial test setup lacked
an attached formatter source and was corrected before reproducing the defect.

## Final-build automated results

**745 tests passed in 73 suites**, zero failed or ignored. **2 compile-fail
doctests passed**. Strict all-target Clippy, all-target cargo check, formatting
and diff hygiene pass. Evidence:
- `target/price-scale-configuration/all-targets-final-build.log`
- `target/price-scale-configuration/doctests-final-build.log`
- `target/price-scale-configuration/clippy-final-build.log`
- `target/price-scale-configuration/check-final-build.log`

All **33 source comparisons passed**, including the 32 existing scripts unchanged:
`target/price-scale-configuration/oracles-final-build/results.json`.
The first corrected renderer replay passed **432 feature images and 96 exact
compatibility comparisons** in `accepted-final/`. The final cache-invalidation
build also passed **432 feature images and 96 exact comparisons** in `final-build/`
(`protocol.json`, `results.json` and `captures-final-build.log`). All recorded
production, fixture, vendor and executable hashes were checked against current
files. The summary is `target/price-scale-configuration/verification.json`.

Tests cover manual/auto ranges in all modes, log rounding/source transitions,
invalid inputs/targets, pane-owned ranges and overlays, future defaults, last-source
cleanup, live update before first frame, invalid mixed patches, visual gradient and
font changes, real Canvas mouse disabling/re-enabling and edit-policy restoration.
Existing pane/order/data/custom/marker/watermark/gesture suites remain passing.
Twelve generic demo tests include three new configuration-control tests, one for
each chart family. Captures additionally verify fixed ranges after the one explicit
live-feed mutation; continuous feed and price pulse are disabled during rasterization.

Software calendar appearance at 800x500, strike manual at 300x300 and maturity
live-manual at 300x90 were visually inspected. Final-build wgpu strike appearance
at DPR 1.5 (1200x750) was also inspected. Gradient, typography, fixed range and
short-height geometry appeared as expected. These inspections do not substitute
for physical acceptance.

## Native acceptance completed

On 2026-10-18 the user reported **“All checks passed in all three modes; windows
closed”** for both software and GPU sessions. All six processes exited 0.
Commands used explicit `ICED_BACKEND=tiny-skia` or `ICED_BACKEND=wgpu` with
`target/debug/examples/pane_management --price-scale-configuration --live-feed`,
plus `--options` for strike or `--yield-curve` for maturity. Logs:
`target/price-scale-configuration/mouse-{calendar,price,yield}-{tiny-skia,wgpu}.log`.

The accepted checks cover Manual range/Narrow/Widen/Freeze with feed running,
restoring Autoscale, Mode/Invert/Margins, Appearance/Grid/Crosshair, invalid input,
plot-mouse and pane-resize toggles and re-enabling them, and pane/axis selection.
Wheel zoom, diagonal dragging, price-axis wheel/drag and box move/resize/Escape
passed at regular, narrow and the earlier problem size. Short-height shrink/enlarge
recovery passed. Controls act on the selected scale; empty-scale controls do not
move series on another scale.

This completes this milestone's available mouse scope. It does not close earlier
independent drawing-order acceptance or timing exceptions, and does not imply
physical touchpad/touchscreen/high-DPI testing. New public events, screenshot/export
and attribution drawing remain separate development work.

# Price-scale controls and runtime configuration

Status: source contract frozen; implementation in progress, 2026-10-18.
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

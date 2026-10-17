# Series drawing order port

Status: implemented; automated checks passed on 2026-10-18.
Automated, source and capture verification passed; software/wgpu mouse acceptance is pending.
Plan: [series-order-plan](../../tasks/series-order-plan.md).

## Source component and consumers

Local Lightweight Charts 5.2.1 SeriesApi.seriesOrder/setSeriesOrder delegate to
Pane. Pane removes/inserts the same source, renumbers z-orders and invalidates
ordered/render caches. PaneWidget renders stable order with a temporary hovered
foreground override. Pane hit testing uses reverse stable order and distance/
priority arbitration; raising a hovered series must not change stored order.
PriceScale uses the lowest z-order formatter; mutations refresh left/right caches.
Add/remove, scale reassignment and moveSeriesToPane preserve or renumber pane-local
positions. The original change-series-order.js tests area order and scale changes.

Inspected complete series-api.ts, pane.ts, yield-chart-api.ts, sort-sources.ts and
hovered-source-order.ts, plus rendering/hit/price-scale/model consumers and original
ordering tests. Graph search/traces were supplemented with literal caller searches.
Coverage was best effort; sort-sources.ts reported changed metadata and was read
in full. Exact source hashes are in series-order-source.json.

Real browser source probe: scripts/check_series_order_reference.cjs. No chart/model
stubs. Source states, maturity indices and formatter observations are frozen in
series-order-states.json; target/series-order/source-results.json retains execution.
The support series occupies a slot: initial public maturity indices are 1 and 2;
a public series can move to index 0, shifting the support slot. A transferred
series starts at index 0 in a destination without support. Detached source API
returns -1 and its setter does nothing.

## Contract and native policies

Public Chart<B>::series_order(id) -> Result<usize, ChartError> and
set_series_order(id, usize) -> Result<(), ChartError> apply to built-in/custom
owners. Calendar and OptionsChart share the generic implementation;
YieldCurveChart explicitly forwards mutations. The private support ID remains
excluded from public ownership validation, while it still counts toward indices.

Indices clamp to the pane's final source. Data, IDs, views, typed handles and
attachments survive; no historical rows are copied. Unknown/detached IDs use
existing typed errors; unsigned Rust indices exclude JS negative/fractional/NaN
inputs. Valid identity operations preserve active interaction and invalidation.
An actual mutation uses existing viewport cancellation exactly once and clears
cached hits; the next normal prepare settles labels, layer associations and hover.
Old prepared frames remain immutable.

The browser probe records two existing source/native formatter differences:
upstream refreshes only left/right formatters on reorder, leaving a warmed overlay
formatter unchanged; native Pane already refreshes overlays too. Upstream chooses
the lowest source even when hidden; native PriceScale already selects a visible
leader. Preserve these established native policies and test them explicitly;
source observations remain separate and are not claimed as exact native parity.

## Rust design and ownership

Chosen: reuse Pane.set_series_order through ChartModel, consume its light-update
effects, then permute the existing per-pane view entries to match model order.
Chart owns model/panes and view vectors; adapters and attachment owner IDs remain
unchanged. No new references, strong cycles or destruction rules are introduced.
Prepared frames own their snapshots independently. Work depends on source count,
not row count; no threads or dependencies are added.

Alternative: keep view storage unchanged and look up model order during every
prepare. This requires more lookup/borrowing paths and coordinating the first
view/layer everywhere; mutation-time alignment fits existing pane-management code.

## Baseline and validation

Baseline source identity verified against the previous accepted capture protocol.
The prior 96 compatibility images are copied into target/series-order/before.
The new 288-image matrix is frozen in target/series-order/baseline.json before
production changes. Capture and physical acceptance are recorded below.
Historical timing and unavailable-device checks are separate and remain open.


## Files changed

Production:
- `src/api/pane_management.rs`: validated public query/setter and existing view permutation.
- `src/model/chart_model/pane_management.rs`: pane operation/effect coordination and no-op preservation.
- `src/model/pane.rs`: read-only current source position, reusing source identity lookup.
- `src/api/yield_curve.rs`: explicit ordering and hover-raising forwards with private IDs protected.

Examples/tooling:
- `examples/pane_management.rs`: opt-in --series-order scene and existing-host controls.
- `examples/pane_management/host.rs`: forwarding and overlapping area setup.
- `examples/pane_management/captures.rs`: four ordering scenes and retained-frame assertions.
- `scripts/capture_series_order.py`: frozen matrix and exact compatibility comparisons.
- `scripts/check_series_order_reference.cjs`: real-browser source probe/native state comparison.
- `README.md`: API use, maturity indices, native formatter policy and demo commands.
- `tests/series_order.rs`, `tests/pane_management_interaction.rs`,
  `tests/primitive_editing.rs`: behavioral and capture-lifecycle regression coverage.
- This report, source hash/state JSON, and the feature plan/checklist record the contract and evidence.

## Tests and automated results

`target/series-order/all-targets.log`: **720 passed in 70 suites**, zero failed or
ignored. `target/series-order/doctests.log`: **2 compile-fail doctests passed**.
`clippy.log`, `check.log`: strict all-target Clippy and all-target cargo check pass.
Formatting and `git diff --check` pass. No dependency or renderer/widget changes.

The eight ordering tests cover next-frame layer order; the ten-state browser/native
sequence with a custom series; maturity hidden support and transfers; no custom row
copies/view destruction; all six built-in families with canonical row pointers,
marker layer remapping, price lines and subsequent historical/latest updates;
warmed side/overlay formatter leadership and hidden-source behavior; stable hover
arbitration versus transient raising; and primitive layer ties after reorder.
The existing interaction suites now include reordering in real Canvas plot, axis,
divider, kinetic and captured primitive edit cancellation, plus rejected/identity
calls preserving an active drag. Demo tests cover six normal/ordering chart modes,
short-height recovery and live updates after transfer/reset.

Red evidence: `target/series-order/red.log` records the absent public API. The first
bidirectional view permutation attempt failed the custom-to-back state; retained
`order-first.log` shows [0,2,1] instead of [2,0,1]. It was corrected to count public
views before the target in the resulting model order, including support-slot cases.
Compile errors in initial test setup remain in `integration-first.log`; no existing
assertions, thresholds or tests were weakened to resolve them.

## Capture and source comparisons

All **31 source comparisons passed**; see
`target/series-order/oracles-final/results.json`. Capture evidence is in
`target/series-order/final/{protocol,results}.json`. Both feature groups passed
(**288 images**) and all **96 exact compatibility comparisons passed**. The matrix covers
three chart families, two asserted backends, three sizes, four DPRs and four scenes.
Old snapshots are rendered again after reordering, swapping, transfer, removal and
chart drop; all bytes must remain identical. The compatibility gate compares the
prior 96 images exactly. Source comparisons include the new browser ordering probe
and all 30 existing check_*_reference.cjs scripts without altered expectations.

Representative software initial calendar (800x500), reordered strike (300x300)
and transferred/reordered maturity (300x90) images were inspected. At very short
sizes content clips to the small panes as intended and recovers on enlargement.
A wgpu hovered calendar image at DPR 1.5 was also inspected and passed.
Initial/reordered and initial/hovered images are distinct in all three families.
The complete capture runner exited 0; see `target/series-order/captures.log`.

## Native mouse acceptance

| Backend | Calendar / strike / maturity | Result |
|---|---|---|
| tiny-skia | Ordering, hover, pane transfer/removal/reset, live navigation and box edits at three sizes, short-height recovery | Pending |
| wgpu | Same checks | Pending |

Physical acceptance is not inferred from raster captures. The user will be asked
to close each demo to record normal process exit after testing.

## Remaining dependencies and boundaries

Mouse-driven pane rearrangement, batch ordering, persistent layouts, events and
export remain separate work. No retained JS SeriesApi object or -1 sentinel is
introduced in Rust; typed IDs/errors provide the same useful live-owner operation.
Existing source/native pane-removal and formatter differences remain documented.
Historical timing exceptions and unavailable touchpad/touchscreen/high-DPI physical
checks remain open; simulated DPR coverage does not accept physical devices.

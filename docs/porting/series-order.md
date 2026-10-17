# Series drawing order port

Status: source contract frozen; implementation in progress (2026-10-17).
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

## Validation and remaining work

Baseline source identity verified against the previous accepted capture protocol.
The prior 96 compatibility images are copied into target/series-order/before.
The new 288-image matrix is frozen in target/series-order/baseline.json before
production changes. Automated, capture, demo and physical acceptance are pending.
Historical timing and unavailable-device checks are separate and remain open.

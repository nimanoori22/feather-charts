# Public events

Status: implementation in progress. Baseline `dd3f1c8`.

## Source and contract

Investigated the complete SeriesApi and TimeScaleApi, their ChartApi forwarding,
TimeScale lazy range calculation, TimeAxisWidget sizing, PaneWidget and
MouseEventHandler click/tap recognition, and SeriesMarkersPrimitive attachment.
The source is local Lightweight Charts 5.2.1. `public-events-source.json` pins the
compiled source and ambient constants; `public-events-states.json` records real
Chromium observations. Run `node scripts/check_public_events_reference.cjs`.

SeriesApi consumers include markers and expiring-price-alerts: a successful data
commit must precede notification; detach must stop observation. Historical loading
uses logical ranges and may mutate data in response. ChartApi converts pointer
parameters lazily, and TimeScaleApi converts strict indices to original items.
Existing native plugin invalidation remains synchronous and independent of drains.

Data replacements emit Full even when empty/equal; updates emit Update even when
equal; pop emits only when it removes fulfilled rows. Failures emit no data change.
Strict-range transitions emit horizontal before logical notifications, including
when clamping makes two horizontal payloads equal. Fractional-only changes emit
logical only. Hidden axis size is 0x0, measured in logical pixels.

## API and ownership decision

Use ChartEvent with DataChanged, DoubleClick, VisibleLogicalRangeChanged,
VisibleHorizontalRangeChanged and HorizontalAxisSizeChanged. EventInterests enables
each new family, default off. HorizontalRange reuses ValueRange. Enable/disable
does not synthesize an initial event or discard already queued events. The next
actual engine transition is observed. Initial successful layout is a size change.

Chart owns a lossless pending queue. `take_events` consumes it; `update` consumes
the same stream on every successful return. Errors preserve previously committed
notifications. Enabled applications must drain after each operation batch; there
is no silent cap or conflation of distinct commits. Payloads are owned, and data
events contain only historical SeriesId/scope. Host reactions create a later batch;
linked charts/history loaders must guard equality to avoid feedback loops.

Scale observation uses a typed internal delegate at existing strict/logical causes.
Chart retains its RAII subscription; the delegate captures only a weak queue.
No chart/model reference or closure cycle exists. Disabled interests remove the
subscription, so old applications perform no public-range payload cloning.

Alternatives: returning events from every setter breaks established signatures;
public callbacks require reentrant mutable ownership. Comparing only final endpoint
snapshots loses strict triggers and historical payloads. Typed internal observation
plus deferred owned application delivery fits existing synchronous ownership.

Deferred delivery intentionally does not reproduce JS handler identity, `this`,
manual destroy or arbitrary callback reentrancy. Engine signals keep operation order;
layout may emit intermediate model ranges while converging. Axis sizes publish
only the final successfully installed frame, never offscreen prepare_snapshot or
failed conversion. Snapshot preparation may produce model range events. Unchanged
redraws/immutable frame drawing produce no notifications. Pending range invalidations
use the engine's existing prepare-batch semantics.

Plot recognition follows source 500 ms and Manhattan distance <5 for mouse, <30
for touch. Successful first click/tap emits Click; second emits only DoubleClick;
third starts a new pair. No synthetic crosshair mutation on DoubleClick. Native
axis recognition retains its established Iced policy. Drag/leave/focus, edit,
axis/divider priority, owner revision and release outside cancel plot pairing.
The new interest gates pairing: disabled applications retain their existing
ordinary-click delivery. A distant successful click starts a fresh native pair;
the source instead suppresses a distant second release within its timer.

Delegate payloads no longer require `E: 'static`; callback storage still requires
`'static` captures. This permits a typed scale payload without imposing an unrelated
lifetime restriction on the generic engine's horizontal-item type.

## Validation and remaining work

The native/source data and range sequences pass. Focused native, Canvas cancellation,
custom ownership and demo controls pass. Full-source rerun, final backend captures
and physical mouse acceptance are pending. Earlier timing,
drawing-order and unavailable-device exceptions remain independent.

## Frozen acceptance matrix

Three modes (calendar, price, yield), four scenes (initial, data-change,
fractional, hidden), three chart sizes (800x500, 300x300, 300x90), DPR
1/1.25/1.5/2, and two asserted actual backends (tiny-skia/wgpu): 288 feature
images. Each scene asserts event scopes/counts or range/size causes and then
asserts no idle events. Retained frames must draw identically after mutations,
reorder, transfer, removal and owner drop. The runner also requires 96 unchanged
compatibility images from the previously accepted baseline; prior gates remain.

Initial all-target tests passed. The first source runner timed out at conflation
while its Cargo child waited behind concurrent build work; the 180-second limit
and checks were retained for an uncontended retry. Logs are retained under
`target/public-events`. Review added deterministic regression cases for plot pairing
cancellation, including release outside and divider hit priority.

A final cancellation-order regression reproduced `[range, logical, cancellation,
data]` when clearing a series during a captured box edit. Cancellation now enters
the same queue immediately, yielding `[cancellation, range, logical, data]`. This
also removes the old separate edit-event queue. A boolean retains the existing
animation wake-up condition until the consuming drain. The regression and original
edit/autoscale suites verify the corrected behavior.

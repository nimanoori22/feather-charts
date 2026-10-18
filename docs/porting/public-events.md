# Public events

Status: implemented and accepted for available mouse scope, 2026-10-18.
Baseline `dd3f1c8`; final code checkpoint `582c760`.

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

Source correspondence:

| Source component | Responsibility |
|---|---|
| `src/api/series-api.ts` | Successful public data commits and notification scopes |
| `src/api/time-scale-api.ts` | Logical/original-item ranges and logical axis dimensions |
| `src/api/chart-api.ts` | Lazy public pointer-event conversion |
| `src/model/time-scale.ts` | Separate strict/logical change causes and endpoint mapping |
| `src/gui/time-axis-widget.ts` | Logical dimension-change notification |
| `src/gui/pane-widget.ts`, `src/gui/mouse-event-handler.ts` | Plot click/double-click/tap recognition and forwarding |
| `src/plugins/series-markers/primitive.ts` | Plugin invalidation and attachment lifetime |

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
Ownership is `Chart → queue`, `Chart → model → delegate → closure → Weak(queue)`,
and `Chart → RAII subscription → Weak(delegate)`. Drop detaches the subscription;
queued owned payloads already taken by the host survive chart/series removal.
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

## Validation

The frozen final build passed:

- `cargo test --offline --all-targets`: 763 tests across 74 suites.
- `cargo test --offline --doc`: 2 doctests.
- `cargo check --offline --all-targets`.
- `cargo clippy --offline --all-targets -- -D warnings`.
- `cargo fmt --check` and `git diff --check`.
- All 34 source comparisons, including the 33 previous comparisons unchanged.
- 288 event-feature captures with asserted tiny-skia/wgpu backends and 96 exact
  compatibility-image comparisons. All final feature pixels also equal the earlier
  inspected capture matrix, supporting the unchanged mouse/rendering scope after
  the metadata-order and touch-only corrections.
- Software and GPU mouse acceptance in all three modes; all six processes exited 0.

[Machine-readable evidence](public-events-evidence.json) pins final logs/manifests,
counts, public-source identity and the reported physical results. Raw logs, retained
red regressions and capture matrices remain under `target/public-events/`; final
runs are `*-reviewed.log`, `oracles-reviewed/` and `reviewed/`. No check or threshold
was weakened. Earlier timing, drawing-order and unavailable-device exceptions
remain independently open.

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

## Files changed

- `README.md`
- `docs/porting/public-events-evidence.json`
- `docs/porting/public-events-baseline.json`
- `docs/porting/public-events-source.json`
- `docs/porting/public-events-states.json`
- `docs/porting/public-events.md`
- `examples/options_chart.rs`
- `examples/pane_management.rs`
- `examples/pane_management/captures.rs`
- `examples/pane_management/events.rs`
- `examples/pane_management/host.rs`
- `examples/public_chart.rs`
- `examples/support/chart_demo.rs`
- `examples/yield_curve.rs`
- `scripts/capture_public_events.py`
- `scripts/capture_series_order.py`
- `scripts/check_public_events_reference.cjs`
- `src/api/chart.rs`
- `src/api/click_sequence.rs`
- `src/api/custom_series.rs`
- `src/api/events.rs`
- `src/api/interaction.rs`
- `src/api/mod.rs`
- `src/api/primitive_editing.rs`
- `src/api/series_data.rs`
- `src/api/touch.rs`
- `src/api/touch_input.rs`
- `src/api/widget.rs`
- `src/api/yield_curve.rs`
- `src/helpers/delegate.rs`
- `src/model/time_scale.rs`
- `tasks/public-events-plan.md`
- `tasks/public-events-todo.md`
- `tests/primitive_editing.rs`
- `tests/public_events.rs`
- `tests/series_markers.rs`

## Behavioral test coverage

The public-events integration suite covers full/update scopes, empty/equal and
rejected commits, custom data without drain-time payload copies, all three native
frontends, public identity after removal/drop, up/down wrapper deduplication,
1,000-update drain/default-disabled batches, host reactions, the animation fast
return, strict versus fractional ranges, original calendar strings, snapshot-only
preparation, hidden/restored/failed axis preparation, and Canvas double-click
payloads/cancellation. Deterministic recognizer and touch tests use supplied time
without sleeping. Primitive-edit tests cover cancellation ordering, the edit early
return and preservation through failed preparation. Generic demo tests cover
exactly-once consumption, interest changes, Clear, Reset and bounded history.

The implementation deliberately keeps existing source correspondence, synchronous
plugin/model invalidation and native axis behavior. No dependencies were added.
No further source component is a prerequisite for these event families.
Screenshot/export and attribution drawing remain separate feature work. Earlier
timing, drawing-order and unavailable-device checks remain independently open.

## Physical mouse acceptance

Software: user reported “All checks passed in all three modes; idle counters
passed; windows closed.” Calendar, strike and maturity processes each exited 0.
The run covered event controls, typed readouts, data scopes, double-click/axis
priority, idle counters, live-feed interactions at three sizes and short-height
recovery. It used the final gesture/rendering code; the later metadata cancellation
ordering fix is verified by the targeted regression and final automated matrix.
GPU: user reported “All checks passed in all three modes; idle counters passed;
windows closed.” All three explicit `ICED_BACKEND=wgpu` processes exited 0.
The same controls, gestures, size changes and idle checks passed.
Touchpad, touchscreen and high-DPI physical devices remain untested independently;
fractional-DPR automated captures do not replace those physical checks.

The final touch lifecycle audit reproduced a stale double-tap pair after adding a
pane between taps. The Canvas adapter now clears only the saved tap pair on
revision, geometry, owner, focus, epoch or interest invalidation, independently
of whether a finger is currently down. The red/green Canvas regression retains
this case. Mouse recognition/rendering is unchanged by this touch-only correction;
physical touch devices remain outside accepted scope.

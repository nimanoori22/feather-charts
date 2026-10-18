# Additional public chart events

Date: 2026-10-18. Status: implemented and accepted for available mouse scope.
Core API `f17b20d`; demo/order `bc2fcbf`; touch lifecycle `582c760`.
Baseline: `dd3f1c8`. Checklist: [public-events-todo.md](public-events-todo.md).
Use the existing feature-specific planning convention. Earlier plans and their
independent acceptance records remain intact.

## Outcome and scope

Applications can react to series data changes, plot double-clicks, visible
horizontal ranges and horizontal-axis dimensions through the reusable component.
Calendar, strike/options and maturity/yield charts share the feature. Applications
can update an external readout, request more history, or align another widget
without polling the complete data set or replacing chart ownership.

Deliver these event families:

| Event | Information |
|---|---|
| Series data changed | Public SeriesId and full-replacement/update scope |
| Plot double-click | Existing typed PointerEvent, including pane, logical/horizontal item and available hit/custom data |
| Visible logical range changed | Optional fractional logical endpoints |
| Visible horizontal range changed | Optional original calendar, strike or maturity endpoints |
| Horizontal-axis size changed | Width and height in logical pixels; hidden axis reports zero dimensions |

Existing Click, CrosshairMoved and PrimitiveEdit behavior remains supported.
Screenshot/export, attribution drawing, additional price-range subscriptions,
network history loading, mouse-driven pane rearrangement and a general-purpose
event bus are separate work. No new dependency, async channel, Tokio runtime or
thread-safe chart ownership is planned. Historical timing, earlier drawing-order
acceptance and unavailable-device checks remain open independently.

## Preliminary source and native findings

Graph discovery/traces, source snippets and direct reads establish the starting
contract below; Task 1 completes the source/consumer audit before production edits.

- Source `src/api/series-api.ts`: successful setData emits `full`, successful
  update emits `update`, and pop emits `update` only when rows were removed. Thus
  setData with equal/empty data and update with an equal value are not automatically
  suppressed as no-ops. Rejected operations must not report a committed data change.
- Source `src/api/time-scale-api.ts` was read completely. It subscribes to engine
  visible-bars/logical-range delegates and GUI time-axis size changes. Public
  horizontal endpoints come from original horizontal items, not formatted labels.
- Source `src/model/time-scale.ts` compares strict visible ranges separately from
  fractional logical ranges, firing visible-bars before logical-range notifications.
  Endpoint conversion clamps through the source's own range conversion. Event
  triggers are not equivalent to simply comparing final public endpoint payloads.
- Source `src/gui/time-axis-widget.ts` fires size changes when its logical Size
  changes. `hidden-time-scale-size.js` requires hidden width and height, including
  event values, to be zero. This is an axis-size event, not a window-size event.
- Source `ChartApi` forwards double-clicks from PaneWidget using its existing mouse
  parameter conversion. `mouse/double-click-pane.js` expects one Click and one
  DoubleClick for a double-click sequence. Pane doubleTap also routes to this
  event family; axis double-click actions are separate.
- Real data-event consumers include series markers, expiring-price-alert examples
  and primitive autoscaling examples. They require updated series state before
  notification and correct attachment cleanup. History-loading examples use logical
  ranges to query bars before/after, so duplicate/lost notifications can feed loops.
- Rust ChartEvent currently contains Click/CrosshairMoved/PrimitiveEdit, delivered
  by `Chart::update`. Setters return their existing result types; preparation can
  also change ranges and dimensions. `update` has animation/edit fast returns and
  drains pending edit cancellations; new delivery must cover those paths.
- Rust TimeScale already has visible-bars and logical-range delegates with RAII
  Subscription handles. Reuse those semantics; do not introduce another scale.
- The current Canvas adapter recognizes successful clicks on release and cancels
  them after drag/leave/focus changes. PriceAxisInput already uses Iced mouse::Click.
  Exact Iced Click recognizes consecutive clicks within 300 ms and distance <6;
  source MouseEventHandler uses its own rules. Preserve established native axis
  behavior and document any deliberate native timing difference for plot input.
- YieldCurveChart settles its private scaffold before update/prepare and explicitly
  forwards mutations. Internal support data must not masquerade as public series
  changes. Shared index changes must not create false data events for peer series.

Coverage checks reported metadata matches/no recorded gaps for the cited native
files and most source files. Source time-scale metadata differed; the actual
range conversion and notification implementation was read directly. Rust graph
call edges for Chart::update were sparse, so its real body and early returns were
inspected. This is bounded planning evidence, not a complete event parity claim.

## Delivery design and ownership

Preferred design: extend the typed ChartEvent family and use chart-owned pending
events with an explicit consuming method. Keep the existing update return type;
it drains the same stream once after processing its message. Programmatic callers
drain after their setters and successful preparation. Applications handle events
after the chart borrow ends and can then mutate it safely.

Provisional shape, to freeze in Task 1:

```rust
pub enum DataChangeScope { Full, Update }

// New variants of existing ChartEvent<T>:
// DataChanged { series: SeriesId, scope: DataChangeScope }
// DoubleClick(PointerEvent<T>)
// VisibleLogicalRangeChanged(Option<LogicalRange>)
// VisibleHorizontalRangeChanged(Option<HorizontalRange<T>>)
// HorizontalAxisSizeChanged { width: f64, height: f64 }

pub struct HorizontalRange<T> { pub from: T, pub to: T }

impl<B: ChartBehavior> Chart<B> {
    pub fn set_event_interests(&mut self, interests: EventInterests);
    pub fn event_interests(&self) -> EventInterests;
    pub fn take_events(&mut self) -> Vec<ChartEvent<B::Item>>;
    // Existing update(...) -> Result<Vec<ChartEvent<B::Item>>, ChartError> stays.
}
```

EventInterests enables the new event families explicitly, with new notifications
off by default; existing input/edit notifications retain their default behavior.
This mirrors source listener-aware work and prevents old applications accumulating
data notifications they never consume. Payload names and the range wrapper must
reuse existing types where they fit; they are not frozen solely for resemblance
to TypeScript. Document enabling/disabling, initial notification policy and whether
pending selected events remain deliverable after interest changes.

With queued delivery, an enabled consumer must drain each application update/batch.
Do not silently cap/drop events or merge distinct data mutations to conceal an
undrained queue. Do not retain data arrays, typed series handles, primitives or
frames in data-change events: store IDs/scope only. Test sustained feed with regular
draining and default-disabled operation. Record the undrained-queue contract.

| Alternative | Tradeoff | Decision |
|---|---|---|
| Owned pending events plus consuming drain | Fits current Vec return model; supports programmatic setters and safe host reactions | Preferred |
| Per-operation event return values | Explicit, but changes many existing setter/prepare signatures and complicates composition | Reject for this milestone |
| Public synchronous callback subscriptions | Source-like timing and natural unused-listener behavior, but introduces chart reentrancy and callback ownership concerns | Consider only if real consumers require immediate callbacks |

Chart owns pending events and any internal observation state. Pending payloads are
owned values with historical IDs, so later removal/drop cannot invalidate references.
If existing scale delegate subscriptions are used, retain RAII tokens and capture
only minimal signal state, never a strong chart/model owner. Release signal borrows
before reading model state or dispatching. Boundary snapshots/change generations
are another viable capture mechanism; choose after proving trigger parity and cost.
No callbacks into host code run while mutating the engine.

This is deliberate deferred Rust delivery, not a promise to reproduce arbitrary JS
callback reentrancy. Native markers/plugins retain synchronous engine invalidation;
their correctness must not depend on an application draining notifications.

## Behavioral contract to freeze

1. Exactly-once delivery across setters, prepare and update. Taking events twice
   yields an empty second result; update does not replay events already taken.
   Invalid operations add no event and preserve previously queued valid events.
2. Data events follow successful public commits. Preserve full versus update scope,
   empty/equal setData, equal updates, historical updates, whitespace updates and
   nonempty/zero/empty pop semantics. Emit only for the addressed public owner,
   including typed custom/up-down wrapper paths, never normalized peers or scaffolds.
3. Separate fractional logical transitions from strict visible-bar triggers and
   original-item payload mapping. Handle empty/None transitions, gaps, right offset,
   fit-content, data changes/reindexing, conflation, animation and whitespace policy.
   Do not suppress a source-required trigger solely because public endpoints compare
   equal. Conversely, idle redraw/identical layout must not create repeated events.
4. Capture public-operation order; do not coalesce separate successful data calls.
   Probe source ordering between data, logical range, horizontal range, size and
   pointer notifications. State any native transaction-boundary differences.
   Internal layout convergence can publish a settled observation once, with the
   permitted policy documented rather than silently losing transitions.
5. Horizontal-axis dimensions describe final logical geometry, excluding price
   axes/corner stubs and DPR multiplication. Hidden is 0x0; restoring visibility,
   font/label width changes, price-axis visibility and window resizing can change it.
   Emit after successful settled preparation, not intermediate measurement passes.
6. Resolve `prepare_snapshot` explicitly: it can mutate model dimensions without
   installing a displayed frame. Prefer size notifications for installed UI frames;
   distinguish model-range observations from displayed-size observations. Failed
   preparation must not publish a successful new size or discard committed data
   events. Drawing an immutable old frame emits nothing.
7. Double-click uses successful same-pane plot clicks with correct pointer data.
   Preserve first-click delivery and replace the second ordinary click with the
   double event as demonstrated by source. Test triple clicks, timeout/distance,
   button changes, release outside, drag, focus loss, pane removal/reorder and input
   revisions. Primitive edits and axis/divider gestures keep existing priority.
8. Double-click notification does not force a synthetic crosshair change or reset
   price autoscale. Existing axis double-click reset stays independent. Audit the
   existing touch tap path and preserve source double-tap delivery where applicable;
   no new zoom gesture or platform browser workaround is implied.
9. Host reactions occur after taking owned events. An event handler that changes
   range/data creates a later event batch, without recursion, lost events or an
   automatic synchronization loop. Document guards for linked-chart/history loading.
10. Keep work proportional to subscribed event metadata. No full-data cloning per
    update, extra frame preparation to obtain payloads, or per-bar event payloads.
    Preserve pulse/expiry fast paths and existing interaction smoothness.

## Ordered implementation slices

Each slice includes focused behavioral tests and a local checkpoint. Split a slice
if it grows beyond about five files; all branches of the contract remain in scope.

### 1. Freeze source observations and API — no dependencies; medium

Complete relevant source/API/dependency reads and real consumer traces, including
MouseEventHandler, scale lazy notifications, size layout and plugin lifecycles.
Run actual local 5.2.1 probes for data scopes/count/order, range triggers/payloads,
double-click sequence and size changes. Freeze hashes/results, initial notification
policy, queue/interest lifecycle, snapshot/error semantics and native timing policy.

Likely files: new `docs/porting/public-events.md`, source/state JSON and
`scripts/check_public_events_reference.cjs`, plan/checklist. Verification: source
probe passes before production edits; old compatibility baseline is pinned. No
further user design input is required unless real consumers contradict the design.

### 2. Typed delivery and consumption — depends on 1; medium

Add event types/interests and the one consuming event path, preserving update's
signature and existing edit/pointer order. Update repository exhaustive enum matches
and document the public API migration; split those consumers if needed. Cover
default-disabled, enabling/disabling, pending drain and all early/error returns.

Likely files: new `src/api/events.rs`, `mod.rs`, chart/interaction ownership,
new `tests/public_events.rs`; a separate caller-migration slice if broad.
Verification: behavioral tests prove no duplicate/lost pending events, including
animation-only tick, edit return and invalid message; existing consumers build.

### 3. Built-in data notifications — depends on 2; medium

Emit source scopes after set/update/pop commit, preserving existing return values.
Integrate up/down wrappers without double emission. Data queries/options/pane/order
operations do not pretend to be data mutations. Include equal/empty/whitespace,
historical/conflation rejection and shared-index changes.

Likely files: chart setters, `series_data.rs`, up/down wrapper and public event
tests. Verification: exact native/source data sequences; zero notifications on
rejection or empty pop; no peer notifications or data-array cloning.

### 4. Typed custom data and frontends — depends on 3; medium

Use the same scope/commit rules in custom operations and explicit yield forwarding.
Preserve removed/foreign handle errors, pane transfer, empty owners and scaffold
settling. Native marker/plugin updates remain independent from public consumption.

Likely files: `custom_series.rs`, yield custom/forwarding modules, event tests.
Verification: same sequences in calendar/strike/maturity with custom and built-in
owners; no private support IDs or duplicate wrapper events; drop/read lifetimes pass.

Checkpoint A: review event API, defaults, queue ownership and source counts/order;
save tested local checkpoints before adding range/input delivery.

### 5. Double-click recognition and payloads — depends on 2; medium

Extend Canvas click recognition and validated ChartMessage routing; reuse existing
pointer conversion. Use exact Iced/source behavior investigation to select the
smallest recognizer, with injected-time testing where needed. Respect axis/edit/
divider priority and touch tap semantics; do not add another frame per click.

Likely files: widget or focused input helper, interaction validation/conversion,
touch input only if needed, native input tests. Verification: one Click then one
DoubleClick with correct payload; negative/cancellation/triple-click cases and
axis autoscale reset regressions pass without sleeping-based timing tests.

### 6. Logical and original-item range events — depends on 2 and 4; medium

Bridge existing scale signals or minimal change tracking to owned public payloads
at the frozen boundaries. Preserve strict/logical distinctions and original items;
include setters, data remapping, prepare and kinetic/zoom ticks. Exclude hidden yield
implementation details without omitting real public domain changes.

Likely files: event collection module, existing scale/model exposure only if needed,
chart/interaction boundaries and public event tests. Verification: fractional moves
within the same strict range, boundary crossings, empty/data changes, all chart
families and no-change redraws match the frozen trigger contract.

### 7. Settled horizontal-axis dimensions — depends on 6; medium

Observe successful final layout with axis visibility projection, including hidden
0x0. Distinguish displayed frames from offscreen snapshot preparation and failures.
Avoid repeated events from axis measurement convergence or unchanged DPR-only size.

Likely files: chart prepare/event collector, frame owner exposure if required,
yield forwarding and layout/event tests. Verification: resize/font/axis-visibility,
empty/short recovery, failed prepare, snapshot and repeated draw cases have the
specified logical sizes/counts; existing immutable frames remain unchanged.

Checkpoint B: review ordering, enabled work, no duplicate collection, frame/error
boundaries and input priority; save tested local checkpoints.

### 8. Host reaction and lifecycle regressions — depends on 3–7; medium

Exercise drained-event reactions that request history or change a linked range,
without networking or a new demo architecture. Verify batches remain separate,
identity survives moves, removed-series historical events have safe payloads and
private scaffolds stay private. Test a long live-feed sequence with regular drains
and disabled interests without growing queues or copying all rows.

Likely files: public event, pane interaction and primitive editing tests, plus
existing family tests as needed. Verification: meaningful ordering/lifetime/load
assertions, unchanged overlay/edit/animation tests and no reentrant borrow failures.

### 9. Demo controls and readouts — depends on 8; medium

Extend the existing generic host with opt-in event monitoring: family toggles,
clear bounded display history, data/change counters, latest double-click details,
logical/original ranges and axis dimensions. Use both direct controls and Canvas
input so setter→prepare→take and update-return consumption are exercised without
double draining. Keep logging off by default and avoid terminal output per frame.

Likely files: `examples/pane_management.rs`, new events submodule, host/capture
submodules and README. Verification: generic controls test all three frontends,
clear/disable/reset, pop/append and range/visibility/resize; native builds succeed.
The demo's bounded display history is separate from lossless API consumption.

### 10. Source, raster and mouse acceptance — depends on 9; medium

Freeze deterministic event sequences and captures before replay: data replace/
update/pop, double-click, fractional/strict range changes and visible/hidden/resized
axis states. Reuse capture tooling across calendar/strike/maturity, asserted
tiny-skia/wgpu backends, regular/narrow/short sizes and DPR 1/1.25/1.5/2. Record
exact counts before final runs. Pass the 33 existing source comparisons unchanged
alongside the new oracle; keep unchanged compatibility images and prior thresholds.

Verification: inspect representative frames and validate event payloads/count/order,
not just pixels. Ask the user to test each backend in all modes: double-click plot
and price axis, live data controls, range/size readouts, zoom/pan/axis and box edits,
resize/short recovery and idle event counters. Closing windows must exit normally.
Only available mouse scope is accepted; unavailable devices remain untested.

### 11. Final validation and report — depends on 10; small

Run `cargo fmt --check`, `cargo test --all-targets`, `cargo test --doc`,
`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings` and
`git diff --check`. Recheck important source/plugin/widget consumers. Document
source components, behavior, consumer contract, discarded JS patterns, ownership,
alternatives, exact changed files, tests/evidence and remaining dependencies.
Save local commits and a clean working tree; push only when requested.

## Risks and completion gates

| Risk | Mitigation |
|---|---|
| Events only emerge on the next mouse input | Explicit drain plus setter/prepare boundaries; test with no input/ticks |
| Fast paths discard pending notifications | Test every animation/edit/error return and unify consumption |
| Repeated layouts create storms or wrong ordering | Freeze source triggers; observe settled dimensions and retain operation order |
| Final payload equality hides strict-range triggers | Track strict/logical causes separately; test equal endpoint payloads |
| Two ordinary clicks leak from double-click | Match source one-click/one-double sequence; same-pane and cancellation tests |
| Default event work slows live feed | New interests off by default; metadata-only data events and steady drain tests |
| Yield scaffold leaks events | Route public commit identities separately from internal support updates |
| Handlers cause recursion/synchronization loops | Deferred owned batches; application equality guards and reaction tests |

Complete means all five event variants reach real applications in all three chart
families, source-required counts/payloads and documented native boundaries hold,
existing events remain supported, compatibility gates pass unchanged, and available
software/GPU mouse acceptance passes. Planning does not certify implementation or
close any earlier exception. No tool installation is anticipated.

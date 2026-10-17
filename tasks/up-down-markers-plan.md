# Up/down update markers implementation plan

Date: 2026-10-17. Status: implemented; automated and software/GPU mouse acceptance pass.
Starting checkpoint: `16f3622` (image watermarks complete).
Execution checklist: [up-down-markers-todo.md](up-down-markers-todo.md).

## Outcome and scope

Port the built-in `createUpDownMarkers` behavior for Line and Area series:
tracked value updates produce a colored point/chevron, neutral updates use the
series line color, automatic markers expire, and manually supplied markers can
be inspected, replaced and cleared. Keep canonical data mutation synchronous,
old frames immutable, and existing normal marker collections independent.

Use the current date/time Chart API. The upstream yield-curve demo supplies useful
historical-update values, but adding a numeric/yield-curve chart is a separate
milestone. Baseline/OHLC/custom series, general plugin wrappers, sound/notifications,
fade effects, a new runtime and new renderer dependency patches are outside scope.
Historical timing acceptance stays OPEN/deferred; unavailable physical devices
remain untested. Do not overwrite older plans. Use this repository's feature-specific
plan/checklist convention. No delegation or additional approval gates are required.

## Investigated source and consumers

The seven complete files under upstream `src/plugins/up-down-markers-plugin/`
were read: options, types, wrapper, primitive, expiring-markers-manager, view and
renderer. The wrapper routes setData/update to the primitive, rather than observing
all series mutations. The primitive seeds a time-key/value map on setData and
manages one marker per horizontal key. The manager replaces same-key markers,
keeps insertion order and uses browser timers. The view converts each marker's
explicit time/value through its owning series/time scale. The renderer uses bitmap
coordinates, a filled circle and an open chevron; it is not a regular annotation
arrow. There are no hit-test, axis-label or autoscale hooks. No zOrder override
means normal series primitive ordering.

Graph tracing of createUpDownMarkers identifies the yield-curve tutorial. Direct
reads also cover graphics and coverage fixtures: set initial curve data, then
historically update three distinct points to positive, neutral and negative values.
The tutorial repeats historical shifts relative to its initial curve values.
Coverage metadata matches the primary files/tutorial/graphics fixture; the coverage
subtree is excluded and was read directly. No broad completeness claim is made
from graph edges. Pin source revision and working-tree hashes before implementation.

## Early contract checkpoint: actual behavior versus incidental JS details

An actual-source oracle with fake Date/window timers must settle these before
production edits. Start the fake wall clock at a nonzero epoch, not Date.now()==0.

| Finding in pinned source | Proposed native contract to verify and document |
| --- | --- |
| setData seeds managed values; update does not advance that map or register appended keys | Preserve the initial/full-reset comparison baseline for tracked keys; an append alone does not silently become a tracked baseline. Repeated changes must not accidentally compare with the preceding update. |
| `if (existingPrice)` skips a baseline of zero | Treat presence explicitly with Option; valid zero/negative values are supported. Record this deliberate rejection of JS truthiness, rather than claiming exact output parity for that bug. |
| Duration defaults to 5000 ms; code passes zero to setTimeout, although the comment promises indefinite display | Preserve executed zero-duration expiration (due immediately), unless further consumer evidence changes the contract. Manual markers without a timeout remain indefinite. Clearly document the discrepancy. |
| Same-key replacement deletes then reinserts | Preserve one active marker per key and refreshed insertion/draw order. Do not silently substitute a key-sorted map for observable order. |
| setData replaces the comparison baseline but does not clear active markers | Preserve baseline reset separately from marker clearing. Handle now-unresolvable marker times safely. |
| applyOptions replaces an object while the attached view retains the previous options object | Use owned native options with effective color updates; discard stale JS object-sharing behavior. Separate source-parity fixtures from this intentional native correction. |
| Source changes markers before series.update can reject data | Follow the existing Rust data API's transactional boundary: rejected updates must not change markers, baselines or deadlines. |

Do not assume source comments, intended trading semantics and executed code are
identical. Tag fixture expectations as parity, native validation or deliberate
JS-specific correction. Recheck imports, wrapper/adapter lifecycle, all relevant
consumers and primitive draw order before implementation. These choices are bounded
implementation decisions to document, not reasons to introduce user approval flows.

## Required rendering and lifecycle behavior

- Defaults: positive #22AB94, negative #F7525F, lifetime five seconds. Neutral color
  comes from the owning Line color or Area lineColor; ordinary series option changes
  must refresh it. Color options are parsed/validated before mutation.
- Circle radius is 4 * vertical DPR plus half-pixel correction. Correction comes
  from max(1,floor(horizontal DPR)) parity. Circle center x is rounded in bitmap
  coordinates; preserve the different, unrounded x used by chevron vertices.
- Chevron half-width 4.7, offset 7, vertical scale .5; stroke width is
  floor(2 * horizontal DPR). Positive/negative signs control vertical placement;
  neutral points draw no chevron. Test fractional DPR, boundaries and clipping.
- The owner is a series; placement uses explicit marker values, not conflated
  aggregate rows or nearest visible values. Existing normal primitive ordering,
  pane membership and series visibility must remain intact.
- Manual markers are independent of baseline data tracking. Resolve duplicate,
  unordered and missing horizontal keys explicitly; absence must not panic through
  the source ensureNotNull path. Reject nonfinite prices/invalid times safely.
- Clearing/removing a controller, removing its series or dropping Chart cancels
  its future expiry work. Stale deadlines cannot clear refreshed markers. Retained
  prepared frames remain frozen through expiration, options updates and removal.
- Direct ordinary Chart data APIs remain usable. As in the source, marker-aware
  commands are explicit; do not intercept every data write or change existing
  users' update behavior. Document how mixing the two paths affects tracked state.

## Rust design, alternatives and provisional API

Preferred: an opt-in Chart-owned controller bound to one supported series. Reuse
existing primitive ownership/attachment order, with a typed controller ID backed by
an existing PrimitiveId rather than a second independent ID allocator. Expose
explicit marker-aware data operations. Store only the required baseline time/value
snapshot, active marker order and optional deadlines, not another full DataLayer.
On changes, publish immutable primitive drawing input through the existing path.
Keep reference ownership and cleanup auditable; avoid back-references to Chart.

Alternative: a host-owned UpDownMarkers controller which owns a primitive handle
and receives every update/expiry call from the application. This reduces Chart
state but makes idle expiration, removal and regular host data routing easy to
miss. Prefer Chart ownership if it can reuse the existing Tick/widget path without
polling or cyclic shared state. Do not introduce a generic plugin controller registry.
Confirm the smallest private state-sharing arrangement after tracing data/cleanup.

Provisional surface, subject to the contract checkpoint:

```rust
pub enum MarkerSign { Negative, Neutral, Positive }
pub struct UpDownMarker { pub time: Time, pub value: f64, pub sign: MarkerSign }
pub struct UpDownMarkersOptions {
    pub positive_color: String,
    pub negative_color: String,
    pub update_visibility_duration: Duration,
}
pub struct UpDownMarkersId(PrimitiveId); // opaque, typed, no extra ID allocator
// Chart operations: add controller; set managed data; update managed data(now);
// inspect active markers(now); set manual markers; clear; replace options; remove.
```

Data commands use existing BuiltInSeriesDataItem<Time>, series validation and the
historical flag; unsupported series return a typed error before attachment. Full
owned options replacement matches established Rust APIs. Preserve untouched options
by editing a host-owned clone. Reject deadline arithmetic overflow before data or
marker mutation. Use supplied Instant values for deterministic update/expiry tests;
never sample wall time in drawing. Decide and document option-lifetime changes for
already-existing markers against the source (new duration affects new updates).

```text
Chart -> controller (series ID, baseline map, active markers, deadlines)
Chart -> existing series primitive attachment -> immutable marker input
Prepared frame -> frozen renderer -> owned coordinates/colors
```

No timer closure captures owners. No Rc cycles, Arc<Mutex<_>>, threads or engine
subscriptions. Generic remove_primitive and remove_series must clean controller
state as well as the dedicated remove method. Old frames retain only old drawing
payloads, not controller/baseline state. Avoid per-update history scans; earliest
expiry can initially be found among active markers without an unbounded stale heap.

## Expiration integration: correctness and idle work

Use monotonic Instant/Duration and deterministic advance(now) semantics. Native
headless callers can feed Tick/advance using an explicit deadline. The widget
should request the earliest needed redraw across marker expiry, touch deadlines
and active animation. A marker waiting five seconds must not keep is_animating
true and schedule a 16 ms loop merely to expire once.

Existing integration has two critical gates:

1. Widget RedrawRequested currently publishes Tick only for animation; idle touch
   deadlines are separate. Add marker deadlines to that existing scheduling logic
   and publish a Tick when due, including when no price pulse/live feed is active.
2. Chart::update currently returns early through sample_price_animation_tick.
   Expiry processing must happen before that fast path, or explicitly invalidate
   it when markers are due. Pulse-only ticks before a deadline must retain their
   current cheap path; expiry must not be swallowed by it.

Check exact-deadline boundaries, early redraws, late ticks, replacement before old
expiry, empty time scales, hidden windows/series and cleanup. Cancellation must leave
no stale request-redraw loop. Never use sleep-based unit tests or browser timer IDs
in engine state. The existing software single-geometry damage boundary stays intact.

## Delivery sequence

0. Freeze source/consumer evidence, resolve the contract table, define typed API,
   ownership and clock rules. Record an actual-source timer/update/renderer oracle
   and pre-feature default-off captures. Freeze fixture counts before execution.
1. Deliver manually set positive/neutral/negative markers through a real public
   attachment. First record a compiling visible-output RED, then match circle/
   chevron geometry, neutral color and normal-series ordering on both backends.
   **Checkpoint A:** visible renderer and source layout oracle pass.
2. Add marker-aware setData/update with baseline tracking, latest and historical
   updates, whitespace and transactional rejection. Verify repeated baseline-relative
   changes, zero/negative baselines, appends and direct API bypass explicitly.
3. Add expiration/deadline scheduling with fake-clock tests and widget redraw tests.
   Manual markers persist; default automatic markers expire while idle; refresh and
   removal cancel obsolete work. Verify coexistence with the price-animation fast path.
   **Checkpoint B:** data/expiry contract passes and an idle chart stops redrawing.
4. Close lifecycle, options, mixed rendering and work bounds: multiple controllers,
   series/pane removal or movement, hidden series, clear/full reset, retained frames,
   conflation, ordinary annotations, watermarks, price pulse and editable primitives.
5. Add --up-down-markers to public_chart, with deterministic latest/historical
   update and manual/clear/lifetime/color controls. Include a pause-live mode to
   observe expiry independently of feed/pulse. Test Line and Area, not OHLC/Baseline.
   **Checkpoint C:** integrated demo and bounded update/expiry work pass.
6. Capture 2 backends x DPR 1/1.25/1.5/2 x sizes 800x500/300x300/300x90 x 5 scenes
   (directions, historical/manual, refreshed deadline, mixed content, expired/cleared)
   = 120 feature images; compare 24 default-off images. Freeze time/state for captures,
   draw retained frames twice, retain failures and inspect representative images.
   Source oracle covers exact timeline boundaries independently of screenshots.
7. Run full validation, review and publish a port report. Record native software/GPU
   mouse checks at regular/narrow/problem sizes, short-height recovery, idle expiration
   and coexistence. Report source corrections and any device gaps explicitly.
   **Checkpoint D:** source/native gates pass; old timing/device exceptions unchanged.

Tasks/dependencies/likely files and verification are in the checklist. Split shared
Chart state, data updates and widget scheduling into sequential small slices; no
large renderer/dependency refactor or new performance benchmark matrix is needed.

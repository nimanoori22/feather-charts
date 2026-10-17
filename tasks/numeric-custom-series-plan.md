# Custom series on numeric horizontal axes

Status: implemented; automated verification and software/GPU mouse acceptance passed
on 2026-10-17. Baseline: `2a32573`.
Tasks: [numeric-custom-series-todo.md](numeric-custom-series-todo.md).
These feature-specific documents preserve the existing plans and acceptance records.

## Outcome and boundaries

Extend the existing custom-series API to OptionsChart (HorizontalPrice) first and
YieldCurveChart (Maturity) second. Applications can supply custom range/distribution
renderers against strikes or maturities, with the existing shared engine, widget,
price scales, pointer events and conflation. Calendar custom callers retain their
current source syntax and behavior.

Include custom construction, typed handles/data retrieval, replace/latest/history/
whitespace updates, options, autoscale/current value, hit payloads, original item
identity, mixed built-in/custom composition, lifecycle and frozen frames. Include
numeric conflation and yield scaffold settlement; neither is an optional follow-up.

Exclude new option-pricing models, feeds, arbitrary horizontal-behavior factories,
a new renderer/plugin framework, non-unit common metadata and new custom-specific
pulse/up/down-marker features. Preserve annotations/primitives already supported by
custom series; demonstrate coexistence with supported pulse/update-marker built-in
peers. Keep runtime paths synchronous. No new dependencies are expected.

## Investigation and source correspondence

Upstream source anchors:

- src/api/chart-api.ts: addCustomSeries creates a custom definition and calls
  _addSeriesImpl directly; shared registration, defaults and lifecycle follow.
- src/model/{icustom-series.ts,series/custom-series.ts}: generic original items,
  logical bar indices, application payloads/options, value builder, whitespace,
  renderer/hit callbacks, optional reducer and destruction contract.
- src/api/yield-chart-api.ts: inherited addCustomSeries bypasses the overridden
  addSeries restriction/default overrides. The Line/Area override must not be
  copied onto custom additions. Chart-wide yield localization remains relevant.
- src/model/{data-layer.ts,data-conflater.ts,series/custom-pane-view.ts}: union
  membership, original/internal items, prepared renderer input, reducer contexts,
  cached dense representations and current-value/price-coordinate behavior.
- tests/type-checks/non-time-based-custom-series.ts: explicit numeric custom data,
  typed options, whitespace and rejection of calendar/built-in data mismatches.
- Existing plugin graphics/interaction/coverage custom-series cases: rendering,
  mouse parameters, price-line/fallback hits and conflation. Read their actual
  consumers and fixtures at the source-contract checkpoint.

The real factory/API path was traced with the graph and verified by reading the
implementation. Graph coverage matched the inspected Rust API/model/UI paths and
upstream API/definition/conflater files. Upstream icustom-series.ts was marked
changed; its live contract was read directly. Before coding, check further evidence
paths, read gaps and pin actual source working-tree hashes, not revision alone.

The Rust lower layers already support generic items:

| Layer | Existing reuse | Work required |
| --- | --- | --- |
| CustomSeriesPaneView and CustomConflationContext | Associated Item/InternalTime and typed payload/options already exist | Preserve trait compatibility; avoid a replacement trait hierarchy. |
| ChartModel/DataLayer/custom rows | Generic behavior item/internal types and typed custom registration | Map behavior-specific errors and perform family validation before mutation. |
| UI custom preparation | Logical coordinates, price transform and erased frozen renderer; no calendar payload in the adapter interface | Keep this boundary erased; generalize the owning adapter behind it. |
| src/api/custom_series.rs | Weak view handles, shared model ownership, validation, event payload downcasts | Remove Time/TimePoint restrictions from handle, adapter, mutators and PointerEvent accessors. |
| src/api/custom_series/conflation.rs | Budgeted cache/jobs and typed reducer callbacks | Parameterize row storage/context by actual item/internal types; avoid new Copy constraints. |
| OptionsChart | Alias of Chart<price behavior> | Inherit generalized operations; no extra forwarding wrapper. |
| YieldCurveChart | Private generic core, dirty flag, bounded scaffold and readonly Deref | Explicit custom forwarding/preflight and settlement; no DerefMut escape. |

## Behavioral contract

1. Keep horizontal types distinct: original_data.time and event data contain
   HorizontalPrice or Maturity; CustomBarItemData.time remains a logical index.
   Renderer x comes from the shared index scale, never directly from strike value.
   Calendar conversion/normalization remains unchanged.
2. Preserve custom price-value-builder semantics, including last-value selection,
   autoscale extrema, source-derived empty/invalid handling and optional coordinate
   conversion before layout. Keep user payload/options typed and view-defined
   whitespace distinct from fulfilled membership.
3. All writes validate before canonical mutation. Wrong/detached/foreign handles,
   unordered/duplicate data, bad prices/options and unsupported historical
   conflation must preserve data, cache generation, domain, marker deadlines and
   existing frames. Use the chart behavior error hooks; do not hardcode time errors.
4. Yield custom writes need explicit extent preflight: the current low-level custom
   DataLayer path does not call ChartBehavior::validation_converter, and Maturity's
   identity converter does not enforce the scaffold point bound. Reject unsafe
   extents before storing data, not later when prepare attempts settlement.
5. Successful yield custom writes/removal mark settlement dirty. Batch writes settle
   once for the effective domain; custom-only charts can grow beyond the minimum.
   Preserve the established source extent semantics after shorten/clear/remove;
   do not invent domain shrinking. Support-only items stay out of public data,
   hits, autoscale and meaningful-data crosshair membership.
6. Oracle custom defaults through the real inherited construction path. Preserve
   chart-wide yield percentage localization and user overrides, while treating
   custom common defaults separately from the yield Line/Area override.
7. Keep exact-row event payloads and renderer hit payloads separate, owned and typed.
   Public accessors require the matching horizontal item/view. Whitespace does not
   return a neighboring custom datum; conflated rendering must not replace original
   event/data_at payloads with aggregates.
8. Conflation preserves original/internal items, representative indices, reducer
   contexts, original counts, odd tails, colors and source-derived cached update
   behavior. Missing reducers retain existing fallback. Compare actual consumer
   chains, including a non-associative/context-inspecting reducer, rather than only
   sums or an isolated helper with different grouping.
9. Retained handles may keep detached model data readable but must not keep the live
   view attached. Removal/chart drop invokes the existing destroy hook exactly once;
   old renderer frames remain drawable/hittable without a live chart borrow.

## Design, alternatives and ownership

Preferred: keep CustomSeriesHandle<V>, deriving model row types from V::Item and
V::InternalTime. Implement operations on Chart<B> with equality bounds tying those
associated types to B. Calendar CustomSeriesHandle<View> syntax remains unchanged;
no redundant behavior type parameter is needed merely to store view data.

Alternative: CustomSeriesHandle<V, B = HorzScaleBehaviorTime>, with explicit behavior
branding and default compatibility. This makes family binding visible but adds
bounds/phantom state to handles and event accessors even though their stored item
and internal types are already fixed by V. Choose it only if the compile spike
shows a concrete inference/type-safety problem with the preferred design.

Do not choose a runtime Time-or-Number enum, Any-based public data interface,
separate copied numeric adapter or stronger Send/Sync/Copy requirements. Keep
existing internal type erasure for heterogeneous renderer composition and hit data.
Parameterize the private cache with row item/internal types or V as appropriate;
choose the smallest form that keeps its existing logic readable.

```text
Chart<B> -> model/coordinator -> strong custom model handle
         -> pane adapter     -> strong same model + strong OwnedView<V>
User handle<V>               -> strong model + weak OwnedView<V>
Prepared frame              -> frozen renderer clone + owned hit geometry
YieldCurveChart             -> Chart<yield behavior> + private support/dirty state
```

No model/view points strongly back to the chart. Keep callback borrows short:
release model borrows before view.update/renderer callbacks, as today. Renderer
clones must freeze visible state; a clone sharing mutable live geometry violates
the existing contract. Do not remove the current resource cleanup hook as part of
this change or broaden its reentrancy promises. Preserve manual Clone/Debug impls
without adding unnecessary V: Clone/Debug bounds.

Provisional API shape (finalize in the first compiling spike):

```rust,ignore
pub struct CustomSeriesHandle<V: CustomSeriesPaneView<Frame>> { /* existing ownership */ }
impl<B: ChartBehavior> Chart<B> {
    pub fn add_custom_series<V>(/* existing view/options/pane/scale arguments */)
        -> Result<CustomSeriesHandle<V>, ChartError>;
    // V::Item = B::Item, V::InternalTime = B::InternalItem, Metadata = ().
    pub fn set_custom_data<V>(&mut self, handle: &CustomSeriesHandle<V>,
        rows: Vec<CustomSeriesDataItem<V::Data, B::Item>>) -> Result<(), ChartError>;
    // Same matching bounds for update and custom/common option replacement.
}
impl<T> PointerEvent<T> {
    pub fn custom_data<V>(&self, handle: &CustomSeriesHandle<V>) -> Option<&V::Data>;
    // Matching V::Item = T; preserve typed custom_hit_data too.
}
// YieldCurveChart exposes corresponding explicit methods with Maturity bounds.
```

Map the API back onto old calendar examples, numeric typed source tests, erased UI
preparation, pointer extraction and yield settlement before modifying production
files. Generalizing the whole widget/frame/model ownership again is unnecessary.

## Ordered delivery and gates

1. Freeze source contract, source oracle, defaults, lifecycle and pre-change images.
   Confirm actual reducer/update consumers and yield custom defaults first.
2. Generalize handles/cache/event accessors in compiling calendar-compatible slices;
   prove one OptionsChart custom renderer end to end. **Gate A:** existing calendar
   callers compile unchanged; no duplicated adapter/render loop.
3. Complete OptionsChart mutations, axes/events/hits, options and dense conflation.
   **Gate B:** numeric source comparisons and ownership/error cases pass.
4. Add explicit yield custom methods with extent validation, dirty settlement and
   correct defaults. **Gate C:** custom-only/mixed curves match settled source
   domain/membership; failed writes cannot leave an unpreparable chart.
5. Add one reusable numeric custom demo renderer with price/yield modes, mixed peers
   and deterministic fixtures; run backend and physical acceptance.
6. Full validation, review and port report. **Gate D:** all promised checks pass,
   with device/timing exceptions still separately recorded.

Task dependencies, file budgets and focused checks are in the linked checklist.
Keep shared generic changes sequential. Split slices above about five files; each
verified slice gets a local checkpoint. Ordinary checkpoints need no extra approval.

## Verification protocol

Before production edits pin source/runner hashes and freeze case counts. Extend
scripts/check_custom_series_reference.cjs when feasible, or add a narrow numeric
oracle that reuses it without weakening calendar cases. Add yield inherited-factory
checks to the existing actual-source harness and distinguish stubs from browser
integration. Native item validation/scaffold bounds are documented corrections.

Use compile compatibility and compile-fail doctests (or an existing harness) for
calendar/numeric data and wrong-family views; do not install trybuild just for this.
Tests use explicit clocks and clone/reducer/cleanup counters. Retain the existing
100,000-row bounded-visible-payload and small-update work assertions; apply
corresponding numeric tests with explicit work limits frozen before implementation.
Check dense/sparse transitions, opt-out, invalidation during budgeted precompute,
no full-history work on hover/price-axis drag and no scaffold rebuild on value-only
same-domain updates. Do not substitute timing thresholds for deterministic work.

Proposed feature matrix: 2 numeric modes x 2 backends x 4 DPR (1/1.25/1.5/2) x 3
sizes (800x500/300x300/300x90) x 5 scenes = **240 images**. Scenes: sparse custom,
mixed peers/overlays, update/whitespace, dense/conflated, empty/reset recovery.
Freeze exact scene membership/counts at checkpoint 1. Preserve **24 images each**
for calendar built-ins, calendar custom, price built-ins and yield built-ins =
**96 baseline comparisons**; select/prepare calendar custom fixtures before the
adapter refactor. Pin binary, source, fixture and font identity. Inspect clipping,
logical spacing and renderer content; native unchanged pixels must match exactly.
Frozen renderers must draw identically after update, remove and owner drop.

Physical mouse: explicit ICED_BACKEND=tiny-skia and ICED_BACKEND=wgpu, both numeric
modes, regular/narrow/earlier problem size. Check live updates, custom hover/hit
payload display, wheel dense/sparse zoom, diagonal pan, both price-axis gestures,
precision/maturity labels, options, clear/reset/remove/re-add, annotations and box
editing/short-height recovery when exposed. Do not infer acceptance from captures.

Final commands: cargo fmt --check; cargo test --all-targets; cargo check --all-targets;
cargo clippy --all-targets -- -D warnings; affected actual-source oracles, including
custom/conflation/price/yield and shared axis/interaction/primitive regressions.
Record commands/counts/revision and the complete source/consumer/ownership/design
report in docs/porting/numeric-custom-series.md. No push unless requested.

## Risks and open decisions

- Associated-type inference and downstream source compatibility: compile old callers
  before broadening mutators; freeze final signatures at Gate A.
- Yield extent bounds and inherited custom defaults: explicit preflight and source
  construction tests; no unrestricted mutable access to the private core.
- Generic internal items need Clone, not Copy: inspect reducer context construction
  and retained rows without making every behavior more restrictive.
- Frozen renderer/live-view aliasing and user callbacks: preserve current short
  borrows and weak ownership; prove removal/drop/old-frame behavior.
- Pre-existing reducer discrepancies, if exposed by numeric cases, require a
  separately bounded regression/fix slice; never rewrite expected source output.

No blocking user choices are needed to prepare or start this plan. Touchpad,
touchscreen and physical high-DPI checks remain untested; the historical timing
exception remains OPEN/deferred independently of this feature.

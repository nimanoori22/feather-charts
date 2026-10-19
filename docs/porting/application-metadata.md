# Application metadata (milestone F)

Baseline: `f578d47`. Design frozen; implementation and acceptance in progress.

## Source, consumers and behavioral contract

Investigated complete `model/get-series-plot-row-creator.ts` and
`api/get-series-data-creator.ts`, public set/update/data/dataByIndex/pop and mouse
conversion, actual DataLayer mutations, customValues unit tests and plugin demo
payloads. Source wraps both fulfilled and whitespace rows with the supplied
customValues reference. Queries/pop/pointer conversion construct new outer rows
but retain that payload reference; replacing a point without customValues clears
it, including whitespace→value replacement. Reindexing preserves surviving
payloads. Public fulfilled queries exclude whitespace. Data-change notifications
report scope; pointer events contain data at the captured index. Built-in geometry is not
an application metadata owner; a custom renderer may retain the original typed
data it deliberately captures in its frozen renderer.

Graph discovery and consumer traces cover series queries/pop, ChartApi pointer
conversion, toolkit custom series and accessibility readers. Relevant source
DataLayer and plot-row-creator metadata changed since indexing; actual current
source was read instead of trusting those entries. Native coverage checks report
no recorded gaps (best effort, not a completeness guarantee). Required consumers
are the public chart families, custom view/model adapters, primitive model access,
markers, events and retained snapshots. No asynchronous or thread-safe contract is
required.

## Alternatives and chosen API

1. `Chart<B, M = ()>` reuses the engine's existing M parameter. Data, model reads,
   primitive contexts and pointer events remain statically typed. `M: Clone +
   'static` permits owned records or cheap `Rc<Record>`; neither Debug, equality,
   Send nor Sync is required. Custom views use the same metadata parameter for
   their existing whitespace metadata; fulfilled custom data remains V::Data.
2. Owned `Rc<dyn Any>` metadata would support heterogeneous payloads but would
   introduce downcasts at every built-in read and alter model/primitive boundaries
   anyway. A side table would duplicate canonical membership and complicate
   replacement, whitespace, pop and failure atomicity. Neither improves these
   consumers enough to justify erasure or parallel data storage.

Concrete engine prototypes in `tests/metadata_design.rs` compile both typed and
Any-erased payloads using a non-Clone/non-Debug/non-PartialEq application record
containing Cell, behind Rc. The public typed round-trip is a recorded failing
consumer before production changes (`target/application-metadata/red-public.log`).
Choose typed metadata; identity and sharing are selected by M. Plain Clone follows
value semantics; Rc cloning retains identity and interior changes. No implicit
payload equality, serialization, deep cloning policy or unsafe cast is introduced.
Source callback identity, mutable outer-row sharing and manual destroy are not
reproduced. Existing owned queues and RAII remain.

Existing unit constructors/defaults and aliases remain usable without new type
annotations. Generic associated constructors otherwise leave M ambiguous, so keep
`Chart::new`, Default, OptionsChart constructors, custom-domain construction and
YieldCurveChart constructors specialized to unit metadata. Add explicit typed
constructors: `Chart::<_, M>::new_with_metadata`, price constructors with a
`_with_metadata` suffix, `with_custom_behavior_and_metadata`, and
`YieldCurveChart::<M>::new_with_metadata`. No separate owner or runtime registry.

Important types:

```text
Chart<B = HorzScaleBehaviorTime, M: Clone + 'static = ()>
PointerEvent<T = Time, M = ()> { series_data: Vec<(SeriesId, BuiltInSeriesDataItem<T, M>)>, ... }
ChartEvent<T = Time, M = ()>
PrimitiveContext<'a, B = HorzScaleBehaviorTime, M: Clone + 'static = ()>
DrawingPrimitive<B = HorzScaleBehaviorTime, M: Clone + 'static = ()>
YieldCurveChart<M: Clone + 'static = ()>
```

Existing set_data/update_data/series_data/data_by_index/pop_data accept/return M;
no duplicate metadata-only mutation methods. OptionsChart remains the existing
unit alias; metadata users name `Chart<HorzScaleBehaviorPrice, M>`. Custom handles
retain `V::Metadata` and reject incompatible chart/domain ownership statically.
Native built-in primitives implement their existing contracts generically over M;
external primitive implementations may likewise add M when needed. Unit callers
and custom views retain their old signatures.

## Ownership and migration slices

```text
Chart<B, M> -> DataLayer<B, M> and ChartModel<B, M> -> canonical M values
Chart -> owned event queue -> captured M values
CustomSeriesHandle<V> -> strong typed model, weak live view
Prepared screenshot -> frozen geometry/text/renderers, no live chart reference
```

Rc payloads retain their object until the final row/event/readback/retained custom
owner is dropped. Clearing/removing series releases engine ownership; existing
owned observations remain valid. An application payload that refers back to its
chart should use Weak to avoid an application-created cycle. No new reverse link
or callback closure captures the chart strongly.

The compiling boundary migration is coupled: model/layer M, API impl blocks,
primitive traits/attachments and built-in primitives, custom handle/adapter/cache
metadata, pointer/event types, widget output and yield/price factories must agree.
F1 therefore migrates that complete type boundary in one compiling slice, with
unchanged default-unit all-target compilation. F2/F3 then prove data/lifecycle and
wrapper/event behavior with focused external consumers; routing remains through
the existing generic engine rather than new data algorithms. F4 adds controls,
source observations and strict renderer/physical acceptance. This keeps each
checkpoint compilable without a temporary parallel chart implementation.

Files expected: api owner/behavior/data/events/interaction/widget, existing
api impl modules, primitives/plugins, custom-series adapter/cache, numeric/yield
factories, public metadata tests, four-domain consumer and capture fixtures.
Engine data algorithms and ordinary renderer geometry need no redesign.

Prior formatting-GPU, timing, dedicated drawing-order, physical-device, Intl and
Display-P3 exceptions remain independent.

## Adapter and data checkpoints

The typed adapter migration (`020bdda`) passes all-target compilation with the
existing unit consumers unchanged. Only API/extension signatures changed; model
data algorithms and renderer preparation already accept M. The first typed public
record round-trip passes without Debug/PartialEq/Send/Sync bounds. Typed custom
views use `V::Metadata` in their existing canonical model/cache and data builders,
with no Clone bound added to the view or its options. Widget messages and prepared
geometry remain metadata-independent.

Focused contracts cover all six series, same-price metadata replacement/clearing,
failed replacement/history, shared reindexing, whitespace, newest-first pop,
retained pointer data, 1MB Rc payload clone/drop accounting, all chart families,
custom views and primitive reads. A missing pane in the first yield test fixture
was corrected by explicitly creating it; no engine behavior was changed for that
fixture error. A custom renderer that retains original data keeps that payload
alive in its snapshot; built-in geometry snapshots do not. Source observations
match all six families using the real current local 5.2.1 source.

## Public consumer and renderer protocol

The existing pane-management application now has an explicit metadata mode; its
unit mode remains unchanged. Its Host carries an associated Metadata parameter,
with short aliases for built-in rows, custom inputs and events. The event queue
has a named owned type alias; no queue implementation or delivery semantics
changed. These names keep the generic boundary readable without lint suppressions.
The shared example box primitive is generic over M. The interval view adds a
defaulted whitespace metadata parameter, while its fulfilled Datum and renderer
remain unchanged. No existing interval constructors needed migration.

Metadata mode uses Rc<Record> for the blue line and a toolbar for latest/history
replacement, clearing, invalid input, pop, retain/read/release and restore. Hover
shows the captured ID/revision; price is unchanged by metadata-only operations.
Retained data belongs to the application panel and survives clearing the engine.
The four-domain consumer test covers all controls, ownership changes, recovery
and Reset. Pause feed for stable record inspection; resume for navigation checks.

Review reproduced a demo-only identity error: sparse blue data ended at bar 22,
but the feed built a bar-23 template before replacing its time. The regression
failed with record ID 23 instead of 22. Feed construction now derives the record
ID from the actual horizontal value, sharing the panel's existing domain lookup.
The regression passes in all four domains; no engine mutation algorithm changed.
Both failing and corrected logs are retained with the final evidence.

The new capture protocol freezes nine scenes (initial/latest/history/cleared/
retained/restored/moved/removed/recovered), four domains, three sizes and DPR
1/1.25/1.5/2 on both asserted backends: 864 screen replays and 864 retained exports.
384 metadata-only screen comparisons must equal their initial pixels. Weak
payload probes must expire after the live owners are reset/dropped, while retained
exports remain unchanged. Existing capture scripts/assertions remain unchanged.

Rust files changed: `src/api/{behavior,chart,configuration,conflation,coordinates,
custom_series,events,image_watermark,interaction,markers,options_chart,
pane_management,price_scale,primitive_editing,primitives,screenshot,series_data,
series_queries,text_watermark,time_scale,touch,up_down_markers,widget,yield_curve}.rs`,
`src/api/custom_series/conflation.rs`, `src/api/yield_curve/custom_series.rs`,
`examples/pane_management.rs`, `examples/pane_management/{host,data,events,helpers,
metadata,metadata_captures}.rs`, `examples/numeric_custom_series/view.rs`,
`examples/support/editable_box.rs`, `tests/metadata_design.rs`,
`tests/application_metadata.rs` and `tests/application_metadata/view.rs`. Source
and renderer scripts, inventory, README and checklist accompany those changes.

No model data algorithm port remains for metadata. Pointer source metadata, locale
behavior and configured color parsing remain milestones G–I; final combined
validation is Z. Physical acceptance and final gate results are recorded below
when complete, without closing unrelated exceptions.

## Validation checkpoint

The corrected consumer passes 860 tests in 87 suites, two compile-fail doctests,
all-target build/check, strict all-target Clippy, formatting and inventory checks.
All 43 current source comparisons pass. Both asserted renderers pass 864 screen
replays, 864 retained exports and 384 metadata-only pixel comparisons. Representative
cleared-data and moved-pane captures were inspected visually. Results are retained
under `target/application-metadata/corrected`.

The first compatibility attempt ended with SIGTERM after a successful software
event capture, without an assertion failure. Its partial output is preserved;
the cause of the signal was not established. A separate unchanged-protocol retry
is required before acceptance. Final compatibility and physical acceptance remain
pending at this checkpoint.

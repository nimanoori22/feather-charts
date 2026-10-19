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
report scope; pointer events contain data at the captured index. Geometry is not
an application metadata owner.

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

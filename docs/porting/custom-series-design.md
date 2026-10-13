# Public custom series: implementation contract

Status: source contract and initial API design recorded; production implementation waits for
the preparation-performance checkpoint in `tasks/custom-series-plan.md`.

## Source and consumers

`src/model/icustom-series.ts` defines an application-owned pane view, typed
original data, renderer, whitespace predicate, price builder and options.
`src/model/series/custom-series.ts` combines that view with normal series
registration. `src/model/series/custom-pane-view.ts` converts horizontal
positions, supplies bar spacing and an array-index visible range, and delegates
drawing and optional hit testing. Its price converter uses the owning series'
actual price scale and visible first value. A missing custom hit falls back to
the projected price envelope; point, line and range priorities arbitrate hits.

`ChartApi.addCustomSeries` and `PaneApi.addCustomSeries` use the ordinary series
registration path. SeriesApi data updates, primitive attachment and pane
membership depend on that same owner. The box-and-whisker plugin uses high,
low and median to supply extrema and current value. The price builder's final
element is the current value, regardless of where it lies within the extrema.

Graph queries identify these API/model relationships. Interface dispatch is
incomplete in the graph; direct source reads determine the renderer contract.
Rust coverage checks for the public chart, model, custom series, data layer,
frame owner, hit testing and decoration integrations reported no recorded gaps
at generation 2026-10-13T19:39:22Z. This is best-effort evidence, not an
exhaustive consumer claim.

## Existing Rust owners

`CustomSeries<I, O, D, M>` already owns typed plot rows, common custom options,
formatter, generation and price projections. `CustomDataLayerAdapter` retains
the builder and passes typed payloads through normalization without storing
them in the heterogeneous data layer. ChartModel registration already retains
the same typed handle in the coordinator and price-scale source adapter.

The public Chart currently registers only built-in views. Price decorations,
markers and primitive conversion currently inspect built-in Series directly.
Extending only the drawing path would leave those integrations disconnected.
The existing custom renderer/view traits remain the extension contract.

## Designs compared

One approach makes Chart generic over an application's complete series enum.
It permits static dispatch, but every host and public widget would need that
enum, and adding a custom type changes the chart type. It also duplicates the
already-existing typed owner boundary.

The chosen approach retains the current Chart and typed custom owner, with
one object-safe adapter for heterogeneous frame preparation. A private common
projection interface lets decorations, marker placement and primitive price
converters use built-in and custom owners without copying all custom history
or introducing a second model. Typed handles validate chart membership before
mutation and lookup. Type erasure occurs only at composition/event boundaries;
typed lookups use checked identity/type matching.

The adapter updates the supplied view with bounded visible original data,
then creates an immutable renderer snapshot. Compare copying a Clone renderer
against requiring a separate snapshot method.
Clone is simpler for existing traits, but its documented contract must prohibit
sharing mutable live geometry. An explicit snapshot method enforces a clearer
boundary at the cost of a new author-facing abstraction.

Choose the Clone renderer bound for the first public adapter. A renderer clone
must own its visible geometry; cloning a live mutable view handle does not meet
the contract. Retained-frame tests exercise drawing and hits after replacement
and removal. The preparation adapter receives the settled visible range,
index-to-coordinate closure, bar spacing, size, ratio and frozen owner price
transform. This keeps the multi-pane owner generic over horizontal behavior
without downcasting ChartModel or borrowing a live model during drawing.

Common projection is borrowed from the same registered Series/CustomSeries
owner. It exposes first/current values, row search, common options and extension
state. Ordinary built-in preparation keeps its concrete Series borrow. Custom
preparation is the only new heterogeneous renderer boundary.

Custom normalization must build all normalized rows and validate builder values
before changing DataLayer membership. Calling a stateful price builder once for
validation and again after mutation is insufficient: the second call can fail.
The prepared rows are consumed in one synchronous mutation, with no callback
held across model application.

## Required ownership and lifecycle

```text
Chart -> custom preparation adapter -> typed view
ChartModel/coordinator/price source -> Rc<RefCell<CustomSeries<D>>>
typed public handle -> same CustomSeries<D> identity
typed public handle -> weak view/options owner
prepared frame -> copied visible geometry and frozen price transform
```

Neither the typed model nor view retains Chart. Frame drawing and hits must not
borrow the live model or view. Removing a series invalidates its public handle
for chart operations and releases the preparation adapter; retained frames
remain drawable. An adapter's Drop calls the view cleanup hook exactly once;
retaining a data handle must not retain an attached chart or postpone cleanup.

The adapter owns the view and its renderer-specific options together. Its owned
view wrapper invokes destroy on Drop; frames clone only the renderer, and the
public handle has only a Weak reference to the wrapper. Temporary price-builder
adapters borrow this wrapper synchronously during normalization and are dropped
before the update is applied. There is no strong path from any of these owners
back to Chart, so their ownership graph cannot form a chart cycle. Membership
and typed model identity are checked before every mutation; retaining a model
handle after removal does not grant access to another chart's series.

## Required behavior and incidental source patterns

Required behavior includes synchronous update/draw ordering, visible array-index
ranges, builder-derived extrema/current value, original payload/time recovery,
actual owner-scale conversion, optional hits, and cleanup on removal. Source
plugins rely on the pane view being updated before its renderer is drawn.

Rust does not reproduce JavaScript object spreading, prototype inheritance,
mutable nullable renderer references or manual public destroy calls. Common
options and application options compose; an owned view supplies cleanup through
Drop, and a frame owns a renderer snapshot. Callback identity and shared mutable
browser state have no demonstrated requirement at this boundary. Snapshot
cloning copies only bounded visible data and must not borrow the application
view during drawing.

## Behavioral gates

- Validate all times, ordering, colors and builder values before mutation.
- Keep typed data and metadata, explicit whitespace and shared time indexes.
- Replace an existing row incrementally; shared-time insertion may reindex.
- Bound preparation and autoscale traversal to visible rows.
- Use actual owner scales for price modes, inversion, manual navigation,
  primitives, current-value labels and price lines.
- Preserve custom hit priority, optional cursor/object identity and fallback.
- Pointer data uses exact logical indexes; whitespace does not invent a value.
- Support two different custom data types in one chart and stale/foreign handles.
- Preserve retained frames across updates, clear/reload and removal.
- Verify both native renderers and the source box-and-whisker contract.

Conflation rendering and arbitrary horizontal-scale public APIs remain outside
this milestone; existing custom conflation contracts remain intact.

## Proposed API and consumer mapping

Keep common chart options in the existing CustomSeries owner. Renderer-specific
options belong to the typed preparation adapter. Rust composition replaces
TypeScript option inheritance: the renderer's `Options` can be an application's
own type, while chart scales and decorations read `CustomSeriesOptions`.

```rust,ignore
pub struct CustomSeriesHandle<V> { /* private identity and typed owner */ }

impl<V> CustomSeriesHandle<V> {
    pub fn id(&self) -> SeriesId;
    pub fn data_at(&self, index: TimePointIndex) -> Option<V::Data>;
}

impl Chart {
    pub fn add_custom_series<V>(
        &mut self,
        view: V,
        options: CustomSeriesOptions,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<CustomSeriesHandle<V>, ChartError>;

    pub fn set_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        data: Vec<CustomSeriesDataItem<V::Data, Time>>,
    ) -> Result<(), ChartError>;

    pub fn update_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        data: CustomSeriesDataItem<V::Data, Time>,
        historical: bool,
    ) -> Result<(), ChartError>;

    pub fn replace_custom_view_options<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        options: V::Options,
    ) -> Result<(), ChartError>;

    pub fn replace_custom_series_options<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        options: CustomSeriesOptions,
    ) -> Result<(), ChartError>;
}
```

The public adapter binds V to the existing CustomSeriesPaneView<Canvas Frame>
with Item=Time, InternalTime=TimePoint and Metadata=(). Data, view options and
renderer must be owned 'static values; Renderer implements Clone, and typed hit
data implements Clone + 'static for event ownership. These bounds do not require
Send, Sync or Debug on application payloads. data_at is exact-index and restores
original time; it remains readable after detachment. Chart mutations additionally
validate current registration and the same typed model identity.

The initial public Chart uses Time and unit common metadata, as do its existing
built-in APIs. Applications retain arbitrary typed metadata inside their custom
datum. Lower-level CustomSeries/DataLayer generic metadata remains available.
Explicit whitespace still participates in shared time membership.

A custom drawing target uses the existing Iced Canvas Frame, the same adapter
boundary as DrawingPrimitive. TinySkia and WGPU consume that same logical
drawing API. Compare a separate owned-command target against direct Canvas
drawing: commands would require a second author-facing drawing vocabulary and
translation layer, while a frozen renderer can already draw synchronously into
both backends. Choose Canvas with immutable owned renderer snapshots; engine
custom data and price projection remain independent from Iced. Existing
CustomSeriesPaneRenderer/CustomSeriesPaneView traits bind to that target.
PaneRendererCustomData additionally carries the settled pixel ratio. Canvas
coordinates remain logical; a renderer can use the copied ratio to reproduce
source bitmap snapping and then convert back to logical coordinates. Frame
alone does not expose FancyCanvas's bitmap coordinate scope, so omitting this
information would prevent the box-and-whisker plugin's pixel rounding from
matching at fractional ratios. Ratio changes therefore require a new frozen
renderer snapshot even when original data is unchanged.
Require the renderer to clone an immutable visible snapshot;
the copied renderer must not share mutable live geometry. This retains existing
renderer/view contracts while ensuring retained-frame behavior is tested.

Model consumers use a private common projection boundary for first/current
values, exact/nearest rows, colors, markers, primitive autoscale and price lines.
It borrows each existing owner; it does not build a parallel projected history.
Existing built-in preparation still borrows the concrete built-in Series.
Custom frame composition uses a small object-safe adapter backed by its typed
view and existing CustomSeries handle. Frames retain the immutable renderer,
bounded original data and copied price transform, never a mutable view owner.

Public typed data lookup and pointer-event payload lookup validate handle
identity and use checked type matching. Erasing an owned cloned datum into an
event does not erase its ownership or retain the chart. Custom hit payloads stay
separate from hit equality so a fresh payload allocation cannot cause an idle
redraw loop. Ordinary series removal also removes custom adapters and invokes
cleanup, while a retained typed data handle remains readable but unattached.

Hover preparation retains the winning custom hit payload separately from
ChartHit equality. Drawing and pointer events use that same owned payload;
they must not call an application hit callback again to reconstruct the result.
This preserves a stateful callback's actual arbitration result and avoids
per-draw hit allocations. A fresh explicit hit query may invoke the callback,
but retained frame drawing never consults the live application view.

`get-series-data-creator.ts` maps custom rows back to public data by restoring
`originalTime`; ChartApi mouse events and SeriesApi data/dataByIndex/pop consume
that result. Typed Rust data and pointer lookup must therefore clone only the
requested datum and restore its original public time, including business-day
strings. Renderer payloads retain their typed additional fields rather than
emulating TypeScript's destructuring that strips time/color properties.

## Consumer mapping and implementation slices

| Consumer | Rust mapping and contract |
| --- | --- |
| ChartApi / PaneApi registration | Chart allocates its ordinary unique SeriesId and registers the existing typed CustomSeries owner in the requested pane/scale |
| DataLayer and SeriesApi mutations | Typed builder adapter normalizes once; DataLayer supplies shared indexes, and the same typed model receives original rows |
| CustomPaneView update | Adapter binary-selects visible rows and join neighbors, supplies rebased usize array range and logical x coordinates |
| Box-and-whisker renderer | Cloned renderer owns visible high/low/median geometry; the frozen actual owner price transform converts prices during Canvas drawing |
| Autoscale and price decorations | Common owner projection exposes builder extrema and final current value; no proxy built-in series or copied history |
| Markers and primitives | Existing attachment identity resolves the same owner projection and price scale; removal releases its attachments |
| Pointer and custom hit events | Event owns one exact-index cloned datum, restored original time, and separate optional typed hit payload; checked handle lookup restores the type |
| Removal and retained frames | Adapter Drop cleans up the supplied view once; immutable cloned renderers remain drawable; removed handles cannot mutate Chart |

Implement registration and frozen rendering first, followed by complete data
lifecycle, then decorations/hits/events and attachments. Transactional row
construction is a foundation of the first public mutation path, even though its
full rejection matrix belongs to lifecycle validation. Tests must distinguish a
custom missing exact-index row from nearest-neighbor autoscale/marker lookup.
Lower-level custom membership reindexing must still update every registered
owner when another series introduces a shared time point.

## First foundation slice: rejected data is non-destructive

Investigated upstream DataLayer custom row creation and the existing Rust
CustomDataLayerAdapter/ChartModel typed consumers. Rust exposes fallible row
creation, so errors must preserve chart membership even where the upstream
builder convention assumes valid input. Previously replacement removed old
memberships before preprocessing/building rows, and append inserted a point
before calling its price builder.

Choose prepared owned rows before mutation over cloning the complete layer and
rolling it back on error. Staging moves typed payloads without requiring a new
Clone bound or copying unrelated history. Price builders run once per fulfilled
input; a second validation callback could disagree with the first. The prepared
rows are then consumed by the existing membership/reindex handoff. No model
borrow or callback survives this synchronous handoff.

Changed `src/model/data_layer.rs`. Regressions first failed on discarded original
membership and a leaked empty append point. They now cover rejected builder
values, invalid time preprocessing, shared built-in membership, last-time state,
registration state and once-per-input builder calls. All 344 library tests,
library cargo check/strict Clippy and formatting pass. Public rendering, complete
validation/options, retained-frame lifecycle and decorations are subsequent
slices; this foundation alone does not complete custom-series acceptance.

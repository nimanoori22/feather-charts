# Series markers

## Source and contract

Reference revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0`.
Investigated `src/plugins/series-markers/{types,options,wrapper,primitive,pane-view,
renderer,utils,series-markers-circle,series-markers-square,series-markers-arrow,
series-markers-text}.ts`, the series primitive adapter, series autoscale aggregation,
pane hit selection/drawing and marker interaction/coverage/type tests.

Graph discovery and the two-way `createSeriesMarkers` trace establish wrapper,
primitive and tutorial/accessibility/type-test consumers. Dynamic plugin callbacks
require direct source inspection; the method trace was unavailable. Coverage checks
flagged modified `model/series.ts` and excluded coverage tests, both read directly.
Graph results are best-effort evidence rather than complete dynamic call graphs.

Public consumers replace/read markers, apply options and detach an attachment.
Primitive consumers need independent attachments, synchronous data-change
reindexing and autoscale margins. Views require nearest shared-time lookup then
nearest series row, same-time stack order and per-series scale projection.
Renderer consumers require bitmap alignment, shape sizing, text bounds and layers.
Pane event consumers require first-hit ordering inside each attachment and reverse
stable series order across equal-priority marker hits. Source marker hits report
normal order regardless of their drawing layer. General built-in hit arbitration
is outside this milestone.

Required behavior: all four shapes, six positions, optional ID/text, source size
clamping, sorted times with equal times permitted, independent attachments,
replacement/clear/detach, whitespace/nearest time mapping, hidden/empty output,
data/reindex/navigation/scale invalidation, autoscale pixel margins, drawing layers,
shape/text hit tolerances and owned pointer identification. Source negative sizes
clamp to zero; zero suppresses shapes but permits text. Explicit-price positions
carry a required finite price. Optional external IDs need not be unique.

Source margins use position presence and spacing, not arbitrary text/stack bounds
or explicit prices. Across attachments, the last enabled nonempty primitive margin
replaces the prior primitive margin; across series scales take maximum margins.
Marker times do not add time-scale rows. Before the first series time search right;
otherwise search left after shared-time nearest lookup. An empty collection draws
and hits nothing. Clearing data keeps annotations for reload.

## Design and ownership

Compared a direct single collection on each series with narrow attachment IDs.
Choose attachment IDs: repeated source construction and explicit detach semantics
must not accidentally replace another producer's markers. IDs are process-unique,
never recycled, and separate from optional user IDs. No generic primitive registry.

Series owns collections and their original time values/options. Chart owns the
model and the normal prepared-frame pipeline. Attachments are plain typed IDs;
dropping an ID does not detach (source also retains an attached primitive after a
temporary wrapper is discarded). Removal is explicit; series/Chart drop releases
collections through existing ownership. No callback capture, back-reference or
new strong/weak edge is introduced; no additional Rc cycle can occur.

```text
Chart -> model -> series -> owned marker attachments
Preparation -> owned shapes/text/hit metadata -> immutable frame
Canvas -> borrows frame -> owned input message -> Chart event
```

API: `Chart::add_series_markers(series, markers, options) -> Result<MarkerCollectionId,
ChartError>`, `set_series_markers(id, markers)`, `series_markers(id)`,
`series_marker_options(id)`, `replace_series_marker_options(id, options)`,
`remove_series_markers(id)`. `SeriesMarker<T>` owns time, position, shape, color,
size, optional string ID and text. `MarkerPosition` explicit-price variants embed
the price. `MarkerHit` identifies SeriesId, collection, replacement-local index
and optional external ID. Replacement invalidates old index identity.

JavaScript mutable arrays, object/subscription identity, manual destruction,
DOM attachment and draw-time mutation of text widths are discarded. Rust rejects
invalid colors/nonfinite or unrepresentable geometry/unordered input transactionally
rather than retaining a partly invalid collection or crashing later. Text metrics
and glyph paths are prepared using the existing Iced font machinery; Canvas font
pixels are not required to match. Exact Iced paragraph/glyph APIs were inspected.

Model autoscale margins are calculated synchronously using current spacing before
pane recalculation. Marker projection runs after layout settles. Public drawing
uses owned geometry; hover uses the displayed settled logical coordinates without
magnet snapping. Data changes in other series remap through shared time indexes.
Normal markers draw with their series, above-series on the top background pass,
and top on the top foreground pass after crosshair. Existing label layering remains.

## Implementation and validation

Pending; results and exact changed files are recorded as slices complete.

## Remaining dependencies

Full series/price-line hover arbitration, arbitrary custom primitives, arbitrary
horizontal behavior in the public Chart, touch/kinetic input, editable annotations
and physical acceptance remain separate. Source subscription cleanup quirks are
not reproduced; ownership releases data deterministically.

# Preparation performance

Foundation: `36a2955`. Status: bounded preparation and existing-time replacement
implemented; three-set controlled static, ordinary static/latest and append
measurements complete. Append remains on the full reindex route, with tail
exceptions reported below.

## Contract, measurements and budgets before production changes

Fixed logical visibility is 20 through 100 (81 rows). Retain at most two
neighbors on each side for continuous series: an extra drawing join and a cubic
control point. Model history remains intact. Unchanged preparation must avoid
history-sized coordinate conversion and copied render data. A new shared time
must update shared indexing; the current implementation uses a full reindex
route. Replacing the latest existing timestamp must avoid a full-history
response copy.
Operation-count assertions gate these contracts; elapsed-time checks stay out
of normal tests to avoid noisy machine-dependent failures.

Before optimizing, the provisional timing budget is at most 10% growth in
fixed-visible static preparation when history grows from 1,200 to 100,000 rows,
and no greater than 10% regression against baseline median/p95 in representative
existing navigation/update workloads. Three alternating baseline/current runs,
20 warmups and 100 measured samples are required for timing evidence. Report
tail variance instead of suppressing inconvenient runs. Debug diagnoses cost;
release evaluates performance. CPU snapshot and glyph measurements do not
establish GPU rendering speed or physical-device smoothness.

`examples/preparation_profile.rs` separates `Chart::update_data` (normalization,
shared indexing and model application), `Chart::prepare_snapshot` (settlement,
projection, layout and decorations) and `IcedMultiPaneChartFrame::from_snapshot`
(glyph conversion). It measures static preparation, latest-bar replacement and
append for Line/Candlestick/Histogram, one and 16 sources, and histories of
1,200/20,000/100,000. Source data generation is outside timed samples. The
viewport remains fixed; only the first source receives a live update. Size is
917 by 870.80005, ratio 1.25. No renderer backend participates in these CPU
measurements. They are phase timings, not a sampling profiler's attribution of
every function.

A separate `--model-phases` diagnostic measures DataLayer normalization/index
response creation, model application, explicit invalidation settlement, per-series
visible projection/drawing commands, and model axis snapshots. It uses fixed
800 by 500 engine dimensions and ratio 1.25; it excludes Iced font measurement,
layout iteration and glyph conversion. These five engine probes do not sum to
the public Chart pipeline: each consumer has different preparation and lazy
cache behavior. `prepare_series_plot` and owner layout are private boundaries,
so the public profile does not claim to isolate every layout/font operation.

The initial focused test fails behaviorally: `prepare_line` copies all 1,200
rows into its owned output despite only 81 visible rows plus four necessary
neighbors. It checks Simple/WithSteps/Curved and two history sizes, preserves
the full model history, and validates that visible endpoints remain present.

## Source evidence and design

Graph search traced `prepare_series_plot` to pane views/renderers. Coverage for
the current public API profile, chart, series, data layer, multi-pane owner,
line/area/baseline preparation and line renderer reported no recorded gaps,
with metadata matching generation 2026-10-13T19:26:09Z. This is best-effort;
dynamic interfaces and unrecorded edges do not prove exhaustive coverage.
Direct source reads show that continuous-series preparation projects every
history row, and line rendering subsequently copies those projected items
again. Other built-ins first create a full-history timed vector before slicing.

The parent's complete upstream `time-data.ts` investigation traces
`visibleTimedValues` consumers to `SeriesPaneViewBase._makeValidImpl` and
`SeriesMarkersPaneView._makeValid`. Its contract partitions logical indexes,
optionally keeping one join neighbor on either side. Rust `walk_line` additionally
reads a control point beyond that drawing range. Select the extended drawing
range plus one storage neighbor at each side for continuous series, then rebase
the drawing range into that slice. Candlestick/Bar need exact visibility;
Histogram retains its existing join policy.

Two viable approaches are caching complete projected history and selecting only
the needed rows by binary search. Choose binary selection: it bounds conversions
and allocations without introducing cache invalidators. Every preparation uses
current coordinate mapping and copies owned geometry. Series owns complete
history; a frame owns only immutable copied visible geometry, with no live-model
back-reference or cycle. Generic index-based partitioning preserves the existing
`visible_timed_values` binary-search contract rather than introducing a parallel
algorithm. Source/model options, ranges and layout changes naturally produce new
geometry without a stale cache.

The parent also identified full-history range scanning in
`PlotList::min_max_on_range_cached`; fixed-range min/max must bound traversal.
Custom-series first-value/autoscale need the same scrutiny in the next stage.

## Results

The initial debug diagnostic run (20 warmups, 100 samples) isolates the two
history-dependent phases. Parent compilation overlapped parts of this run, so
these figures are diagnostic and are not the final acceptance comparison:

| Line rows / sources / operation | Update median ms | Snapshot median ms | Glyph median ms |
| --- | ---: | ---: | ---: |
| 1,200 / 1 / static | 0.0003 | 7.574 | 0.440 |
| 20,000 / 16 / latest replacement | 112.729 | 97.408 | 0.789 |
| 20,000 / 16 / append | 291.126 | 95.376 | 0.781 |
| 100,000 / 1 / static | 0.0006 | 29.158 | 0.470 |
| 100,000 / 16 / static | 0.0006 | 373.513 | 0.782 |

The metadata counter separately reproduces 5,000 clones for one existing-time
replacement in a 5,000-row history. It does not rely on elapsed-time noise.
The bounded-renderer regression test independently constructs original
full-history LineRendererData from the settled model mapping and compares
Simple/WithSteps/Curved drawing commands at ratios 1, 1.25 and 2, preserving
edge joins and cubic controls.

Both behavioral regressions are now green in
`tests/preparation_performance.rs`: fixed-visible Line output contains at most
85 points for 1,200 and 20,000 stored rows, while complete model history remains
available. Existing-time replacement returns one changed row, leaves shared
time points unchanged, and clones metadata at most eight times rather than
5,000. These bounds test useful output and unchanged-history cost rather than
depending on a timing threshold.

Shared Cargo target reuse initially linked the current library into a rebuilt
baseline example despite archived source still containing the original code.
The anomalously fast replacement timing exposed this before acceptance. Those
baseline 2/3 logs and the first baseline's resumed remainder logs were
quarantined and excluded; force library recompilation by
touching each source checkout's `src/lib.rs` before switching builds. Every new
profile process now prints a semantic startup sentinel: replacing the last of
three rows returns three response rows on the baseline and one on the current
engine. The runner verifies this identity before accepting a case. Debug and
release binaries were independently checked. Source archives and standalone
executables prevent later target changes from altering an active measurement.

### Controlled static history gate

Three serial alternating baseline/current release sets use 20 warmups and 100
measured samples per case, with no competing builds/captures. Baseline is
`36a2955`; current engine is `2e62032`. The controlled fixture pins every source's
last stored value to 100, and latest/append updates to 100 plus revision epsilon.
Visible rows 20 through 100 remain unchanged. This controls global last-bar
price lines and labels as well as logical visibility.

For the 16-source Line and Candlestick cases, both history lengths and both
versions have exactly the same 856 by 842.400024 plot bounds, 16 right-axis
labels, and retained hit items (Line 1,360; Candlestick 1,296). Line has zero
visible last-value price lines; Candlestick has 16. Geometry behavior is also
covered by the independent full-history command oracle and renderer captures.

The table gives the median of three per-run medians/p95s, in milliseconds,
for the snapshot phase (not an inferred whole-pipeline percentile):

| Kind / rows / sources | Baseline median / p95 | Current median / p95 |
| --- | ---: | ---: |
| Line / 1,200 / 16 | 5.066434 / 5.383063 | 3.198423 / 3.300258 |
| Line / 100,000 / 16 | 167.568680 / 182.620826 | 3.255993 / 3.386536 |
| Candlestick / 1,200 / 16 | 8.491489 / 8.677704 | 8.140351 / 8.366307 |
| Candlestick / 100,000 / 16 | 33.198466 / 35.423138 | 5.962693 / 6.175479 |

Line's current median/p95 history-growth ratios are 1.0180/1.0261. Every
individual run's ratios are below 1.039. Candlestick's ratios are
0.7325/0.7381, with no growth in any run. Both pass the provisional 10% static
history-growth screen. This does not imply identical timings at all lengths or
that each font/layout operation has constant cost.

The original varying-last-price matrix is retained separately. Its first
current set had 16-source Line/Candlestick snapshot growth of 59%/47% despite
identical visible data: the last price changes from below the pane to inside it,
adding visible global price lines and changing label layout. That is a different
rendered scene, not a clean history-only comparison. It remains a valid ordinary
workload and is not silently discarded. Three valid original-scene static/latest sets are now complete after repairing
the missing old-engine cases. In the aggregate,
16-source Line median/p95 growth is 1.656/1.836 and Candlestick is 1.497/1.516;
even single-source Line's p95 ratio is 1.105. These differing-decoration results
do not pass a literal 10% screen and are reported separately from the identical
controlled scene. They cannot be described as universally flat workload costs.

The controlled single-source fixture was also repeated three times for all
three profiled built-ins. Snapshot median/p95s in milliseconds:

| Kind | 1,200 rows | 100,000 rows | Median / p95 growth ratio |
| --- | ---: | ---: | ---: |
| Line | 0.419328 / 0.480937 | 0.413100 / 0.461875 | 0.9851 / 0.9604 |
| Candlestick | 0.427689 / 0.501066 | 0.425116 / 0.504365 | 0.9940 / 1.0066 |
| Histogram | 0.362904 / 0.405263 | 0.355206 / 0.403007 | 0.9788 / 0.9944 |

All pass the same 10% history-growth screen; original varying-decoration
single-source Line tails are still retained above. The separate native editing
profile's three build-free repetitions also pass across 1,200/20,000/100,000
histories: per-run static median growth ratios are 1.0016, 1.0045 and 1.0025,
and p95 ratios 0.987, 1.0044 and 0.9795. An earlier native run overlapped Cargo
and is excluded. These native measurements include routing and host preparation;
they do not establish physical mouse or touchscreen acceptance.

### Existing-timestamp updates

The validated original-scene alternating release sets show the changed-row handoff
removes the old history cost. The table contains raw public `update_data` phase
medians/p95s (median of three validated run summaries), in milliseconds:

| Kind / rows / sources | Baseline median / p95 | Current median / p95 |
| --- | ---: | ---: |
| Line / 20,000 / 1 | 5.882731 / 7.405863 | 0.003739 / 0.004202 |
| Candlestick / 20,000 / 1 | 6.006083 / 6.699324 | 0.004069 / 0.004565 |
| Histogram / 20,000 / 1 | 5.650732 / 6.405476 | 0.003854 / 0.004020 |
| Line / 100,000 / 16 | 128.765623 / 144.559512 | 0.043893 / 0.099523 |
| Candlestick / 100,000 / 16 | 134.078018 / 172.641749 | 0.056646 / 0.144831 |
| Histogram / 100,000 / 16 | 166.059040 / 178.065984 | 0.032572 / 0.047607 |

This complements the deterministic metadata-clone guard; it does not include
snapshot/font/glyph work or a new shared-time insertion.

Raw logs are retained under `/tmp/feather-preparation-final/`, with controlled
sets in `controlled-static/` and original varying-decoration workloads in the
parent directory. Standalone checked baseline/current debug/release executables
and source archives are under `/tmp/feather-preparation-*`. Independent engine/font/layout/primitive diagnostics are recorded below;
append comparisons and repeated append summaries remain pending.

### Independent narrow diagnostics

Additional 20-warmup/100-sample release and debug probes are stored in
`diagnostics/`. These are diagnostic single runs, not the three-run acceptance
comparison, and their phases cannot be added to the public Chart timings.
The direct model scenario is 800 by 500, ratio 1.25, Line, 20,000 rows, one
source; unlike the public snapshot it does not measure Iced font/layout/glyphs.
Release medians in milliseconds:

| Independent engine phase | Baseline latest | Current latest | Current append |
| --- | ---: | ---: | ---: |
| DataLayer normalize/index/response | 5.574670 | 0.000553 | 6.144851 |
| Model application | 1.589733 | 0.001333 | 1.441178 |
| Explicit invalidation settlement | 0.033508 | 0.023771 | 0.204128 |
| Visible geometry and drawing commands | 2.233402 | 0.009842 | 0.016733 |
| Model axes/decorations | 0.027143 | 0.009406 | 0.025149 |

Independent public font/layout/primitive probes operate on a settled controlled
snapshot, 917 by 870.80005 chart bounds, one source, ratio 1.25. Font measurement
calls `IcedAxisMeasurer::measure`; layout directly calls
`allocate_multi_pane_layout` with measured requests; primitive preparation
obtains the actual owner context and prepares a copied box renderer. No backend
painting or widget routing occurs in those small probes:

| Independent phase | 1,200 median / p95 ms | 100,000 median / p95 ms |
| --- | ---: | ---: |
| Font measurement | 0.172101 / 0.180140 | 0.167940 / 0.217578 |
| Pane allocation | 0.000180 / 0.000284 | 0.000181 / 0.000428 |
| Context and box preparation | 0.000308 / 0.000375 | 0.000350 / 0.000821 |

The nanosecond-scale allocation/box tails are noisy; these diagnostics do not
apply the 10% acceptance screen or establish a physical frame rate. Private
owner layout iteration, font work and primitive decoration remain grouped in
the end-to-end snapshot measurement; the separate probes identify public
subsystem cost rather than asserting a complete additive decomposition.

Reproduce focused probes with:

```sh
cargo run --release --example preparation_profile -- --rows 20000 --series 1 --kind line --phase latest --model-phases
cargo run --release --example preparation_profile -- --rows 100000 --series 1 --kind line --phase static --fixed-last-price --auxiliary-probes
```

Genuinely inserting a new shared time currently follows a full time-point and
series-row reindex/response route. Insertion in the middle requires shifting
later indexes; a monotonically appended time does not intrinsically require
cloning every prior row. The current full append route is an implementation
limitation, to be measured and documented separately from the bounded static
and existing-timestamp replacement contracts.

### Completed append comparison

All 108 append cases are complete: three serial alternating baseline/current
sets, all three kinds, three history lengths and both source counts. Each case
has 20 warmups and 100 samples, no competing Cargo/capture work, and a verified
engine sentinel. Pauses for other development occur between complete cases.
An idle runner terminated during one pause was restarted without losing any
active measurement. The immutable engines remain `36a2955` and `2e62032`.

These are medians of the three per-run median/p95 measurements, in milliseconds.
Update includes normalization, shared indexing and model application; snapshot
is the separate public preparation phase. Neither column is a whole-pipeline
percentile.

| Kind / rows / sources | Baseline update median / p95 | Current update median / p95 | Baseline snapshot median / p95 | Current snapshot median / p95 |
| --- | ---: | ---: | ---: | ---: |
| Line / 20,000 / 16 | 107.850 / 111.194 | 85.687 / 93.895 | 35.265 / 35.854 | 5.510 / 5.756 |
| Candlestick / 20,000 / 16 | 95.495 / 100.528 | 85.194 / 111.692 | 10.673 / 11.256 | 6.338 / 6.608 |
| Histogram / 20,000 / 16 | 100.078 / 104.346 | 83.702 / 90.524 | 9.974 / 10.495 | 5.566 / 5.810 |
| Line / 100,000 / 16 | 621.290 / 646.804 | 559.530 / 593.436 | 163.886 / 171.375 | 6.244 / 6.659 |
| Candlestick / 100,000 / 16 | 570.375 / 624.530 | 535.480 / 568.624 | 29.357 / 31.627 | 7.030 / 7.474 |
| Histogram / 100,000 / 16 | 616.353 / 658.618 | 567.902 / 593.387 | 29.458 / 30.817 | 6.314 / 6.736 |

At 100,000 rows with one source, current append update medians remain
33.120..34.649 ms and snapshot medians 1.175..1.264 ms. At 16 sources the full
update still takes 535..568 ms. Preparation improvements do not make genuine
new-time insertion independent of history or source count.

The Candlestick 20,000-row/16-source update p95 grows by 11.1%, exceeding the
prewritten 10% no-regression screen even though its median improves by 10.8%.
Its glyph p95 also grows from 0.100989 to 0.114898 ms (13.8%); median improves
from 0.087353 to 0.084102 ms. These are the two aggregate append exceptions and
are retained. The initial Line 100,000/16 append update had a large regression
(621.290 to 759.092 ms median); runs 2/3 do not reproduce it, and its three-run
aggregate improves. That first run is included in the aggregate, not discarded.

These append-tail exceptions do not change the controlled unchanged-scene
static preparation gate or bounded existing-time operation counts. They do
prevent claiming every update/glyph workload passes a 10% screen. The append
route and tail variation remain explicit follow-up work; the threshold has not
been weakened.

Full ordinary phase evidence is recorded in
`/tmp/feather-preparation-final/ordinary-all-phases-summary.json`: all 324
version/kind/history/source/phase/group entries have three per-run measurements.
Raw logs are alongside it; invalid artifact-reuse logs remain quarantined in
`invalid-shared-target-reuse/`. Controlled-scene logs and summaries are in the
separate `controlled-static/` and `controlled-single/` directories.

## Renderer validation

The six built-in capture matrices pass on TinySkia and WGPU at ratios 1, 1.25,
1.5 and 2: 224 captures per backend. Both backends additionally pass 56 hover
states and 20 primitive-edit states. Geometry, finite coordinates, pixel layering
and repeated rendering assertions run inside the capture examples. A contact
sheet was inspected for Line/Area/Baseline and hover-disabled overlap; backend
results agree visually. Artifacts are in `target/preparation-captures/` (ignored
generated files).

The hover-disabled capture exposed an existing foreground ordering defect:
all fills preceded all outlines, although upstream FillSeriesCompositeRenderer
draws each source's fill then outline in one foreground call. Moving fill drawing
to its source foreground pass preserves normal order and paints a raised fill
exactly once. The original capture fails before this correction and all 56 hover
captures per backend pass afterward. No capture assertion was relaxed.

Formatting, all-target tests/check/Clippy and the 16 existing source-reference
scripts pass. The hover oracle was rerun after the ordering correction. Hardware
smoothness and touchscreen acceptance are not established by offscreen captures.

## Actual box-edit routing measurements

The extended `public_chart --primitive-edit-profile` dispatches native Canvas
editing through ChartState and Chart::update, including primitive preparation,
labels and glyph conversion. Fixed visibility is 20..100, size 800x500 and ratio
1.25. Each case has 20 warmups and 100 samples. The debug acceptance runs below
had no competing build or capture. An earlier run overlapping cargo check is
retained in the raw log as diagnostic only and excluded from these comparisons.

| Run | 1,200 static median/p95 ms | 20,000 static median/p95 ms | 100,000 static median/p95 ms | 20,000 live median/p95 ms | 100,000 live median/p95 ms |
| --- | --- | --- | --- | --- | --- |
| 2 | 12.686 / 12.990 | 12.705 / 12.938 | 12.706 / 12.821 | 12.919 / 13.225 | 13.471 / 13.674 |
| 3 | 13.022 / 13.189 | 13.071 / 13.302 | 13.081 / 13.247 | 13.242 / 13.484 | 13.809 / 14.035 |
| 4 | 12.989 / 13.429 | 12.986 / 13.128 | 13.022 / 13.154 | 13.171 / 13.426 | 13.695 / 13.870 |

Static median growth is at most 0.46%; p95 growth is at most 0.44% across the
three runs, below the prewritten 10% screen. Existing 20,000-row live replacement
is much faster than the previous 40.039 ms diagnostic median. Sixteen-source
live medians remain 32.066..32.693 ms because work still scales with source count.
These CPU debug measurements establish history behavior, not a GPU or 60 fps
guarantee. Release phase acceptance remains a separate repeated comparison.
Raw log: `target/preparation-captures/primitive-edit-timing.txt`.

## Incremental update design before implementation

Upstream DataLayer updates an existing shared point without reindexing time;
ChartApi invalidates affected views before applying time/row changes. Required
semantics are ordered data, original time/metadata preservation, whitespace
membership and base-index behavior, and synchronous crosshair recalculation.
The upstream full-row response is an incidental handoff, not a requirement to
copy history on every replacement.

Compare a separate fast public Chart-only replacement path against an explicit
incremental DataLayer handoff. Choose the handoff so model consumers share one
correct update path. Add `SeriesChanges::changed_index: Option<TimePointIndex>`:
None means full replacement; Some(index) means replace/remove that one row using
zero or one fulfilled rows. `PlotList::update_row(index, Option<Row>)` binary
searches and upserts/removes owned rows, preserving all other metadata without
cloning it. Series applies this handoff, and reports whether fulfilled membership
changed. Coordinator refreshes aggregate indexes only when membership changed
or a full/shared reindex occurred; same-membership replacement keeps them intact.
Genuine time insertion still follows the existing full reindex path.

DataLayer owns normalized membership; Series owns plotted rows; an update owns
its changed row. No callback/model borrow survives the handoff and no sharing
cycle is introduced. Invalid input must be checked before any mutation. Tests
must cover historical insertion at an existing shared point, whitespace/value
transitions, original metadata, stable history and crosshair/source consumers.

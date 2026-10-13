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

Implemented on 2026-10-13; automated checks complete, fresh physical acceptance
pending. Public annotations work on Line, Area, Baseline, Bar, Candlestick and
Histogram in independent panes. Marker projection/index caches track shared time,
series data and settled scale geometry; hover alone does not reindex or reproject
unchanged collections. Text-width and glyph caches are bounded to 256 entries each.
Drawing uses prepared paths; it performs no marker projection or text shaping.

Normal annotations draw after their own series' automatic/custom price lines and
before the next series. Above-series/top annotations use the corresponding top
passes. The overlap fixture checks an opaque marker over its own price line on
both backends. Low-level strokes without series metadata remain supported.

Stationary-pointer marker changes request a bounded redraw to reconcile the next
public event even when the snapped crosshair is unchanged. Native Canvas input
checks click identification before any draw, clearing a stationary hit, and
settlement without a continuing animation request. Full series/price-line hit
arbitration is intentionally not represented as complete.

Two source prerequisites were corrected: source pixel margins now add to configured
fractional scale padding, and flat autoscale ranges expand by five minimum moves
on each side (using relative-mode minimum move rules). Clearing markers restores
padding. The existing flat-range tests now assert the source-expanded bounds.

### Files

New production files:
`src/api/markers.rs`, `src/model/series_markers.rs`,
`src/model/chart_model/markers.rs`, `src/model/series/markers_pane_view.rs`,
`src/renderers/series_markers_renderer.rs`, `src/ui/series_annotations.rs`.

Modified production files:
`src/api/{mod,chart,interaction}.rs`,
`src/model/{mod,chart_model,series,time_scale,price_scale,pane}.rs`,
`src/model/chart_model/price_lines.rs`, `src/renderers/mod.rs`,
`src/ui/{mod,chart_frame,line_chart,multi_pane_frame,price_axis}.rs`.

Verification/demo files:
`tests/series_markers.rs`, `examples/public_chart.rs`,
`examples/public_chart/markers.rs`, `examples/public_chart/tests.rs`, `examples/public_api_profile.rs`,
`examples/series_markers_reference.rs`,
`scripts/check_series_markers_reference.cjs`.
Documentation: this report, `README.md`, `tasks/series-markers-plan.md`,
`tasks/series-markers-todo.md`.

### Behavioral tests and source comparison

Ten public integration tests cover empty text and alpha preservation, transactional validation and attachment
lifecycle; additive/disabled/restored margins; nearest-time/whitespace lookup,
shared-time insertions and historical updates; hidden/clear/reload output;
immutable retained frames; stacked positions; all six families; independent panes;
normal/log/percentage/indexed/inverted/manual scales at fractional ratios;
text-only and shape hits; overlap selection; native click input; stationary hover
clearing; and equal-total above/below margin swaps invalidating other collections.

A font-cache test measures 40,000 repeated label preparations (400 visible labels
across 100 frames): one width paragraph shape and one glyph-path shape for the
shared style, then one additional shape of each kind after changing font size.
These are test-only cache-miss counters, not a claim about every label in a live
application. Distinct strings/styles beyond the cache bound can reshape.

`node scripts/check_series_markers_reference.cjs` executes the actual transpiled
source placement, sizing, bitmap geometry and hit helpers: **980 placement/bitmap
cases and 84,700 shape-hit cases passed**, including 280 margin comparisons.
The fixture covers all six positions, stacking, inversion, extreme/fractional
spacing and negative/zero/fractional/large sizes. Source arrow hit orientation is
preserved even where it differs from the rendered arrow. Text uses Iced metrics;
public text-hit tests validate prepared bounds rather than browser font pixels.
All 14 comparison scripts passed; existing comparisons cover **29,235** cases.

The regression-test mutation experiment inverted marker autoscale enablement:
the margin test failed at the expected assertion; restoring it returned all ten
integration tests to green. No tests were skipped or quality thresholds lowered.

### Commands and rendering evidence

```sh
cargo fmt --check
cargo test --all-targets --quiet
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --examples
git diff --check
node scripts/check_series_markers_reference.cjs
cargo run --example public_chart -- --series-markers --smoke
cargo run --example public_chart -- --series-markers --marker-captures-only --capture-dir /tmp/marker-captures
cargo run --example public_chart -- --series-markers --marker-captures-only --capture-dir /tmp/marker-wgpu --wgpu
```

Formatting, all tests, all-target check, strict Clippy, example builds and diff
checks pass. The complete suite contains 433 tests (337 library, 21 existing public
API, 10 markers, 64 native-example interaction tests and one public demo layout test). Headless public smoke
passes for all six families. Bounded native startup with `--series-markers` ran
without errors until its intentional ten-second timeout.

The final matrix has **144 TinySkia and 144 WGPU captures**: six families × six
marker scenarios × ratios 1/1.25/1.5/2. Scenarios cover stacking/text, explicit price,
inversion, layers, hit after navigation, and marker-over-price-line ordering.
Earlier broad captures also exercised the existing demo scenarios. Contact sheets
were inspected for shapes/text/layers/clipping and fractional scaling; the final
overlap and stacking captures were inspected on both backends. Local evidence:
`/tmp/feather-series-markers/verified-{tiny-skia,wgpu}/`,
`/tmp/feather-series-markers/verified-overlaps.png`.
These temporary artifacts are not repository dependencies.

### Narrow-window hover regression

Physical testing reported the bottom of the chart jumping during hover in a
half-sized window. The demo's status text included the full marker payload and
wrapped to different line counts as pointer events changed. Iced consequently
changed the height left for the chart. An actual Iced widget-layout regression
reproduced a 20.8-pixel shift at 640 logical pixels wide before the fix.

The status widget now fills the available width without wrapping; long status
details can be clipped horizontally rather than changing the chart's bounds.
`cargo test --example public_chart hover_status_keeps_chart_bounds_stable_in_narrow_windows`
verifies the complete host view at widths 640/480/360 and height 480, passing bounds
updates and alternating marker/non-marker hover events through the public Chart
API. It checks real status identity changes and stable chart geometry. The test
failed before the fix and passes afterward. Marker engine behavior is unchanged;
fresh user-observed mouse/touchpad acceptance remains pending.

### Performance

Debug build, prepared model + frame + glyph paths, 917×870.80005 logical pixels,
ratio 1.25, 20 warm-up iterations and 100 measured iterations. Alternated two
runs of base `2718f18` and this implementation with identical 1,200-row empty-marker
zoom/append fixtures. Values below are the range of medians; p95 is the highest
observed across the two runs. This measures preparation, not GPU display latency.

| Family/workload | Base median ms | Current median ms | Base/current max p95 ms |
| --- | --- | --- | --- |
| Line zoom | 6.660–6.772 | 6.592–6.942 | 7.550 / 7.905 |
| Line append | 10.968–11.031 | 10.921–11.813 | 12.190 / 13.165 |
| Candlestick zoom | 10.333–10.430 | 10.326–10.410 | 11.338 / 11.544 |
| Candlestick append | 14.701–14.971 | 14.705–14.945 | 16.380 / 16.323 |
| Histogram zoom | 7.978–8.086 | 7.935–7.999 | 8.786 / 8.777 |
| Histogram append | 12.175–12.464 | 12.220–12.403 | 13.765 / 13.945 |

No broad empty-marker slowdown was observed; Line append varied upward in one run.
These local timings are evidence, not a hard performance guarantee.

Stress fixture: 20,000 rows, 20,000 markers, a 400-row visible range, shared
“Signal” text. Across the three families, marker hover medians were 16.157–18.908 ms
(p95 17.115–19.732), zoom 17.095–20.145 ms (p95 18.168–21.959), and appends
62.598–66.354 ms (p95 65.273–71.682). Appending invalidates time/series generations
and synchronously rebuilds the marker index; ordinary hover reuses it. This debug
stress case exceeds a 16.7 ms frame budget and is not claimed to sustain 60 fps.
Large-scale streaming remains an optimization opportunity; no async machinery was
introduced. Reproduce with:

```sh
cargo run --example public_api_profile
cargo run --example public_api_profile -- --rows 20000 --marker-count 20000 --hover
cargo run --example public_api_profile -- --rows 20000 --marker-count 20000
cargo run --example public_api_profile -- --rows 20000 --visible-rows 400 --hover
```

For the same 20,000-row/400-visible-row workload without markers, hover medians
were 7.888–10.593 ms (p95 8.783–11.645), zoom 8.715–11.447 ms (p95
9.748–12.472), and appends 34.174–37.228 ms (p95 35.719–39.466). Marker
preparation adds roughly 8 ms to hover/zoom and 28–30 ms to append in this stress
fixture. This comparison uses the current engine for both populated and empty
collections; only the 1,200-row comparison above uses the previous revision.
Raw local timing log: `/tmp/feather-series-markers/profile-results.txt`.

Local implementation checkpoints: `0a595a1`, `415617a`, `c13ad2e`, `4a7703e`.
GitHub push remains a separate user action.

### Physical acceptance

Run `cargo run --example public_chart -- --series-markers` and repeat with another
family flag. Check shape and text hover/click, crowded attachments, click versus
drag, plot zoom/pan, price-axis wheel/drag scaling, live append, style/layer changes,
marker replacement and clear. Repeat using mouse and touchpad. Record the observed
hardware/backend/scale factor and any defects; synthetic input cannot close this
gate. Earlier built-in-family hardware gates remain independently pending.

## Remaining dependencies

Full series/price-line hover arbitration, arbitrary custom primitives, arbitrary
horizontal behavior in the public Chart, touch/kinetic input, editable annotations
and physical acceptance remain separate. Source subscription cleanup quirks are
not reproduced; ownership releases data deterministically.

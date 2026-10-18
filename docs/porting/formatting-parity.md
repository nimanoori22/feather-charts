# Formatting parity (public parity milestone A)

Status: A0–A3 implemented; A4 automated gates passed; mouse acceptance pending. Baseline
`f18ad0a`. Tasks P0/A0–A4 in `tasks/public-parity-todo.md`.

## Source, consumers and required contract

Investigated local 5.2.1 PriceScale, Series formatter construction/last-value data,
PriceTickMarkBuilder, localization/formatter function contracts, and chart/pane
configuration. Bidirectional graph traces link batch formatting to tick rebuilding,
axis optimal-width measurement and drawing, scale mode/options changes and source
order changes. Sparse native trait edges required actual source reads. Coverage is
best effort; changed source metadata was checked against current bodies.

Actual source graphics consumers include tickmarks-custom-formatter (batch-selected
precision), custom-percentage-format and series-price-formatter. They require batch
output on the real axis as well as public formatter queries, localized crosshair/
last-value/price-line labels, and correct fallback through option changes. Source
series can be built-in or Custom; both use the same price format contract.

The frozen browser matrix covers 72 built-in/custom × four scale modes × nine
callback combinations. Normal/log use global price batch, otherwise global scalar,
otherwise series batch/scalar. Percentage uses the corresponding percentage hooks;
indexed-to-100 deliberately uses its own formatter. Absolute last-price formatting
uses global price hooks regardless of scale mode; percentage last-value text uses
global percentage hooks regardless of scale mode. Tick generation first produces
scalar labels/edge marks and then applies the final complete batch.

JS applyOptions with an undefined localization formatter does not clear it (merge
skips undefined). Native explicit Clear is an intentional extension; omitted patch
fields preserve settings. Do not claim undefined-clear parity.

## Alternatives and chosen design

1. Add owned scalar/batch operations to the existing source bridge and let PriceScale
   resolve global versus source formatting. Store an owned clone of the localization
   policy in each scale/pane, refreshed through ChartModel. This fits existing
   RefCell source borrowing and lazy marks without exposing borrowed trait objects.
2. Build a separate shared callback dispatcher or inject chart-wide callbacks into
   every Series formatter. This couples series-specific public formatters to chart
   localization (wrong source semantics), complicates indexed modes and duplicates
   ownership/invalidation across series.

Choose 1. Default bridge methods preserve existing source implementers. Callbacks
remain Rc, synchronous and invoked during preparation; no Send/Sync/Clone requirement
is added to custom views. No renderer/device creation occurs here. Prepared frames
retain resolved strings and do not call the current live formatting policy later.

Ownership: Chart owns ChartModel/TimeScale, panes/scales and series. Each scale owns
its localization value (Rc callback clones) and existing source adapters; adapters
own series handles but no pane/chart backreference. Source formatter retains both
scalar and optional batch callbacks. Policy replacement invalidates labels without
creating a strong ownership cycle. Host closures must not create cycles through
separately shared chart owners. User callbacks have the same panic boundary as
existing formatter callbacks; they do not receive mutable chart references.

Public boundary: preserve existing constructors; add a typed localization patch
with Unchanged/Set/Clear callback fields, a localization readback, and one owner
apply method. Patch equality compares Rc identity and strings, never invokes user
callbacks to compare output. Source/data behaviors refresh together; data keys are
not reinterpreted. No-op patches preserve gestures/events. Runtime formatting may
change layout allocation through the ordinary settlement path.

Batch formatters must return one label per input value. Rust cannot represent the
source's out-of-contract undefined entries as string labels. Enforce this callback
contract with a descriptive assertion, like a panic in the user callback itself,
rather than indexing unchecked, silently discarding output or inventing valid
labels. Extra/missing labels are programmer errors; tests cover this native boundary.

## API slice A1

```rust
trait PriceScaleDataSource {
    // existing scalar operation remains required
    fn format_tickmarks(&self, values: &[f64]) -> Vec<String>;
    // default implementation maps the scalar formatter
}
```

Series formatter queries stay series-specific. Pane adapters delegate batches to
their series formatter; they do not borrow the scale back. PriceScale owns mode/
localization dispatch and formats the final actual marks including edges. New
runtime patch signatures are finalized in A3 after the current source policy is
connected and tested.

## Evidence and status

Source fixtures: `formatting-source.json`, `formatting-states.json`.
Baseline manifest: `public-parity-implementation-baseline.json`; original audit
observations are retained separately. Target logs under `target/public-parity`.
All prior timing/device/drawing-order exceptions remain open independently.

## API slices A2–A3

```rust
pub struct LocalizationOptionsPatch<T> {
    pub locale: Option<String>,
    pub date_format: Option<String>,
    pub price_formatter: FieldPatch<PriceFormatterFn>,
    pub tickmarks_price_formatter: FieldPatch<TickmarksPriceFormatterFn>,
    pub percentage_formatter: FieldPatch<PercentageFormatterFn>,
    pub tickmarks_percentage_formatter: FieldPatch<TickmarksPercentageFormatterFn>,
    pub time_formatter: FieldPatch<TimeFormatterFn<T>>,
}
impl<B: ChartBehavior> Chart<B> {
    pub fn localization_options(&self) -> LocalizationOptions<B::Item>;
    pub fn apply_localization_options(&mut self, patch: LocalizationOptionsPatch<B::Item>);
}
```

A detached readback clones only strings and Rc handles; domain items and custom
views gain no Clone requirement. Applying a patch first builds the complete next
value, compares callback identity, then refreshes both behavior formatters and all
current scales in one synchronous owner operation. Future panes/overlays inherit
the same policy. An unchanged patch queues no redraw/events or gesture cancellation.
No formatting string is rejected: empty date formats and unknown locale strings
retain the existing behavior's fallback; milestone H defines supported locales.

YieldChartApi's real defaults use chart localization, not a per-series price-format
override. Move the native wrapper default there and remove its forced custom-series
price-format replacement. This preserves explicit scalar/batch series options and
makes Clear reveal the underlying series formatter for existing/future series.
Default rendered yield text remains fixed three decimals followed by a percent sign.
This corrects public series formatter readbacks to source semantics; consumers that
need yield labels should query the owning scale, not the series-specific formatter.

## Core checkpoint (P0/A0–A2)

Implemented custom batch retention, built-in/custom source adapters, final-batch
axis labels, and chart-wide scalar/batch dispatch for price/percentage modes.
Current/future panes and named overlays inherit the owned policy. Built-in/custom
last-value text now uses the same percentage policy. Yield defaults moved to chart
localization and no longer overwrite explicit custom-series price formats.

Validation: first batch and global regressions failed on the expected ignored
callbacks before fixes; yield readback failed on the misplaced default before its
fix. All 72 browser/native matrix cases agree. A1 full all-target tests, all-target
check and all-target Clippy passed; the subsequent A2 library, formatting and yield
suites pass. Exact commands/logs are retained under target/public-parity/formatting.
Self-review covered dispatch precedence, custom callback cardinality, ownership,
cache invalidation and unchanged default labels; no new dependencies or devices.
Runtime update tests, actual renderer captures, full unchanged compatibility replay
and physical acceptance are still required at A3/A4. No milestone acceptance claim.

## Runtime checkpoint (A3)

Added LocalizationOptionsPatch, detached localization_options readback and synchronous
apply_localization_options on generic Chart and YieldCurveChart. Both horizontal
behaviors and all scale caches refresh together. Empty/equal patches do nothing;
Rc pointer identity compares callbacks without running application code.

Runtime tests cover set/clear, actual axis labels, retained snapshot strings,
callback lifetime with retained readback, unchanged data, pending events and active
pan on empty/equal updates, plus percentage batch fallback and future named overlays.
The new API first failed compilation before implementation; all runtime tests then
pass. Library/formatting/yield tests and focused Clippy pass. Unknown locale/date
strings retain existing fallback; full locale parity remains milestone H.
No physical/render acceptance has been inferred from these tests.

## Demo and acceptance protocol (A4, in progress)

Run `cargo run --example pane_management -- --formatting --screenshot-export --live-feed`.
Add `--options`, `--yield-curve` or `--custom-domain` for the other three domains.
Set ICED_BACKEND=tiny-skia or wgpu explicitly for physical checks.

Price/Price ticks toggle chart-wide price callbacks; Percent/Percent ticks toggle
percentage callbacks. Scale mode cycles normal, logarithmic, percentage and indexed
on every pane. Indexed labels deliberately ignore chart-wide/series callbacks.
Series batch installs a shared-precision formatter on the blue line and custom
intervals. Clear formatters reveals the per-series/default fallback. On maturity
charts, Price starts enabled because yield formatting is a chart default; clearing
it intentionally reveals ordinary price text. Horizontal changes crosshair/item
labels; Locale changes locale/date format without interpreting stored keys again.

Try those controls at regular size, pane/series moves and Reset, then live-feed
zoom/pan, price-axis wheel/drag and box edits/Escape at regular, narrow and the earlier
problem size. Shrink/enlarge to check recovery. Capture/export a formatted chart,
change formatters, and verify that its existing preview remains unchanged; recapture
must show the new policy. Do not infer touch/high-DPI acceptance from mouse results.

`python scripts/capture_formatting.py <new-output-directory>` uses strict renderer
identities and captures four domains × six scenes × three sizes × four pixel ratios
on both renderers. Each image is replayed exactly; retained exports are rerendered
after replacing policy, resizing and dropping the owner. This supplements existing
unchanged protocols rather than replacing their baselines.

## Port report and migration

Source components: `src/model/{price-scale,series,price-tick-mark-builder,pane,
chart-model,localization-options,price-formatter-fn}.ts`,
`src/formatters/iprice-formatter.ts`, `src/api/{chart-api,yield-chart-api}.ts` in the
local Lightweight Charts tree. The source graphics consumers named above require
common batch precision, mode-specific chart localization and stable scalar fallback.
The browser fixture freezes the exact consumed source hashes, not only HEAD.

Required semantics are formatting precedence, final complete tick batches, shared
localization across scales and correct runtime cache refresh. TS object mutation,
manual destroy and undefined callbacks are incidental mechanisms. Native typed
patches use explicit Clear and Rc ownership; no JS object identity or callback
subscription machinery is reproduced. Identity comparison is used solely to avoid
invalidating on a replacement by the exact same native callback.

Production Rust files changed: `src/model/iprice_data_source.rs`,
`price_formatter_fn.rs`, `series.rs`, `series/projection.rs`, `price_scale.rs`,
`localization_options.rs`, `pane.rs`, `chart_model.rs`, `src/api/configuration.rs`,
`yield_curve.rs` and `yield_curve/custom_series.rs`. Tests:
`tests/formatting_parity.rs`, `tests/localization_runtime.rs` and the public consumer
in `examples/pane_management/{formatting,formatting_captures}.rs`, connected through
`examples/pane_management.rs` and its existing host adapter. No new dependency.

Usage:

```rust
use feather_charts::api::{Chart, LocalizationOptionsPatch};
use feather_charts::model::series_options::FieldPatch;
use std::rc::Rc;
let mut chart = Chart::default();
chart.apply_localization_options(LocalizationOptionsPatch {
    price_formatter: FieldPatch::Set(Rc::new(|price| format!("{:.3}", price.value()))),
    ..Default::default()
});
chart.apply_localization_options(LocalizationOptionsPatch {
    price_formatter: FieldPatch::Clear,
    ..Default::default()
});
```

Applications can keep existing constructors. Custom tick callbacks now actually
run and must return one label per input. Yield-specific display defaults now live
in chart localization; the series formatter correctly returns its own configured
format independently. A detached localization readback retains callback ownership;
drop it to release those handles. Prepared frames/exports retain resolved labels,
so old previews remain unchanged until the application explicitly captures again.

Remaining dependencies: A4 physical acceptance; milestones B–I add the other public
parity features. Locale labels outside existing behavior remain H's responsibility;
no claim of Intl completion is implied. Rendering, touch, drawing-order and earlier
timing exceptions retain their independent acceptance records.

## Automated acceptance results

807 tests across 81 suites and two doctests pass. Formatting, all-target check and
strict all-target Clippy pass. The final runtime tests additionally assert that
Clear refreshes actual prepared axis text and a changed callback preserves the
owner's active pan and pending data event. All 38 source comparisons pass; hashes
confirm all 37 existing source-check scripts remain unchanged.

Both actual renderers pass 576 screen replays and 576 retained exports. The unchanged
event/legacy, custom-domain, screenshot and attribution protocols pass, with all
2982 accepted output images matching their prior hashes. This includes the existing
96 exact legacy baseline comparisons. Current Rust sources and fixture hashes still
match the final formatting capture protocol; production/demo sources are unchanged
since capture. A later assertion-only test refinement passed focused tests/Clippy.
Representative software/GPU calendar, maturity, percentage and moved-session images
were visually inspected. Evidence: [formatting-evidence.json](formatting-evidence.json).

Four explicitly tiny-skia software demos are open for available-mouse acceptance.
Software and GPU physical checks are pending; A4 and milestone A remain unchecked.
The roadmap proceeds to series price-scale reassignment after those checks pass.
Local core checkpoints: `2343cc9`, `8cb4848`; no push was requested or performed.

## User acceptance and progression

The user replied “it's okay, let's go for the next feature” after receiving the
software mouse acceptance procedure. This accepts formatting and explicitly
authorizes milestone B. It is a general acceptance statement; no separate GPU mouse
result or itemized device result is inferred. A4 retains that missing physical
record. Automated evidence is unchanged. Later combined acceptance can supply the
missing backend record, as with the previous screenshot milestone.

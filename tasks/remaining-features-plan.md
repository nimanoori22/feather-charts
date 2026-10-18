# Screenshot export, custom horizontal factories and attribution

Date: 2026-10-18. Status: screenshot accepted by user; implementing custom horizontal factories.
Baseline: `23476f1`. Checklist: [remaining-features-todo.md](remaining-features-todo.md).
Use the existing feature-specific plan convention; preserve earlier plans and
their independent acceptance records.

## Outcome and scope

Deliver the three remaining features in this order:

1. Reusable screenshot/export API for calendar, strike and maturity charts.
2. Public construction of charts with application-defined horizontal behavior.
3. Native TradingView attribution-logo rendering and host-controlled link activation.

Each milestone must work, pass its own gates and receive a local checkpoint before
the next starts. These features do not establish exhaustive parity with every source
option. Existing timing, dedicated drawing-order mouse acceptance and unavailable
touchpad/touchscreen/high-DPI physical checks remain open independently.

The planning phase is complete and implementation is authorized. No new tool installation
is anticipated. Do not add Tokio, engine async channels or thread-safe ownership.

## Preliminary investigation

Planning used graph discovery, traces, exact snippets and source reads. Source
ChartApi.takeScreenshot graph call edges were empty; actual bodies and literal
callers/tests were inspected instead. Coverage was best effort. Source horizontal
behavior/layout interfaces had changed metadata and were read directly; excluded
coverage tests were also read directly. Complete source/consumer audits and real
source probes are required at each contract checkpoint before production edits.

| Local source / native component | Finding that constrains implementation |
|---|---|
| TS `src/api/chart-api.ts`, `ichart-api.ts` | takeScreenshot defaults both addTopLayer and includeCrosshair to false; temporarily hiding the live crosshair is an incidental JS technique |
| TS `src/gui/chart-widget.ts` | Screenshot flushes pending drawing, then composes panes, separators, axes and corner stubs using bitmap dimensions |
| TS pane/price-axis/time-axis drawBitmap | Top canvas inclusion applies to every corresponding chart region, not only the plot |
| TS screenshot and panes coverage cases; graphics screenshoter helper | Exports are used as complete chart images and regression artifacts |
| Rust `src/ui/line_chart.rs`, `chart_frame.rs`, `multi_pane_frame.rs` | Owned frames and real renderer capture paths already exist; crosshair drawing currently groups other annotations/top primitives, so filtering cannot simply skip that helper |
| Exact Iced `core/src/renderer.rs`, `renderer/src/fallback.rs` | Headless construction returns a Future; screenshot readback is synchronous RGBA. Explicit backend selection must detect fallback |
| TS `src/api/create-chart.ts`, `model/ihorz-scale-behavior.ts` | createChartEx accepts user behavior; conversion, ordering, cache keys, labels and weights must remain coherent |
| Rust `src/api/chart.rs`, `behavior.rs`, `options_chart.rs` | Chart is generic but with_behavior is crate-private. Separate data/scale behaviors and domain-specific ChartError mappings need a public construction contract |
| Rust `ChartModel.configure_horizontal_behavior` | Existing option factories build equivalent options twice without Clone; changing time-key interpretation requires new data |
| TS `src/gui/attribution-logo-widget.ts`, PaneWidget.setState | Fixed SVG is 35x19 logical pixels, offset 10px from left/bottom; last pane owns it; theme uses text-color grayscale >160 |
| TS logo DOM element versus screenshot canvas traversal | The DOM logo is absent from takeScreenshot, even when top canvas is included |
| Rust layout/defaults/configuration/colors | attribution_logo exists and native defaults currently disable it; grayscale parsing already exists; drawing and runtime patch support are missing |

## Architecture and native policies to freeze

### A. Screenshot/export

Reuse prepared immutable frames and the installed Iced backends. Separate capture
of chart content from image rasterization and file encoding. Preserve base/top
composition, crosshair lines/markers/axis labels, price lines, custom renderers,
watermarks, pulse, marker z-orders, primitives, pane labels and separators according
to their actual source canvas placement. A primitive's z-order named Top is not
automatically equivalent to the source's separate top canvas.

Run all four source flag combinations. Do not assume four distinct images:
includeCrosshair permits crosshair content but does not force a top canvas to be
included. Freeze the actual relationship using a positioned crosshair and marked
base/top content. Native normal on-screen drawing must retain its existing order.

Preferred API separates these ownership boundaries (names remain provisional):

```rust
pub struct ScreenshotLayers {
    pub include_top_layer: bool,
    pub include_crosshair: bool,
}
pub struct ChartScreenshot { /* owned prepared chart frame, no live owner */ }
pub struct ScreenshotImage { /* checked pixel dimensions and owned RGBA bytes */ }
pub struct ScreenshotRasterOptions { /* ratio, optional output canvas, background */ }
pub struct ChartExporter { /* reusable offscreen renderer and font resources */ }

impl<B: ChartBehavior> Chart<B> {
    pub fn screenshot_of_prepared(
        &self, layers: ScreenshotLayers,
    ) -> Result<ChartScreenshot, ScreenshotError>;
    pub fn prepare_screenshot(
        &mut self, layers: ScreenshotLayers, now: Instant,
    ) -> Result<ChartScreenshot, ChartError>;
}
// ChartExporter construction may await Iced device initialization once.
// Rendering/readback uses &mut ChartExporter and returns an owned ScreenshotImage.
```

The prepared method captures the last successful displayed frame, including when
new model data is pending; name/document this explicitly. Before any successful
preparation, or after failed preparation cleared the display, return a typed error.
The fresh method uses the ordinary current-viewport preparation/settlement path,
including yield support, and preserves pending events for the host. It must not
drain events or silently restore stale model state after a failure. Its ordinary
preparation may emit legitimate range/size events and sample animations/expiry.
The source flushes pending work internally; these explicit native boundaries are
deliberate. Freeze exact fresh/prepared semantics in A0 before adding methods.

Filter drawing without patching live crosshair options or changing gesture state.
Exported snapshots retain their sampled pulse/expiry/hover appearance after later
updates, removal and chart drop, subject to the existing custom-renderer snapshot
contract. Do not add Clone/Send/Sync requirements to custom views.

Output sizing is raster sizing of the captured layout. Default uses its prepared
logical dimensions and requested pixel ratio. An explicit pixel canvas fits the
whole chart uniformly with padding; it does not stretch labels or reflow panes.
Responsive layout at a different logical size is separate from image resampling;
do not run a temporary resize on the live chart. Freeze rounding, transparent
padding/background and ratio/output-size precedence in A0. Return dimensions with
the buffer. Validate finite positive ratios, nonzero dimensions, checked width ×
height × 4, a documented allocation limit and backend texture limits before work.

Exporter initialization may use Iced's existing Future through the host runtime;
never block on device creation or initialize a device per input/frame. An optional
host task may schedule export, but the model/render hot path remains synchronous.
Reuse renderer/font resources across exports and document required font loading.
Strictly requested tiny-skia/wgpu must report unavailability instead of silently
claiming the other backend. Propagate initialization/readback failures as far as
the exact Iced API permits; document any lower-level panic boundary found in A0.

Core output is RGBA with explicit row/channel/alpha/color-space semantics verified
against both actual renderers. File paths, dialogs and encoding belong to the host.
Use existing raw-image/PPM support and an RGBA preview in the demo. PNG convenience
is optional follow-up, not a reason to add a codec/runtime to the engine. SVG/PDF,
video, clipboard integration and capture of surrounding application controls are
outside this milestone.

Alternative: screenshot the whole native application window. It captures unrelated
controls and does not provide chart layers or reusable offscreen rendering.
Alternative: construct a renderer inside every Chart/export call. It adds expensive
resource creation and ties model ownership to GPU lifetime. Prefer owned snapshots
plus a reusable exporter. Snapshot → owned frame; exporter → renderer; returned
image → bytes. No component retains its originating Chart.

### B. Public custom horizontal-scale construction

Expose the existing generic owner through an additive public factory. Preserve
Chart::new/default, OptionsChart and YieldCurveChart constructors and invariants.
Test with an external integration-test behavior using a distinct application item
and internal key, not a disguised timestamp/strike. Include typed custom series.

Preferred provisional shape:

```rust
impl<B: ChartBehavior> Chart<B> {
    pub fn with_custom_behavior(
        make_behavior: impl FnMut() -> B,
        chart_options: ChartModelOptions,
        scale_options: HorzScaleOptions,
        localization: LocalizationOptions<B::Item>,
    ) -> Result<Self, ChartError>;
}
```

The factory creates two equivalent, independently mutable behavior instances for
the existing data and scale owners. Invoke it during construction, then drop it;
do not store an owner-capturing closure. Equivalence of ordering/conversion options
and stable key/cache-key contracts must be documented. Test instance isolation and
exact initialization/update hooks. Generic horizontal domains occupy logical
indices; this does not add a continuous numeric-distance axis.

Compare this with requiring B: Clone and exposing a two-instance constructor.
Clone is unnecessary for existing callback-bearing behaviors and may alias mutable
caches; two public instances let callers accidentally choose incompatible policies.
Prefer the existing factory pattern, after the B0 consumer/ownership review.

Audit external conversion/data errors before exposing this constructor. Preserve
owned diagnostic causes through an additive ChartError path if current variants
cannot represent user behavior errors. Avoid requiring users to masquerade as Time
or OptionsChart errors. Do not make ChartError generic throughout existing callers,
add Send/Sync bounds without need, or stringify errors silently. Test a consumer
defined outside the library crate so crate-private access cannot conceal gaps.

Reuse preprocess/converter, key/cache, formatting, tick reset/weighting, canonical
queries, events, primitives, navigation and export paths. Start with formatting/
weight option updates that preserve key interpretation; if a public update entry
point is needed, reuse the existing equivalent-options factory and invalidation.
Do not promise changing Item type or reinterpreting populated keys in place.
Freeze that restriction and the required clear/reload/new-chart flow in B0.
Dynamic behavior registries, type-erased runtime domain switching and new market
data domains built into the engine are outside scope.

### C. Attribution logo

Use the exact local source vector geometry, colors and logical positioning. Reuse
native Canvas paths/color parsing instead of adding a general SVG parser or an
image-generation step. Prepare/cache geometry; drawing does not parse SVG every
frame. Clip both drawing and hit bounds to the current last pane, including tiny,
empty and single-pane layouts. Pane move/removal transfers the logo to the new
last pane without changing autoscaling, margins, data hits or frame ownership.

Keep the existing native default off to preserve compatibility. Honor explicit
construction configuration and add a typed runtime layout patch/readback if useful
for the demo; record runtime toggling as a native extension where the source does
not support it. Do not silently change the source/native default discrepancy.
Theme follows actual text-color grayscale, not a guessed dark background.

Treat activation as a widget/host action: emit a typed attribution activation event
containing the fixed destination, and let the host choose to open it. Do not launch
processes/network requests from model/drawing code. Source URL hostname/path UTM
tracking has no required native equivalent. Freeze event/default-interest behavior
in C0; update exhaustive ChartEvent consumers and migration notes if adding a variant.
Visible logo hit capture precedes plot click/drag/edit recognition; it must not
create a plot double-click, start an edit or intercept nearby axes/dividers.

Keep logo rendering separate from exportable canvas content: default exports omit
it, matching the source DOM logo. On-screen charts include it when enabled. An
include-logo export extension is not part of this plan. Preserve existing source
asset provenance and notices without making new licensing conclusions.

Alternative: implement the logo as a generic pane watermark. This gives it the
wrong last-pane ownership, hit priority and screenshot inclusion. Prefer a small
screen-decoration renderer plus an explicit host activation event.

## Ordered implementation tasks

Each task has at most three completion conditions. Run focused behavioral tests
for each code slice, then check all targets. Split any slice that grows beyond
about five production/test files. Source/design checkpoints are reviews of concrete
evidence; no additional user design approval is needed unless evidence changes scope.

### A0 — Freeze screenshot contract (M; no dependency)

Read complete relevant source components/dependencies and trace screenshot/layer
consumers, including graphics test helpers and exact Iced readback. Freeze real
browser observations, hashes, defaults, all four flag combinations, sizing/alpha,
fresh-versus-prepared behavior, limits/errors and allocation policy before edits.

Acceptance: reproducible source probe; reviewed layer/API/ownership map; baseline
763 tests/34 source scripts and retained 96-image compatibility provenance pinned.
Verify: source-only probe and baseline inventory. Likely files: new export port
report, source/results JSON and `scripts/check_screenshot_reference.cjs`.

### A1 — Separate plot layer selection (M; depends A0)

Acceptance: source-classified series/annotations/pulse/primitives filter correctly;
crosshair controls do not hide unrelated layers; ordinary default drawing unchanged.
Verify: compiling behavioral red/green tests and same-backend pixel comparisons.
Likely files: line_chart.rs, its series/primitive drawing helper, export-layer tests.

### A2 — Complete chart-region composition (M; depends A1)

Acceptance: price/time crosshair labels follow policy; panes/axes/stubs/separators
compose correctly; retained frames/default on-screen pixels remain unchanged.
Verify: multi-pane/hidden-axis/short-height tests and four source flag cases.
Likely files: chart_frame.rs, multi_pane_frame.rs, price/time axis draw helpers/tests.

Checkpoint A-layout: review layer semantics and unchanged screen output; local commit.

### A3 — Expose owned screenshot capture (M; depends A2)

Acceptance: prepared/fresh methods work in all three frontends; invalid/failed
preparation preserves errors/events; snapshots survive mutation/removal/drop.
Verify: public API/lifecycle/edit/animation/event tests with no real-time sleeps.
Likely files: new api/screenshot.rs, mod.rs, chart.rs, yield_curve.rs, export tests.

### A4 — Implement reusable raster exporter (M; depends A3)

Acceptance: checked RGBA dimensions/alpha/sizing contract; reusable actual backend
and fonts; typed failures/limits without implicit per-export device construction.
Verify: actual tiny-skia/wgpu readback, repeated export and invalid-size cases.
Likely files: new export module, its error/image types, raster tests and example host.

### A5 — Deliver application preview and export controls (M; depends A4)

Acceptance: preview/saving uses the public API; layer/ratio/output-size controls
work in calendar/strike/maturity; feed/gestures and event consumption stay correct.
Verify: generic host tests and native preview inspection. Likely files: example
export submodule, common host, demo entry, README and example-only image writer.

### A6 — Accept screenshot milestone (M; depends A5)

Acceptance: source and immutable-image matrix passes; all previous comparisons
remain unchanged; software/GPU mouse and export-file/preview results recorded.
Verify: frozen sizes 800x500/300x300/300x90 and DPR 1/1.25/1.5/2 across three modes,
four layer flags and both asserted backends; freeze exact scene/count matrix in A0.
Check live updates, pulse/expiry, custom data, edits, resize/recovery and later
snapshot reuse. Likely files: capture runner, evidence/report/checklist. Local commit.

### B0 — Freeze public behavior construction/error contract (M; depends A6)

Acceptance: complete createChartEx/interface/consumer audit and source probe;
review factory-versus-Clone/error alternatives; freeze initialization, cache/key,
configuration and lifetime rules with an external consumer design.
Verify: public construction/error red tests and source observations before edits.
Likely files: new custom-horizontal report/source fixture/oracle and integration test.

### B1 — Enable external construction and error propagation (M; depends B0)

Acceptance: external non-Clone behavior constructs a chart; conversion/data failures
retain useful owned causes; built-in constructors and existing signatures work.
Verify: external integration tests, invalid geometry/options and backend-free model
checks. Likely files: api/behavior.rs, chart.rs, mod.rs and public factory/error tests.

### B2 — Complete custom-domain behavior flow (M; depends B1)

Acceptance: original items, ordering/weights/cache invalidation and data/query/events
are correct; custom rendering/primitives/navigation/export work; empty/history/pop/
reindexing and invalid operations preserve the owner contract.
Verify: a genuinely distinct item/internal-key behavior, default full-context append
fallback, opt-in append fast path and snapshot lifetime tests. Likely files: focused
public tests, generic custom-data/event support and only proven missing owner hooks.

Checkpoint B-public: review external usability/error ownership and built-in parity;
local commit. No generic infrastructure beyond the demonstrated consumer.

### B3 — Deliver custom-domain demo (M; depends B2)

Acceptance: application-defined domain uses no private internals; runtime formatting
and typed readouts work; data controls, panes/annotations and export remain usable.
Verify: generic controls and software/GPU smoke captures. Likely files: one new demo,
behavior/view support, README and external-behavior docs/example.

### B4 — Accept behavior milestone (M; depends B3)

Acceptance: source factory/formatting/weight cases pass; three existing modes and
their exact compatibility checks pass; both backend mouse/export checks pass for
the custom domain. Verify: freeze keys/gaps/replacement/formatting/empty scenes at
the same sizes/DPRs before final replay. Likely files: captures/oracle/evidence and
report/checklist. Local commit.

### C0 — Freeze attribution contract (M; depends B4)

Acceptance: complete widget/placement/theme/hit/source lifecycle audit; source DOM
and screenshot observations pinned; native default, runtime patch, activation event
and export-exclusion decisions reviewed.
Verify: real source DOM bounds/theme/last-pane and logo-on/off screenshot probes.
Likely files: new attribution report/source fixture/oracle and renderer/input red tests.

### C1 — Render and configure native logo (M; depends C0)

Acceptance: exact vector/theme/placement contract; enabled visibility and runtime
readback work without autoscale/layout effects; disabled pixels remain identical.
Verify: small/empty/fractional-DPR/last-pane transitions and immutable snapshots.
Likely files: logo renderer, screen-frame integration, layout patch and tests.

### C2 — Route activation with correct input priority (M; depends C1)

Acceptance: logo click/tap produces only the chosen host action; drag/outside/focus/
pane changes cancel correctly; nearby plots/axes/edit/divider behavior is preserved.
Verify: deterministic Canvas/host-event tests and exhaustive-caller compilation.
Likely files: widget/focused hit helper, interaction event enum, caller migration/tests.
Split caller migrations if the enum reaches more than five independent files.

Checkpoint C-input: review event ownership/defaults and last-pane lifecycle; local commit.

### C3 — Accept attribution milestone (M; depends C2)

Acceptance: user-visible toggle/theme/link-action demo works; source/default export
exclusion and old disabled baselines pass; both backend mouse checks pass.
Verify: enabled/disabled light/dark, reorder/removal and tiny-size recovery across
existing/custom modes; host link action is tested without automatically opening
external URLs. Likely files: demo, capture/oracle/evidence, report and README.

### D0 — Final combined regression and documentation (M; depends C3)

Acceptance: all three features work together; final validation/evidence/port reports
are complete; local commits leave a clean worktree, with no push unless requested.
Verify: cargo fmt --check, cargo test --all-targets, cargo test --doc,
cargo check --all-targets, strict all-target Clippy and git diff --check. Run all
34 baseline source comparisons unchanged plus each new milestone oracle. Preserve
previous raster thresholds/baselines. Run heavy builds/source probes before manual
smoothness checks so background load does not contaminate observations.

## Risks and completion gates

| Risk | Mitigation |
|---|---|
| Screenshot flags drop annotations or leave crosshair labels | Source canvas classification and four flag probes; separate plot and axis filters |
| Export changes the live viewport/events or freezes stale data silently | Explicit prepared/fresh APIs; never resize the live owner for raster output; queue/gesture/error tests |
| Huge images, renderer fallback or incorrect alpha/channel order | Checked limits/byte dimensions, backend identity assertions and real readback tests |
| Custom construction exposes traits but no usable errors/state synchronization | External-crate consumer first, two-instance factory contract and typed error-cause tests |
| Custom labels/weights assume time or numeric public items | Distinct public/internal domain, append/reset/cache and original-item tests |
| Attribution appears on every pane, steals gestures or leaks into screenshots | Last-pane ownership, clipped input priority, source DOM/export exclusion tests |
| New checks overwrite old failures or accept unrelated milestones | Retain every failed run and unchanged thresholds; separate physical/timing records |

Scope can be finalized from source evidence and normal implementation judgment;
no unresolved preference requires a user answer now. If an exact Iced limitation
changes the promised output contract or requires substantial new dependencies,
document the concrete constraint and viable alternatives before proceeding.

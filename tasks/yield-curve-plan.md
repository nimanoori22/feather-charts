# Yield-curve chart implementation plan

Date: 2026-10-17. Status: complete for planned scope; automated and software/GPU mouse acceptance pass.
Rust baseline: `24bd3ba` (up/down marker implementation and mouse acceptance).
Execution checklist: [yield-curve-todo.md](yield-curve-todo.md).

## Outcome and scope

Provide a public numeric-maturity chart for Line and Area curves, source-compatible
maturity spacing, month/year labels, yield formatting, multiple curves, data
replacement/latest/historical updates and interactive navigation. Extend up/down
markers to numeric maturities and provide a working yield-curve demo.

Preserve existing date/time Chart callers, custom series, markers, primitives,
watermarks, pulse, interactions and owned frozen frames. Keep the engine synchronous.
Use existing generic model/data/scale and native drawing paths. No Tokio, new locks,
new renderer dependency, numeric values disguised as timestamps, or second chart
engine. Price-based/options charts, arbitrary user-defined chart families, numeric
custom-series rendering, touch annotation editing and new drawing tools are outside
this milestone. Numeric Line/Area and their documented extensions are the promised
surface; do not claim complete createChartEx/custom-series parity.

Use the established feature-specific task files; leave older incomplete timing
plans untouched. Historical timing acceptance remains OPEN/deferred. Mouse checks
can be completed with the user; touchpad, touchscreen and physical high-DPI checks
remain separate until actually tested. No delegation or extra approval gates are
required. Checkpoints are implementation verification gates.

## Investigation and evidence

Upstream checkout revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0`.
That checkout has existing local modifications to shared dependencies. Before
implementation pin actual file hashes as well as revision; never reset that tree
or assume revision alone identifies executed source.

Read complete source components:

- `src/api/create-yield-curve-chart.ts`, `yield-chart-api.ts`, `iyield-chart-api.ts`.
- `src/model/yield-curve-horz-scale-behavior/yield-curve-horz-scale-behavior.ts`
  and `yield-curve-chart-options.ts`.
- `src/api/options/yield-curve-chart-options-defaults.ts`.

Real consumers read: graphics yield-curve fixture, coverage yield-curve fixture,
and `website/tutorials/demos/yield-curve-with-update-markers.js`. The graphics case
uses two sparse, curved Lines from 1 to 360 months. Coverage combines Area/Line,
historical replacement, extension to 420 months, whitespace updates and shorter
full resets. The tutorial uses baseResolution=12 and historical update markers.
These require a shared maturity index, multiple series, shape/style preservation,
correct growth/reset settlement and marker positions independent of calendar time.

Graph searches/traces resolve the factory, API and behavior. The graph records no
factory callers for the script consumers, so direct source searches/reads supply
that evidence. Coverage checks report metadata_match for the primary TS files,
graphics fixture and tutorial. The excluded coverage subtree was read directly.
Rust graph coverage reports metadata_match for the boundaries listed below; clean
metadata is a best-effort signal, not an exhaustive proof.

Existing Rust foundations: ChartModel<B>, DataLayer<B>, TimeScale<B>, frame-owner
preparation and formatted axis snapshots already support behavior-generic work.
The public Chart owns calendar-specialized model/data state. Its validation uses
calendar converters; PointerEvent uses Time; PrimitiveContext exposes a calendar
model; annotations, up/down markers and custom-series handles contain calendar
constraints. Axis drawing consumes owned strings/geometry and should not need a
new numeric renderer. Keep time-specific custom-series methods specialized rather
than generalizing every external interface in this milestone.

## Behavioral contract and source questions to freeze first

| Source behavior | Required implementation/test |
| --- | --- |
| Public maturity values are numbers representing months | A distinct validated numeric maturity type; no Unix timestamps or business-day conversion. Test finite negative, zero, fractional and ordinary positive values against actual source before selecting validation policy. Normalize signed zero for equality/key consistency. Reject NaN/infinity before mutation. |
| Defaults: resolution 1, minimum range 120, start 0 | Preserve executed defaults. Effective scaffold state uses max(0,start), max(0,minimum,lastIndex), max(1,resolution). These are absolute source numeric calculations, not comments interpreted as multiplication by resolution. |
| Scaffold is start+i*resolution for floor((end-start)/resolution)+1 entries | Preserve the ordered union of scaffold keys and real maturities. Establish off-grid/fractional/shifted-start spacing from source; do not promise a different continuous numeric projection. Include end<start and empty data. |
| A private whitespace-only Line creates horizontal positions | Keep support points out of autoscale, marker baselines, public series iteration/events and meaningful-data membership. Preserve any observed minimum-domain effect; hiding internal ownership is an intentional native API choice. |
| Whitespace regeneration is microtask-debounced and keyed by scaffold state | Coalesce at the existing synchronous mutation/prepare boundary. Prove settled output after batches matches flushed JS microtasks, without timer threads or recursive rebuilds. |
| Largest index changes during conversion/weighting, including scaffold data | Oracle full replacement, shorter resets, longest-series removal and empty transitions. Do not assume shrink semantics or copy a self-referential callback cycle. Record source quirks and explicit corrections separately. |
| Weight priority: divisibility by 120,60,36,12,6,3,1 months | Match weights 10,9,8,7,6,5,4, else 0; max-weight selection and formatted tick caches must remain correct after option/data changes. |
| Labels below 12 use M; larger values use years plus remainder | Cover 0M,3M,6M,1Y,1Y6M,10Y, fractional/negative cases and custom localization. The source reads localization.timeFormatter; yieldCurve.formatTime is declared but not read here. Do not add a second formatter by assumption. |
| Left price axis visible, right hidden; yield formatter toFixed(3)+'%' | Preserve defaults and explicit overrides; do not multiply 5.378 into 537.8%. Last-value labels and price lines default off for newly added curves. |
| Built-in additions reject everything except Line/Area | Use a typed curve-series options boundary or consistent errors before allocation. Source permits custom definitions at runtime; numeric custom renderers are explicitly deferred. |
| ignoreWhitespaceIndices defaults true | Crosshair/grid selection uses fulfilled curve points, not support-only maturities. Multiple curves with missing values, empty panes and visibility changes need source comparisons. |

The contract oracle must execute the actual TS behavior and whitespace helpers,
with controlled microtask flushing and recorded series data; include a real public
factory integration check when DOM-dependent defaults/lifecycle cannot be proven
with stubs. Existing browser source-runner support may be reused; no package/tool
installation is currently required. Investigate exact Iced/Flowsurface usage only
if a new runtime API is needed; this plan expects to reuse the present widget.

## Architecture choice and ownership

Compare these viable designs in the first implementation checkpoint:

1. **Shared behavior-generic chart core with typed frontends (preferred).** Reuse
   existing generic model/data/frame owners; retain the calendar Chart entry point
   through a compatible alias/facade. YieldCurveChart owns the same core specialized
   for numeric behavior plus private scaffold state. Static item types prevent
   calendar/numeric mixing, and hot paths need no runtime dispatch. Costs: associated
   error/key bounds and extension contexts must be carried through selected APIs.
2. **Separate yield frontend over generic model/frame owners.** Avoid changing the
   calendar public type and reuse navigation inputs/frame types. Smaller immediate
   public API impact, but duplicates mutation, preparation, ownership and scheduling
   glue unless a shared core is extracted. Reject copying Chart/widget logic.

Do not choose a universal per-row Time-or-Number enum or add a numeric variant to
Time: that permits invalid mixtures and adds branches throughout calendar code.
Select the smallest design that compiles both fronts without duplicated engine or
interaction loops. The compatibility spike must demonstrate it before a broad
refactor. A public ChartApi<B> is provisional, not permission to expose an arbitrary
chart-family plugin framework. Default type parameters/aliases should preserve
existing calendar type names, trait implementations and constructor inference.

```text
Time Chart facade  -> shared core<calendar behavior> -> model + data + attachments
YieldCurveChart    -> shared core<numeric behavior>  -> model + data + attachments
                  -> private scaffold state/support-series identity
Prepared frame    -> owned strings, paths and renderer payloads only
```

Core/model/data behaviors must receive consistent options; no mutable callback
back-reference between them. Chart owns scaffold dirty state and its private
support-series ID. Mutations mark work; settlement rebuilds only if effective
scaffold keys change, then prepares one coherent frame. Drop removes all private
state. Existing shared series handles retain current ownership, with no new Rc
cycle. Formatter closures use the established Rc<dyn Fn> mechanism and must not
strongly capture the chart owner. Old frames retain neither live numeric state nor
scaffold buffers. Removing a public series cannot remove the private support owner.

## Proposed API (to finalize at compatibility checkpoint)

```rust,ignore
pub struct Maturity(f64);
impl Maturity {
    pub fn from_months(months: f64) -> Result<Self, YieldCurveError>;
    pub fn months(self) -> f64;
}
pub struct YieldCurveOptions {
    pub base_resolution: f64,
    pub minimum_time_range: f64,
    pub start_time_range: f64,
}
pub struct YieldCurveChart { /* owns numeric core and private scaffold */ }
impl YieldCurveChart {
    pub fn new(options: YieldCurveOptions) -> Result<Self, YieldCurveError>;
    pub fn add_line_series(/* existing Line options and pane/scale placement */) -> Result<SeriesId, YieldCurveError>;
    pub fn add_area_series(/* existing Area options and pane/scale placement */) -> Result<SeriesId, YieldCurveError>;
    pub fn set_data(&mut self, id: SeriesId, rows: Vec<BuiltInSeriesDataItem<Maturity>>) -> Result<(), YieldCurveError>;
    pub fn update_data(&mut self, id: SeriesId, row: BuiltInSeriesDataItem<Maturity>, historical: bool) -> Result<(), YieldCurveError>;
    pub fn replace_yield_curve_options(&mut self, options: YieldCurveOptions) -> Result<(), YieldCurveError>;
    // Existing fit/range/navigation, pane/axis options, prepare/frame/element/update
    // operations map through shared code; events contain numeric maturities.
}
```

Use existing data items/options/IDs, not a duplicate yield row hierarchy. Finalize
constructor access to chart/localization overrides and exact errors alongside the
existing Rust option conventions. Preserve calendar ChartError patterns; avoid
forcing every time-chart user to handle a new generic error type. Give numeric
errors sufficient detail (bad maturity, invalid options, unsafe scaffold extent,
unsupported series, historical update rejection). Validate options, row order,
values, count arithmetic and resource bounds before changing canonical state.
Choose/document a finite scaffold point bound in the contract checkpoint, reusing
an existing policy if available; reject pathological allocations transactionally.
Do not add a new configuration subsystem for this limit.

Primitive/event/marker item generics should default to Time where needed to keep
existing implementations source-compatible. UpDownMarker<Maturity> and managed
numeric updates must use the behavior's numeric key, retaining the established
seed-relative comparisons and explicit Instant deadlines. Existing time-specific
custom renderer handles can remain on the calendar specialization. Pane watermarks
should share their existing preparation/drawing, not be reimplemented.

## Delivery sequence and gates

0. Pin contract, actual-source oracle, source hashes and time-chart pixel baselines.
1. Prove the generic boundary in small compiling calendar-compatible slices:
   behavior-neutral validation/key/error hooks; core/frame plumbing; typed events
   and widget host; extension contexts. Freeze the final API after this spike.
   **Gate A:** all current examples compile, calendar source comparisons and default
   pixels remain unchanged; no numeric feature is exposed through a half-refactor.
2. Implement numeric maturity behavior and scaffold state as pure tested units.
3. Deliver one public Line curve end to end, including private support ownership,
   defaults and source-compatible domain settlement. Then add Area/multiple curves,
   option replacement, historical/whitespace/append/reset/removal behavior.
   **Gate B:** Line/Area fixture sequences match settled TS output.
4. Complete numeric pointer/crosshair/grid behavior and reuse mouse/axis/pane input.
   Extend up/down markers and verify primitive/watermark/price-pulse coexistence.
   **Gate C:** interactive numeric chart and explicit-clock expiry work on both backends.
5. Add an isolated `yield_curve` example, frozen native fixtures and acceptance.
6. Full validation, compatibility review, report and recorded physical checks.
   **Gate D:** all promised feature/source/native checks pass; device/timing exceptions
   remain separately reported.

Task dependencies, file budgets and verification are in the linked checklist.
All shared changes are sequential. Split any task that exceeds about five files
into compiling prerequisite slices; do not use a giant generic refactor commit.

## Verification protocol

Before production edits freeze actual-source cases/counts and distinguish parity,
native validation and deliberate corrections. Required cases include defaults,
nondefault resolutions/starts/minima, exact scaffold endpoints, off-grid maturities,
negative/zero/fractional values, custom formatter, tick priorities, two-curve domain
growth/shrink/removal, empty/reset/whitespace transitions and multiple writes before
one settlement. Keep failures and traces; never weaken comparisons to get green.

Record a compiling behavioral RED for numeric visible output, numeric event payload,
scaffold-only crosshair exclusion and idle numeric marker expiry before each fix.
Unit/integration tests use explicit clocks; no sleep-based engine assertions.
Verify invalid writes preserve data, scaffold, markers, deadlines and old frames.
Scaffold work must be absent on hover, pan/zoom, pulse-only ticks and same-domain
value updates. Prove with deterministic work counts/storage identity; do not invent
another broad timing matrix. The software one-region damage boundary remains.

Native capture matrix: actual tiny-skia and wgpu, DPR 1/1.25/1.5/2, sizes
800x500/300x300/300x90, five scenes: sparse Line, two-curve Line+Area, custom domain/
formatting, numeric update markers, cleared/empty recovery = **120 feature images**.
Compare **24 unchanged calendar default images**, freezing binary/fixture/source/font
identity. Include mixed watermarks/primitive content in the marker scene. Draw frozen
frames twice and after mutating/removing owners. Inspect representative label,
spacing, clipping and overlay images. Compare semantic payloads to TS; native and
browser font pixels need not be identical. Native default-off pixels must be exact.

Mouse acceptance on explicitly selected ICED_BACKEND=tiny-skia and ICED_BACKEND=wgpu:
regular/narrow/earlier problem size; wheel zoom, diagonal pan, price-axis wheel/drag,
numeric crosshair labels and selection, curve controls/updates, marker idle expiry,
manual marker persistence, box editing if exposed, and short-height recovery.
Don't count the upstream noninteractive tutorial (scroll/scale disabled) as evidence
that the native interactive chart works. Keep physical device gaps explicit.

Final commands: cargo fmt --check; cargo test --all-targets; cargo check --all-targets;
cargo clippy --all-targets -- -D warnings; yield-curve actual-source oracle and affected
calendar axis/interaction/primitive/markers/up-down/pulse/conflation/watermark oracles.
Record checked code identity, counts and exact results. No push unless requested.

## Principal risks and limits

- Generic changes can break calendar constructors, external primitive traits, custom
  renderer handles or event types: preserve defaults and compile real callers early.
- Scaffold keys affect shared indices and apparent spacing: test union membership
  and settled coordinates rather than substituting a simpler continuous axis.
- Debounced source updates and self-influenced extent may have surprising reset
  semantics: oracle first; explicit native corrections need their own expectations.
- Invalid/huge domains can allocate excessive support data: validate bounded size
  before mutation; no history regeneration on ordinary interaction.
- Formatter/default overrides must not accidentally change price units or existing
  calendar formatting. Test both defaults and user overrides.

No blocking user questions are required for this plan. API details, exact oracle
counts and bounded-allocation policy are implementation checkpoint decisions backed
by source tests; they must be recorded before the production slice they govern.

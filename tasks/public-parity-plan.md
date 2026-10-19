# Public API and desktop parity implementation plan

Date: 2026-10-18. Status: milestones A–I and combined completion Z0 complete within scope; independent exceptions remain open. See [completion evidence](../docs/porting/public-parity-completion.md).
Baseline: `55ba7d7`. Task list: [public-parity-todo.md](public-parity-todo.md).
Evidence: [audit](../docs/porting/public-parity-audit.md),
[inventory](../docs/porting/public-parity-inventory.json),
[observations and hashes](../docs/porting/public-parity-evidence.json).

Use feature-specific files; preserve the completed remaining-features roadmap and
all earlier plans. This plan implements practical native behavior for all ten audit
groups. It does not claim that the audit enumerated every upstream defect or that
completing this plan establishes a measured 100% port-completion score.

## Deliverables and order

| Milestone | Audit findings | Application outcome |
|---|---|---|
| A — Formatting correctness | F01/F02 | Global price/percentage and custom batch tick formatting work, including runtime changes |
| B — Series price-scale assignment | F03 | Existing built-in/custom series move between left/right/overlay scales without losing identity or data |
| C — Horizontal-axis input and gesture settings | F04 | Drag/reset the horizontal axis; independently configure time/price axis gestures |
| D — Public scale, series and coordinate helpers | F06 | Applications query ranges/scales/annotations and implement navigation or synchronized crosshairs without private engine access |
| E — Pane pixel sizing | F07 | Applications set pane height and restore layouts predictably |
| F — Application metadata | F08 | Built-in rows retain useful application payloads through mutations, queries and events |
| G — Native pointer metadata | F10 | Applications can implement modifier-key clicks and distinguish native pointer sources |
| H — Default locale behavior | F09 | Supported default tick labels match source locale behavior, with explicit fallback limits |
| I — Configured color parsing | F05 | Configured parsers actually affect validation/preparation; unsupported gamut requests are explicit |
| Z — Combined acceptance | All | Final public consumer, source comparisons, rendering and available-device checks are recorded |

A/B are correctness fixes for options already exposed. C/D/E are the next desktop
usability work. F/G add application integration. H/I follow them; no automatic new
Unicode/color dependency or true wide-gamut renderer is assumed.

## Scope decisions

- Keep model/data/render hot paths synchronous. Do not add Tokio, threads, channels,
  Arc/Mutex or mutable engine access as a public escape hatch.
- Reuse existing Chart ownership, typed IDs/scale selectors, patches, immutable
  prepared frames, event queue and actual tiny-skia/wgpu adapters.
- Preserve default constructors, native default-off attribution, host-controlled
  link activation, screenshot layer rules and sRGB export semantics.
- Existing native validation may remain stricter than JS when explicitly documented.
  DOM nodes, ResizeObserver and JS callback identity are not native API requirements.
- No new tools are expected: Cargo, the existing local source oracle tooling and
  Chromium are sufficient. Reuse current profiling tools only if a measured
  regression calls for investigation. Do not install packages speculatively.
- True Display-P3 rendering is deferred to a separate renderer/host project. This
  plan must make its unsupported state explicit; F05's wide-gamut portion remains
  open unless end-to-end support and hardware acceptance are independently delivered.
- H0 defines the locale support contract before code. At minimum preserve existing
  en-US/fr-FR behavior and reproduce the audited de-DE/ar-EG cases, UTC boundaries,
  fallback and numbering-system behavior. A limited supported-locale set must be
  documented as partial Intl compatibility. Full Intl equivalence cannot be checked
  off by relabeling an ASCII or English fallback as a supported locale.

## Dependency and checkpoint policy

Execute sequentially: P0 → A → B → C → D → E → F → G → H → I → Z.
Some read-only fixtures could be prepared independently, but this plan does not
require parallel agents. A shared contract settles cross-cutting types first;
changes to overlapping owner/projection/widget modules stay sequential.

Every task below has an observable result and a focused verification. File lists
are likely responsibility boundaries, not permission to rewrite unrelated code.
Target a small/medium slice, normally no more than five production/test files;
if the concrete change exceeds that, split compiling adapters/migrations before
adding behavior. Do not leave stubs or inert successful options to keep a slice green.
Save a reviewed local checkpoint after each milestone. Push only on request.
A checkpoint reviews concrete code/evidence; it is not an extra design permission
request. User input is required only for actual new scope or physical acceptance
that cannot be automated.

## Source investigation required before each implementation milestone

Revisit the complete relevant TS components, imports and real consumers, including
source coverage/graphics cases. Use graph discovery/traces plus exact path coverage;
read current source directly for stale, partial or missing graph coverage. The local
source tree has pre-existing edits: preserve it and freeze hashes, not just Git HEAD.

Before production changes, write the required/incidental behavior distinction,
compare at least two viable Rust designs, choose ownership/lifecycle semantics,
show the proposed API, and map it back to real consumers. Freeze source observations
and add compiling source-derived failing tests. The existing diagnostic records
differences; a zero exit from it is not an acceptance gate by itself.

Source hotspots: price-scale/series/formatter contracts for A; Series.applyOptions,
Pane/ChartModel transfers for B; TimeAxisWidget/PriceAxisWidget/MouseEventHandler and
exact Iced input for C/G; public API implementations for D/E; DataLayer/data-consumer
and custom metadata consumers for F; Intl-backed time formatters for H; ColorParser,
layout options and actual native render/export boundaries for I. Consult exact Iced
and Flowsurface patterns where relevant, without treating the latter as specification.

## API and ownership direction

Names below are provisional until each source/design task. Add narrow owner methods
rather than retained scale wrappers or mutable model access. Reuse current types.

| Proposed operation | Required boundary |
|---|---|
| `apply_localization_options(patch)` | Typed patch with unchanged/set/clear callback semantics; price/tick/time policies update together |
| `set_series_price_scale(id, position)` / `series_price_scale(id)` | Checked current membership; left/right/named overlay; options and membership agree |
| `visible_horizontal_range()` / `visible_logical_range()` | Owned optional results; may resolve lazy caches through `&mut self`; no event draining |
| `set_visible_horizontal_range(range)` | Behavior conversion and nearest-point semantics; checked owner invalidation |
| `reset_time_scale()` / `scroll_to_real_time()` | Source-configured defaults/animation semantics, not a hard-coded zero offset |
| `price_lines(id)` / series readbacks | Owned IDs/values or scoped typed access; cover custom as well as built-in series |
| `price_to_coordinate(id, price)` / inverse | Actual current owning pane/scale and source first-value policy; None for unavailable conversion |
| `set_crosshair_at(id, item, price)` | Domain-based helper alongside existing pixel API; preserve synthetic-event policy |
| `set_pane_height(pane, height)` | Persistent layout allocation, not a temporary engine height overwritten on prepare |
| axis interaction settings | Independent time/price pressed-drag and double-click reset, separate from plot wheel/touch options |
| `PointerEvent.input: Option<PointerInput>` | Owned native modifiers/coordinates/source; None when there is no originating native pointer input |

Formatting: first carry scalar/batch formatting through existing traits/projections,
then resolve chart-wide versus per-series precedence at the scale. Prefer one coherent
owned formatter policy per chart/scale over unrelated callback plumbing. Prepared
frames own sampled text/geometry; exports must not call the current live formatter.
Reused callbacks can use existing Rc ownership; do not require Clone/Send/Sync for
custom views or call user code to test whether two settings are semantically equal.
Document strong/weak ownership and callback capture/reentrancy rules.

Scale transfers: Chart remains the mutation owner. Preflight options and destination,
then perform one logical membership update and invalidation. Reject invalid requests
before canceling valid gestures or mutating data. Keep ID, drawing order, annotations,
markers and custom view alive; empty-scale/pane cleanup follows source lifecycle.

Metadata: F0 compares `Chart<B, M = ()>` with a default metadata parameter against an
owned type-erased payload with typed access. The engine already has generic metadata;
the generic approach is the starting preference, but extension/wrapper/public event
costs must be verified before choosing it. Preserve unit-metadata callers. Avoid
mandatory deep copies of large payloads; an Rc payload must remain possible. Do not
use unsafe casts, require Send/Sync, or silently discard data. A decision cannot be
marked complete merely because a field exists in a lower-level generic type.

Colors: prefer one shared configured parsing policy at validation/preparation
boundaries, not a parser per draw call. Parser replacement, if offered, must have
clear cache invalidation and owned snapshot semantics. Convert supported custom colors
to the documented sRGB output contract. Color parsing of a Display-P3 string is not
proof that the render target itself is wide-gamut.

## Ordered tasks

S = small, M = medium. All task acceptance conditions are required; milestone
acceptance additionally applies the common gates below.

### Baseline

#### P0 — Freeze the implementation baseline and close the scope map.

Scope: M. Dependencies: None.

Acceptance:
- Pin native/source revisions and current hashes, all 37 existing source checks, capture protocols/counts and independent exception records.
- Map all ten findings to tasks, define native versus source-compatible behavior, and identify public migrations before production edits.
- Keep the audit inventory and raw diagnostic evidence intact; plan how resolved observations will become real failing/passing regressions.

Verification: Replay the twelve targeted observations; inspect the existing validation/capture manifests. Record results without treating known differences as a pass.

Likely files: Existing audit evidence/inventory, feature source fixtures, new parity implementation report and checklist.

### A — Formatting correctness

#### A0 — Freeze formatting precedence, batch and lifecycle contracts.

Scope: M. Dependencies: P0.

Acceptance:
- Record real source precedence and fallback for all four chart-wide callbacks, per-series formatters, modes and tick batches.
- Compare localized scale-policy versus distributed-hook designs; freeze callback ownership, clearing and cache invalidation.
- Add failing regressions for ignored callbacks using built-in and typed custom series, including actual axis text.

Verification: Run source-only formatting fixtures and focused red tests; include a callback whose batch output cannot be produced by scalar mapping.

Likely files: Source formatting oracle/fixtures, formatting contract report, focused public formatting tests.

#### A1 — Carry custom batch formatting through series and scale bridges.

Scope: M. Dependencies: A0.

Acceptance:
- Series formatters retain and use the optional batch callback, with source-correct scalar fallback.
- Built-in/custom projections and the price-source bridge carry batches to actual tick generation.
- Callback invocation/output-size handling and source-versus-native error policy are explicit; scalar labels remain unchanged.

Verification: Red/green custom formatter tests plus prepared price-axis tick snapshots; validate normal/log/percentage/indexed dispatch.

Likely files: model/series.rs, model/series/projection.rs, model/iprice_data_source.rs, model/pane.rs, focused formatter tests; split bridge changes if needed.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

#### A2 — Apply chart-wide formatting to price scales.

Scope: M. Dependencies: A1.

Acceptance:
- Global price/percentage and corresponding tick callbacks use the frozen precedence and fallback in every label/tick path.
- Current/future panes, overlay scales and custom-series leaders share the correct policy, including YieldCurve defaults.
- Formatting performs no renderer initialization or extra data cloning and preserves immutable prepared labels.

Verification: Source/native precedence cases, multi-pane and scale-mode tests, retained-snapshot text checks.

Likely files: model/price_scale.rs, chart-model formatting integration, localization policy module, yield wrapper integration and tests.

#### A3 — Expose atomic runtime localization updates.

Scope: M. Dependencies: A2.

Acceptance:
- Typed updates support unchanged/set/clear fields and useful readback while preserving existing constructors.
- Time behavior, scale labels and cached ticks update together without reinterpreting populated keys.
- Empty/equal patches, invalid operations, pending events and active gestures follow the frozen native contract.

Verification: Public API update/clear/fallback tests, data-preservation assertions and callback/snapshot lifetime checks.

Likely files: api/configuration.rs, model/chart_configuration.rs, chart-model configuration, behavior formatter integration and tests.

#### A4 — Deliver formatting controls and accept milestone A.

Scope: M. Dependencies: A3.

Acceptance:
- An external public-API demo changes formats at runtime across calendar, strike, maturity and sessions with live data.
- Both actual renderer captures verify labels, crosshair, overlays and old retained exports; available mouse/preview checks pass.
- F01/F02 observations now assert parity, inventory entries cite evidence, and a reviewed local checkpoint is saved.

Verification: Apply common acceptance gates, including software/GPU controls and full/narrow/short-height recovery.

Likely files: Shared demo formatting module, capture scenes, README, formatting evidence/report/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### B — Series price-scale assignment

#### B0 — Freeze series-to-scale transfer semantics.

Scope: M. Dependencies: A4.

Acceptance:
- Audit source option routing, overlay creation/removal and owner lifecycle for built-in/custom series.
- Choose one mutation owner and freeze explicit-selector versus option precedence, no-op and failure behavior.
- Reproduce contradictory option/membership state in compiling failing public tests.

Verification: Source transfer fixtures and red tests for left/right/overlay destinations, absent destinations and invalid mixed options.

Likely files: Transfer source oracle/fixtures, scale-assignment contract and public tests.

#### B1 — Implement one checked scale-transfer operation.

Scope: M. Dependencies: B0.

Acceptance:
- Preflight and transfer preserve data/handles, order, annotations, custom views and source-specific formatting.
- Scale autoscale/manual state, default side, empty cleanup and affected panes follow the source contract.
- No-op/failure paths preserve valid state and gestures; successful transfers cancel stale captures once.

Verification: Engine/public transfer regressions, weak/strong lifecycle tests and autoscale/visible-range snapshots.

Likely files: model/pane.rs, chart-model transfer module, api owner transfer method, tests; reuse existing pane machinery.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

#### B2 — Route public series options through scale assignment.

Scope: M. Dependencies: B1.

Acceptance:
- Built-in and custom option updates agree with actual membership; explicit setter and membership query use the same operation.
- Typed readbacks, stale/foreign selectors and updates after pane moves/removal behave consistently.
- Existing creation signatures remain usable and migration notes explain selector precedence.

Verification: Public consumer tests for both series families, repeated transfer/option changes and subsequent data mutation.

Likely files: api/chart.rs, api/custom_series.rs, api/pane_management.rs or a focused scale-assignment module, tests and migration docs.

#### B3 — Demonstrate scale assignment and accept milestone B.

Scope: M. Dependencies: B2.

Acceptance:
- Controls move built-in/custom series among left/right/overlay scales in all four chart modes.
- Annotations, crosshair, manual ranges and live feed remain correct through transfer, zoom, edits and export.
- Both-backend evidence passes; F03 is resolved with unchanged existing gates and a local checkpoint.

Verification: Common acceptance gates plus repeated transfers during live updates and small-window recovery.

Likely files: Demo assignment module, transfer captures, source assertions, report/evidence/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### C — Horizontal-axis input and settings

#### C0 — Freeze horizontal-axis input and independent-control behavior.

Scope: M. Dependencies: B3.

Acceptance:
- Read source time/price axis handlers and actual Iced input; freeze anchoring, drag/reset defaults and touch double-tap.
- Define independent time/price axis controls, preserving existing wheel/pinch/native price-axis wheel behavior.
- Add source-backed failing real-Canvas-event tests with ownership/priority and cancellation cases.

Verification: JS mouse gesture probe, synthetic native events and source control-flag fixtures.

Likely files: Axis input contract, source oracle, new horizontal-axis/controller tests.

#### C1 — Implement horizontal-axis drag and reset.

Scope: M. Dependencies: C0.

Acceptance:
- An owned synchronous input controller emits scale start/move/end/reset through the existing owner.
- Drag anchors and reset-to-configured-default behavior match source across all horizontal domains.
- Hidden/empty axes, release outside, focus loss, resize/DPR changes and pane mutations do not leave a stale gesture.

Verification: Red/green time-axis drag/double-click tests, active-controller state tests and range/spacing assertions.

Likely files: New ui/time_axis_input.rs, api/widget.rs, interaction routing, model owner bridge and tests.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

#### C2 — Expose and enforce independent axis settings.

Scope: M. Dependencies: C1.

Acceptance:
- Time/price axis drag and double-click reset are independently configurable with typed runtime readback.
- Disabling an active capability cancels its owned capture; unrelated plot, axis and edit gestures retain priority.
- Touch/double-tap routing follows source settings without synthesizing duplicate mouse actions.

Verification: Settings truth table, touch cancellation/synthesis tests and invalid/no-op patch checks.

Likely files: API input configuration, widget/price-axis controller, touch routing and focused tests; split adapters to keep slices small.

#### C3 — Demonstrate axis controls and accept milestone C.

Scope: M. Dependencies: C2.

Acceptance:
- Demo exposes horizontal drag/reset and independent flags in calendar/strike/maturity/sessions.
- Actual software/GPU mouse checks cover all three window sizes, disabled controls and short-height recovery.
- F04 parity assertions and compatibility gates pass; synthetic touch is recorded separately from physical acceptance.

Verification: Common acceptance gates and specific horizontal-axis physical checks; save a local checkpoint.

Likely files: Shared axis demo module, capture/event scenes, source assertions and evidence/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### D — Reusable public helpers

#### D0 — Add current range queries and horizontal navigation helpers.

Scope: M. Dependencies: C3.

Acceptance:
- Owned optional logical/horizontal queries resolve lazy state without draining events or mutating user options.
- Visible item-range setting, reset and real-time navigation reuse source algorithms and configured defaults.
- Empty/absent/out-of-range items, animation cancellation and original-item identity are consistent across domains.

Verification: Source/API range/action sequences, event-order tests, custom-domain and no-data cases.

Likely files: Focused api/time_scale module, model/time_scale.rs, owner forwarding, public tests.

#### D1 — Complete series and annotation readbacks.

Scope: M. Dependencies: D0.

Acceptance:
- Applications enumerate price-line IDs and read current pane/scale/type information without private projection access.
- Built-in and custom option/formatter/current-value access has explicit owned or scoped semantics without new Clone bounds on views.
- Removal, retained handles and transfer behavior are documented and tested; no mutable model escape hatch is added.

Verification: External-consumer compile/runtime tests, stale-selector and snapshot/readback ownership tests.

Likely files: Focused series-query API module, api/custom_series.rs, model projection read boundary, query tests and docs.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

#### D2 — Expose coordinate and domain-based crosshair helpers.

Scope: M. Dependencies: D1.

Acceptance:
- Price/coordinate conversion uses actual current series scale, first-value and logarithmic/percentage rules.
- Domain-based synthetic crosshair placement coexists with the pixel API and matches source event suppression policy.
- Queries and placements handle empty/hidden/overlay series, scale/pane moves and absent horizontal items explicitly.

Verification: Round trips with source fixtures, two charts synchronized through public events, and custom-domain cases.

Likely files: Series-query/conversion module, api/interaction.rs, model/crosshair bridge and public tests.

#### D3 — Validate an external consumer and accept milestone D.

Scope: M. Dependencies: D2.

Acceptance:
- An example uses only public helpers for range controls, live navigation, readouts and synchronized crosshairs.
- All four domains and typed custom series work after pane/scale changes and with exports.
- F06 entries are classified individually with evidence; unavailable readbacks are not marked resolved by name similarity.

Verification: Common gates, external example checks and both-backend control acceptance; save a local checkpoint.

Likely files: Public helper demo, external-consumer test, README and query evidence/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### E — Pane pixel sizing

#### E0 — Freeze persistent pane-height allocation semantics.

Scope: M. Dependencies: D3.

Acceptance:
- Audit source pixel-to-stretch allocation, minimums, neighboring panes and no-op behavior.
- Compare persistent layout allocation with a direct engine-height setter; document why the latter is overwritten on prepare.
- Add source-backed failing reusable API tests for requested height before/after prepare and resize.

Verification: Source height fixtures and red layout/public-API regressions.

Likely files: Pane sizing contract, source fixtures, focused layout/API tests.

#### E1 — Implement persistent programmatic pane height.

Scope: M. Dependencies: E0.

Acceptance:
- The owner converts requested height into persistent layout allocation, using the frozen neighbor/minimum rules.
- Finite validation, one-pane/empty/no-op behavior and gesture cancellation are explicit.
- Ordinary divider/window resize behavior is unchanged; pane move/removal retains coherent allocations.

Verification: Red/green allocation tests through prepare, resize, reorder and short-height recovery.

Likely files: Public pane-management method, layout allocation helper, model bridge and tests.

#### E2 — Demonstrate saved pane layouts and accept milestone E.

Scope: M. Dependencies: E1.

Acceptance:
- Controls set/restore heights and show actual settled sizes after pane move/swap/removal.
- Feed, axis interactions and box edits survive small-window collapse and recovery in both backends.
- F07 is resolved and source/renderer evidence plus a local checkpoint is recorded.

Verification: Common gates with multi-pane sizing scenes and live mouse checks.

Likely files: Pane sizing demo controls, captures, README and evidence/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### F — Application metadata

#### F0 — Choose and freeze the application metadata API.

Scope: M. Dependencies: E2.

Acceptance:
- Compare defaulted typed generics with owned type erasure against real built-in/custom/event/extension consumers.
- Freeze payload ownership, copying, equality/identity and compatibility for existing unit-metadata constructors and aliases.
- Add an external-consumer failing test using a real application payload; choose a design without unsafe or Send/Sync requirements.

Verification: Compile concrete consumer prototypes and trace mutation/query/pop/event/snapshot payload flow before production changes.

Likely files: Metadata design report, public consumer tests, minimal type prototypes/source fixtures.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

#### F1 — Prepare metadata-capable adapter boundaries without changing default behavior.

Scope: M. Dependencies: F0.

Acceptance:
- Implement the chosen representation in the minimum shared adapter/type boundaries while unit-metadata callers still build.
- Primitive/custom-renderer/event preparation boundaries can carry the chosen payload without unsafe casts or new Send/Sync requirements.
- F0 enumerates the exact compile-time migration steps; split each coupled boundary into a compiling subtask before feature routing if necessary.

Verification: External type-compatibility tests and full all-target compilation with unchanged unit-metadata behavior.

Likely files: Metadata adapter/type module, existing generic model bridge, primitive/custom context boundary and compatibility tests; no data behavior change yet.

#### F2 — Retain application metadata in built-in data operations.

Scope: M. Dependencies: F1.

Acceptance:
- Public built-in inputs accept the chosen payload and preserve it through set/update/history/pop/query/reindexing.
- Existing unit-data constructors and aliases remain usable; metadata identity/equality follows the frozen contract.
- Large payloads need no mandatory deep copies, and failed mutations preserve payload ownership.

Verification: Public application-record round trips, clone/drop counters, failure atomicity and large Rc payload tests.

Likely files: api owner/data module, data/model bridge, canonical query support and metadata integration tests.

#### F3 — Complete metadata event and chart-family adapters.

Scope: M. Dependencies: F2.

Acceptance:
- Owned pointer data includes application payloads with correct queue/drop semantics.
- Numeric/yield/custom-domain wrappers and custom series preserve their existing typed behavior and new metadata policy.
- Prepared snapshots, retained handles and removals satisfy the frozen ownership contract; all exhaustive consumers compile.

Verification: Public wrapper/custom-series/event tests, retained-snapshot/drop cases and all-target consumer compilation.

Likely files: Event payload adapter, numeric/yield wrapper adapters, custom-series integration and tests; split wrapper migrations into compiling sub-slices if required.

#### F4 — Demonstrate application metadata and accept milestone F.

Scope: M. Dependencies: F3.

Acceptance:
- An external example shows application IDs/tooltip payloads and reads removed/updated data through public APIs.
- All domains, annotations, interactions and exports continue to work; retained metadata lifetime is verified.
- F08 is resolved, exhaustive consumer migrations are documented and both-backend evidence/local checkpoint exist.

Verification: Common gates, external-consumer tests and payload/gesture checks at regular and narrow sizes.

Likely files: Metadata example/support, consumer migrations/tests, README and evidence/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### G — Native pointer metadata

#### G0 — Freeze native pointer metadata semantics.

Scope: M. Dependencies: F4.

Acceptance:
- Map source modifiers/coordinates to actual Iced keyboard/mouse/touch input and define typed ownership.
- Define chart-local coordinates, modifier freshness, available input source/button data and optional programmatic metadata.
- Plan exhaustive payload migrations and tests without retaining DOM/window/event references.

Verification: SourceEvent consumer audit, exact Iced event-read tests and a concrete Ctrl/Shift-click consumer.

Likely files: Pointer contract, source fixture, owned metadata types and API tests.

#### G1 — Populate owned pointer metadata at the widget boundary.

Scope: M. Dependencies: G0.

Acceptance:
- Click/double-click/crosshair notifications carry correct native data where it exists, including current modifiers.
- Touch-synthesized, queued and programmatic events have explicit deduplication/None semantics.
- No lifetime-dependent event references, synthetic defaults disguised as measured data or changes to unrelated input capture.

Verification: Synthetic Iced event sequences, focus/modifier transitions, touch synthesis and delayed queue consumption tests.

Likely files: api/interaction.rs, api/widget.rs, pointer/click/touch adapters and focused tests; split payload migration if needed.

#### G2 — Demonstrate modifier actions and accept milestone G.

Scope: M. Dependencies: G1.

Acceptance:
- Demo displays input metadata and performs a meaningful modifier-click action through public events.
- Gesture/edit priority, pane/scale changes and all four domain payloads remain correct.
- F10 native equivalents and coordinate limits are documented; software/GPU mouse checks and local checkpoint are recorded.

Verification: Common gates and manual Ctrl/Shift-click/focus tests; keep touchscreen status separate.

Likely files: Pointer metadata demo, event snapshots, README and evidence/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### H — Locale behavior

#### H0 — Freeze the supported default locale contract.

Scope: M. Dependencies: G2.

Acceptance:
- Observe source labels for supported languages, numbering systems, locale extensions/fallback and UTC edge cases.
- Compare improving current locale handling with a Unicode-backed provider; evaluate actual scope/cost before any dependency choice.
- Choose/document supported behavior and fallback; unsupported Intl features remain explicitly partial, with custom formatter precedence intact.

Verification: Source locale fixtures including audited ar-EG and de-DE, existing en-US/fr-FR, and unsupported/invalid locale cases.

Likely files: Locale contract report, source fixtures and failing default-label tests.

#### H1 — Implement supported localized default ticks and fallback.

Scope: M. Dependencies: H0.

Acceptance:
- Supported day/year/time/month outputs match frozen source numbering/formatting, including UTC boundaries.
- Locale resolution/fallback and runtime invalidation work consistently without changing data keys or custom formatter precedence.
- Native text fonts and export paths can render the chosen strings; unsupported locale behavior is explicit.

Verification: Default formatter tests, runtime locale changes, source/native observations and real label captures.

Likely files: formatters/format_date.rs, default_tick_mark_formatter.rs, time behavior formatter integration, locale tests and only an approved required provider adapter.

#### H2 — Demonstrate localization and accept milestone H.

Scope: M. Dependencies: H1.

Acceptance:
- Locale switching changes actual labels in all applicable domains without corrupting ranges, cursor tracking or exports.
- Both renderers show the supported scripts at fractional DPR and narrow/short sizes.
- F09 resolved cases and any remaining Intl limitations are recorded separately; no blanket full-locale claim replaces untested coverage.

Verification: Common gates, locale capture matrix and software/GPU control checks; save a local checkpoint.

Likely files: Localization demo controls, captures, inventory evidence and README/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### I — Configured color parsing

#### I0 — Freeze configured color parsing and gamut policy.

Scope: M. Dependencies: H2.

Acceptance:
- Audit real source parser fallback, ordering, caching, alpha and contrast consumers; pin a parser-invocation regression.
- Compare a shared parsing policy with repeated contextual parsing; choose snapshot-safe ownership and cache invalidation.
- Define explicit unsupported Display-P3 target behavior across checked construction, legacy construction/preparation and runtime options; preserve ordinary sRGB defaults.

Verification: Source parser fixtures, invalid color cases and a reviewed public error/migration contract.

Likely files: Color policy report, source oracle/fixtures, constructor/prepare/runtime boundary tests.

#### I1 — Route configured parsers through core validation and preparation.

Scope: M. Dependencies: I0.

Acceptance:
- Layout, built-in/custom series and row colors use the configured policy consistently.
- Configured parser fallback/cache/alpha/contrast behavior matches source where supported.
- Preparation stores resolved colors in immutable frames; no parser or device setup is added to drawing.

Verification: Red/green configured Display-P3-string parser probe, layout/series color fixtures, callback counters and retained snapshots.

Likely files: model/colors.rs, owner parsing policy, series/layout validators/preparers and tests; split consumers into small compiling slices.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

#### I2 — Complete configured extension colors.

Scope: M. Dependencies: I1.

Acceptance:
- Price lines, labels, watermarks and primitives follow the documented configured-color boundary.
- All extension preparers share the correct parser policy and do not recreate parser caches per draw.
- Invalid extension colors fail consistently without partial attachment or option mutation.

Verification: Extension/parser fixtures, alpha/contrast cases, invalid options and retained snapshots.

Likely files: Extension color adapters and tests; stage lines/labels and watermark/primitive consumers as separate compiling slices when required.

#### I3 — Make unsupported target color-space requests explicit.

Scope: M. Dependencies: I2.

Acceptance:
- Checked construction, legacy construction/preparation and runtime options follow the reviewed useful-error contract.
- Existing sRGB rendering/export defaults remain intact; parsing a wide-gamut string never claims a wide-gamut output target.
- If parser replacement is offered, preflight atomically and invalidate dependent caches while retained exports remain unchanged.

Verification: Constructor/prepare/runtime/export error tests and mixed valid/invalid parser-update cases.

Likely files: Layout/runtime validation boundary, chart constructor/preparation error integration, exporter contract and tests.

#### I4 — Demonstrate color policy and accept milestone I.

Scope: M. Dependencies: I3.

Acceptance:
- Demo uses a configured custom parser and shows explicit unsupported target behavior without breaking sRGB output.
- Actual software/GPU screenshots verify supported colors, contrast, alpha and immutable exports.
- F05 parser routing is resolved; real Display-P3 rendering/hardware acceptance remains explicitly deferred, with a local checkpoint.

Verification: Common gates and color capture scenes; do not use gamut enum/readback as proof of target support.

Likely files: Color demo/captures, API migration notes, inventory evidence/report/checklist.

Checkpoint: review concrete behavior/ownership and test results; save a compiling local commit.

### Z — Combined completion

#### Z0 — Run final combined validation and publish the local completion record.

Scope: M. Dependencies: I4.

Acceptance:
- All available core tasks have source/behavior/renderer/public-consumer evidence; resolve every original targeted observation that falls within the implemented scope.
- Full tests/check/Clippy, existing source checks and unchanged relevant capture protocols pass on the final source; physical results identify backend/binary/sizes and normal exit.
- Inventory classifies each addressed item from evidence; deferred gamut/locale/device/timing/drawing-order work remains explicit, and local commits/worktree status are recorded.

Verification: Run common gates on the final unchanged source and report exact results, not an estimated percentage. No push unless requested.

Likely files: Final implementation report/evidence, audit inventory classifications, README and task checklist.

## Common acceptance gates

For each code slice, run focused behavioral tests and all-target check; run format
and strict Clippy at checkpoints. At milestone boundaries and Z0 run:

```sh
cargo fmt --check
cargo test --offline --all-targets
cargo test --offline --doc
cargo check --offline --all-targets
cargo clippy --offline --all-targets -- -D warnings
cargo build --offline --examples
node scripts/audit_public_surface.cjs --check
git diff --check
```

Maintain all 37 existing source comparisons and add focused source-backed regression
checks for the new contracts. The audit probe may remain a diagnostic, but each
resolved original observation must have an assertion in the acceptance suite.
Update inventory mappings/hashes after verified changes; never reclassify a failure
as intentional solely to get a green command. Preserve original failed samples.

Before each new capture family, freeze its exact scene/count protocol using calendar,
strike, maturity and sessions, including typed custom series. Use regular 800×500,
narrow 300×300, short 300×90 and DPR 1/1.25/1.5/2 on both **asserted actual** backends.
Retain the earlier problem-size live interaction check. Add wider/deeper scenes only
when the new behavior requires them. Never loosen or delete the old 96-image baseline,
1158 screenshot/export, 240 custom-domain or 672 attribution scene expectations to
accommodate an unrelated change. If a feature intentionally changes a scene, retain
the before image and source evidence and review only that expected change explicitly.
Rerun affected protocols at milestones and all combined relevant protocols at Z0.

Mouse acceptance uses the final normal example binary, not a profiling build: new
controls plus wheel zoom, plot/price/time-axis drag/reset, box move/resize/Escape,
release outside, live feed, pane/scale mutations and shrink/enlarge recovery. Validate
preview/export layers, logo exclusion and retained snapshots. Group by four chart
modes where useful; verify process exit and record exact user/backend/binary scope.
Do not run heavy builds/profilers during a smoothness check. Synthetic touch/high-DPI
captures do not close physical touchpad/touchscreen/high-DPI acceptance.

## Risks, boundaries and completion rules

| Risk | Mitigation |
|---|---|
| Callback precedence/cache bugs | Source-first fixture matrix; batch-dependent labels; replace/clear and retained-snapshot tests |
| Transfer changes data or leaves stale scale references | One preflighted owner mutation; ID/order/annotation/drop counters and failed-operation tests |
| Axis input steals plot/edit gestures | Explicit ownership/priority and independent flag/cancellation truth tables |
| Public helper bypasses invalidation or drains events | Reuse owner algorithms; external consumer tests with queued events and active gestures |
| Pixel height is overwritten on prepare | Integrate persistent allocation/stretch rules; test prepare/resize/reorder sequences |
| Metadata causes generic churn or large cloning | Concrete F0 prototypes; default compatibility; stage adapters; Rc payload and clone/drop tests |
| Locale differences expand scope indefinitely | Freeze support/fallback contract; no full Intl claim without matching evidence |
| Parsed wide-gamut colors mistaken for wide-gamut output | Keep sRGB output explicit; reject unsupported targets; separate future hardware milestone |
| Passing diagnostics mistaken for parity | Source-precondition assertions plus real acceptance tests and item-level inventory status |

No production implementation is part of this planning task. A milestone is complete
only after its tests, public demo, relevant source/render comparisons, available
mouse checks and local checkpoint are recorded. A partially supported locale or
explicitly unsupported wide-gamut target must remain marked as such in the final
report. Earlier timing, dedicated series-order and unavailable-device exceptions
retain their independent status. This plan does not authorize publishing/pushing
or adding unrelated indicators, drawing tools, browser wrappers or async infrastructure.

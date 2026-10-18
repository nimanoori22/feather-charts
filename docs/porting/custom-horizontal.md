# Application-defined horizontal behavior

Status: public factory and external consumer implemented; full validation in progress. Screenshot accepted by the
user on 2026-10-18 with an explicit instruction to proceed. That instruction changes
the prior sequencing gate; it does not fabricate a separate GPU mouse result.

## Source and consumers

Local 5.2.1 create-chart.ts/createChartEx accepts a user IHorzScaleBehavior, constructs
ChartApi and calls setOptions with the resolved options. Calendar and price factories
are direct callers. ChartApi, DataLayer, TimeScale and SeriesApi share the JS object:
data preprocessing/conversion, sorted keys, cache keys, item/tick formatting and
weights must agree. The public behavior accessor and external horizontal-scale and
custom-weight fixtures demonstrate real user implementations. Graph traces miss
some external fixtures, so their actual bodies were read. Stale interface metadata
and excluded coverage files were read directly; large literal financial fixture
rows are incidental to their custom behavior methods.

Required: stable ordered keys/cache keys, original public items preserved in queries
and events, formatter/cache invalidation, full-context weighting by default,
ordinary shared-index navigation, and synchronous behavior. Numeric distance is not
horizontal spacing: all domains use logical indices. Existing built-in constructors
and yield's private support series remain intact.

## Design, alternatives and ownership

Add Chart::with_custom_behavior(make_behavior, chart_options, scale_options,
localization) -> Result<Self, ChartError>. Factory invoked exactly twice, then dropped.
Each owner receives independent B state with equivalent conversion/key policies.
No B:Clone or Options:Clone; no Send/Sync bounds. Chart -> DataLayer -> B and
Chart -> ChartModel -> TimeScale -> B. Snapshot ownership stays unchanged. User
callbacks must not create owner cycles; the constructor never retains the factory.

Requiring Clone would reject callback-bearing options and could alias caches.
Exposing arbitrary data/scale pairs permits incompatible keys. A two-call factory
fits existing native ownership and equivalent-options factories. JS shared-object
identity, mutation and manual destroy are incidental, not copied into Rust.

Keep ChartError nongeneric. Add one boxed std::error::Error path and generic
DataLayerError error traits so original user causes can be inspected/downcast without
stringification. Existing ChartBehavior error-mapping signatures/bounds remain
unchanged; application errors need not pretend to be calendar/price errors. A
ChartError<E> rewrite would spread domain parameters through all existing clients.

Add horizontal_behavior_options() readback and
configure_horizontal_behavior(make_options) using the existing two-options model
helper. Formatting/cache changes apply immediately. Key reinterpretation or new
weight rules require clearing/reloading every affected series (including custom/
whitespace sources), or a new chart; stored point weights are rebuilt on data
changes, matching the existing/source lifecycle. No in-place domain-type switch.

Construction shares native option validation; invalid horizontal geometry is
rejected before user scale hooks. The data behavior receives initial scale and
localization hooks too, matching the shared source policy. No retained closure,
registry, async engine or thread-safe cache is needed.

## Verification plan

Real source probe and hashes before production edits. External Rust consumer uses
SessionPoint {session, phase} and separate SessionKey, non-Clone behavior/options,
owned non-Send error causes and factory/hook lifetime observations. Cover updates,
history/pop/empty/invalid operations, custom intervals, original-item events,
formatting invalidation, weighting fallback/fast path and owned export. Demo uses
only public APIs. Retain all prior source and 96 exact raster comparisons; actual
software/GPU captures and mouse checks use the application domain.

## Implemented API and data failure behavior

The constructor, options readback and formatter-configuration methods are public on
`Chart<B>`. Both owners receive scale/localization initialization. Shared horizontal
geometry validation is unchanged for runtime patches and now protects custom
construction before hooks. Other chart-option errors use existing model validation.
The application owns validation of its own behavior options.

A regression test found that built-in replacement removed shared memberships before
fallible preprocessing. Replacement now stages plot rows before cleanup, like the
existing custom-series path. Failed preprocessing preserves data and the original
owned error; a subsequent shared-index mutation and pop verify layer/model coherence.
This intentionally strengthens native Result error handling rather than copying JS
partial mutation on exceptions. Full replacements temporarily retain staged rows;
incremental update hot paths are unchanged.

The external example defines session/phase public items and a separate internal key,
with deliberate key gaps, non-Clone policies and owned non-Send errors. It uses public
APIs for a line, typed custom intervals, original-item hover events, editable box,
pane movement, live updates and screenshot preview/raw export. Device construction
happens once at application boot. No new dependency or engine async machinery.

## Behavioral tests and review

`tests/custom_horizontal.rs` verifies separate behavior owners, factory/drop lifetime,
hook ordering, owned conversion/data causes after chart drop, invalid geometry,
failed replacement recovery, full-context versus opt-in append weighting, custom
row reindex/pop ownership, original-item ranges and queries, label pixel cache reset,
and immutable prepared exports. The example exercises its control sequence through
empty data, append, pane changes and reset. The source probe compares actual local
5.2.1 behavior against a native external test; frozen source hashes are checked.

The original missing-API compile failure and the failing data-loss regression are
retained under `target/custom-horizontal`. An inverted constructor validation
condition was rejected by the focused error test, then restored before verification.
Review covers ownership, Result paths, synchronous hot paths, unchanged callers and
public enum migration. `ChartError::CustomBehavior` is additive but requires a new
arm in exhaustive downstream matches. No key-policy runtime switching is promised.

## Files

Production: `src/api/behavior.rs`, `src/api/chart.rs`, `src/model/data_layer.rs`,
`src/model/chart_configuration.rs`, `src/model/chart_model/configuration.rs`.
Consumer: `examples/custom_horizontal.rs`, `examples/custom_horizontal/behavior.rs`,
`examples/custom_horizontal/captures.rs`, `tests/custom_horizontal.rs`.
Verification: `scripts/check_custom_horizontal_reference.cjs`,
`scripts/capture_custom_horizontal.py`, frozen `custom-horizontal-{source,states}.json`.
README and remaining-features checklist document usage and acceptance. Screenshot
acceptance evidence records the user's authorization without inventing backend tests.

## Remaining scope

Attribution-logo rendering is milestone C and has not started. Existing timing,
drawing-order and unavailable physical-device exceptions remain independent. Custom
keys must stay stable and totally ordered. Changing their meaning requires a new
chart; changing weights requires clearing all shared series before configuration and reloading afterward. Panes, primitives and exporters reuse
existing engine components; no parallel lifecycle or rendering abstraction was added.

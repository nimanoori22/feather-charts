# Incremental live-data append

Foundation: `6c743ca`. Implementation and acceptance results follow below.

Latest status: the [cache and live-feed follow-up](live-append-acceptance.md)
passes 23/24 history screens; performance and physical acceptance remain open.
The original implementation measurements below are preserved as historical evidence.

## Contract and consumers

The behavioral reference is Lightweight Charts `data-layer.ts`, its row creator,
`chart-api.ts::_sendUpdateToChart`, `chart-model.ts::updateTimeScale`,
`series.ts::setData`, `tick-marks.ts`, and the native time weight generator.
Graph references were checked, but upstream metadata was stale for the data layer,
row creator, series and tick/weight components; current source was read directly.
Generic/trait dispatch edges are incomplete, so direct consumers were also read.

An update to a new shared tail time assigns one new logical index. Existing
indexes, row metadata and original times remain unchanged. An owner's latest
point may instead join an existing shared time or insert in the shared middle;
only the latter reindexes peers. Historical updates require an existing shared
point. Whitespace reserves a time but does not advance the last fulfilled base.
The shared base and fulfilled membership must include built-in and typed custom
owners. Failed conversion/row creation must not mutate the shared mapping.

ChartApi invalidates affected pane data before changing the time mapping, then
applies series data, updates fulfilled membership and recalculates panes. The
model preserves follow-latest and whitespace replacement policies. Series,
autoscale, crosshair, markers, primitives and retained frames consume stable
logical indexes and normal generation invalidation. They must not require peer
row replacement when those indexes stay stable.

JavaScript's shared row/array identity is incidental: Rust owns rows in DataLayer
and series separately. Resending a JS array does not imply cloning that entire
array in Rust. All engine operations remain synchronous.

## Design alternatives and ownership

1. Replace the existing response with a mandatory unchanged/append/replace enum.
   This excludes invalid combinations statically, but forces all existing full
   response producers and consumers to migrate together.
2. Extend the current response with an explicit optional append descriptor and
   an optional authoritative single-point membership delta. Keep existing full
   fields for fallback. Validate contradictory/stale descriptors before mutation.

Choose (2) to preserve the established full-update path and public chart API.
The descriptor carries its expected index and owned point. Model validation
checks the current length and strictly increasing time before any invalidation.
TimeScale appends to its existing vector and tick buckets. No borrowed references
escape a handoff, no new shared ownership or asynchronous machinery is needed.

DataLayer owns normalized points and a fulfilled-index set; full mutations rebuild
that set and incremental mutations change only one member. The coordinator and
TimeScale each retain their existing membership set and consume a one-index delta
rather than gathering every target's history. Typed custom payloads stay owned by
their custom series. Existing Rc/RefCell ownership at chart boundaries is unchanged.

Important API additions (the Chart update methods stay unchanged):

```rust,ignore
pub struct TimeScaleAppend<I, O> { pub index: usize, pub point: TimeScalePoint<I, O> }
// TimeScaleChanges adds append: Option<TimeScaleAppend<I,O>> and
// membership: Option<(usize, bool)>; points still carries a full replacement.
fn append_weight(&mut self, previous: Option<&Self::InternalItem>,
                 current: &Self::InternalItem) -> Option<TickMarkWeightValue>;
// Default None requests the existing full-context weighting/reindex fallback.
```

Native time opts into predecessor-based weighting. The initial single point keeps
its default weight; a later tail append does not rewrite the first point using a
new average interval. Unknown horizontal behaviors retain full context. Tick
selection must preserve global weight priority and uniform distribution; a new
higher-weight tail can displace lower-weight marks, so cached selection cannot be
blindly extended.

## Validation and limits

See [the saved plan](../../tasks/incremental-append-plan.md) and [its checklist](../../tasks/incremental-append-todo.md) for the unchanged budgets
and physical acceptance gates. Timing evidence must distinguish warmed updates,
snapshots, glyph conversion, allocation/cold events and required global rebuilds.

A borrowed weighting context was also considered. It would require changing all
behavior implementations and exposing DataLayer's point storage layout; the
optional predecessor capability keeps full-context implementations valid. For
membership, contributor counts would duplicate each point's existing owner maps;
an aggregate set plus a one-point union query is sufficient here.

Tick selection caches the selected index vector after each priority group.
Appending recomputes only the suffix that higher-priority changes can displace,
expanding the affected region by one label spacing at each group. This preserves
existing prefix allocations. Unknown membership versions, new weight groups,
layout changes and uniform distribution retain a correct global rebuild. The
randomized test compares incremental results to the pre-existing global algorithm;
the independent Node check executes the actual upstream methods.

## Integration correction

`Chart::update_custom_data` advanced the widget's structural revision on every
ordinary row update. That reset a native mouse drag on its next event. Built-in
updates did not advance this revision. Custom row updates now follow the same
policy; normal model/view generations still invalidate rendering. The native
regression test first reproduced cancellation for a custom owner, then passed for
both plot and price-axis drags with built-in and custom tail initiators. Structural
changes and explicit cancellation retain their existing behavior.

## Ownership and API compatibility

No new Rc edges, callbacks, threads, dependencies or asynchronous paths were
introduced. A response owns its one point and changed rows; the coordinator moves
them into their existing owners. Existing Rc/RefCell adapters keep their original
drop behavior. Retained custom frames own frozen visible data and remain drawable
after later appends and owner removal.

The public `Chart::update_data` and `Chart::update_custom_data` signatures are
unchanged. Low-level users constructing `TimeScaleChanges` literals must initialize
`append` and `membership` (normally `None` for a full replacement). Replayed,
wrong-index, non-increasing and contradictory append descriptors are rejected by
the model before mutation; direct low-level TimeScale/coordinator calls assert the
same precondition.

## Measured corrections before acceptance

An isolated middle-insertion diagnostic initially regressed from 7.274/8.044 ms
median/p95 to 8.440/9.133 ms. Rebuilding the aggregate set from its already ordered
indexes, rather than inserting each index separately, reduced the diagnostic to
6.913/8.091 ms. Reloading remained near its original cost. These are diagnostic
numbers; the final three alternating sets below determine acceptance.

The first controlled append set for engine `868b46c` then exceeded the 10% screen
on one-source history growth. A key-count regression test reproduced 76 key
evaluations for a 1,200-point tail lookup. DataLayer now recognizes the current
tail before binary searching, and inserts a new maximum directly at the end.
The guard caps key work at 12 for both 1,200 and 100,000 points, while the existing
binary search remains for historical/shared-middle access. Three follow-up Line
diagnostics passed the unchanged median/p95 history screen. All earlier samples,
the earlier candidate binary, manifest and intentional stop/failed-launch log are
retained under `target/incremental-append/pre-tail-lookup/`; they are not relabeled
as results from the corrected engine.

The private per-point owner maps have two viable representations: retain ordered
BTreeMaps, or use standard HashMaps for direct owner lookup. Their consumers only
insert/remove/get owners, visit every owner to reindex, or ask whether any value
is fulfilled. Time order comes from the point vector, response order from the
separate ordered registries, and original time from explicit point creation.
No consumer relies on per-point owner iteration order. The allocation experiment
compares these two representations before choosing one; neither changes the
public API or ownership. All response and registry maps remain ordered.

The experiment retained BTreeMaps. A counted System allocator found 3,808 requested
bytes per one-source point and 23,064 bytes per 16-source point, identically at
1,200 and 100,000 history rows (20 warmups/100 samples). HashMap reduced the
one-owner case to 1,964 bytes but increased the 16-owner batch to 30,172 bytes and
36 allocations instead of 35. Isolated runtime probes did not establish a
consistent benefit. Its production change was reverted; both allocation logs and
all timing probes remain available. These are requested allocation bytes, not
retained heap size or RSS. The example's counting allocator is diagnostic only.

## Behavioral verification

The append integration tests cover all six built-ins, mixed typed custom owners,
whitespace/value transitions, shared-middle insertion, historical restrictions,
clear/reload/pop/remove followed by another append, exact business-day original
times and metadata, invalid builder/row failure atomicity, and once-only custom
cleanup. Retained custom frames remain drawable after later updates and removal;
two different datum types across panes retain their types and indexes.

Native event regression tests inject a built-in or custom append during plot and
price-axis mouse drags. Fixed-range primitive editing survives an append through
commit. Existing touch/kinetic suites exercise ordinary live updates and gesture
cancellation; these deterministic tests do not substitute for device acceptance.

Operation guards independently bound metadata/payload cloning, native key
comparisons, weighting context and membership gathering. Warmed tick tests compare
1,200/100,000 points; a randomized oracle compares cached suffix selection with
the original global algorithm, including whitespace, new priorities, uniform
selection and unknown membership versions. The last-ten-index suffix lookup
uses 11 index visits at either history size; the previous binary search used 18
at 100,000 points and failed that guard.

`check_incremental_append_reference.cjs` executes the upstream DataLayer, row
creator, time weights and TickMarks. It compares 13 interleaved built-in/custom
states and 156 tick selections with the Rust example, including fallback
transitions. Other reference scripts continue to check the existing port.

## Changed files and remaining dependencies

Engine changes are in `src/model/data_layer.rs`, `chart_data_coordinator.rs`,
`chart_model.rs` and its tests, `time_scale.rs`, `tick_marks.rs`,
`ihorz_scale_behavior.rs`, `horz_scale_behavior_time/horz_scale_behavior_time.rs`,
`horz_scale_behavior_time/time_scale_point_weight_generator.rs`, `plot_list.rs`,
`series.rs`, and `src/api/custom_series.rs`. Behavioral integration changes are in
`tests/incremental_append.rs`, `time_mark_preparation.rs`, `public_custom_series.rs`
and `primitive_editing.rs`.

Examples and scripts provide the differential reference, live/narrow/burst
captures, phase profiler, immutable-artifact runner and allocation diagnostic.
This report, README and the saved plan/checklist record the result and open gates.

Arbitrary horizontal behaviors without predecessor weighting retain full rebuilds.
Middle insertions, replacement/removal/pop, uniform tick distribution and new
priority groups may legitimately rebuild global state. Vector capacity growth,
ordered-set insertion and allocation are not worst-case constant time. Conflation
rendering remains a separate follow-on milestone, after append acceptance.
No Tokio, channels, threads or new dependencies were added to the chart engine.

## Final automated checks

At engine `27b543e` (tooling checkpoint `6033815`): `cargo fmt --all -- --check`,
`cargo test --all-targets` (539 tests across 47 targets), `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings`, and `cargo build --examples` pass.
All 20 Node source-reference scripts pass against freshly built examples.

The broader renderer regression pass completed 492 captures per backend. The
final live/narrow/burst subset then passed 72 captures per backend with the final
engine, at ratios 1/1.25/1.5/2. These are 1,128 successful capture executions in
total, including reruns, rather than 1,128 distinct scenarios. A final contact
sheet was visually inspected for narrow Line and burst Candlestick on both
renderers; axes, labels and the volume pane agree.

Reproduce source/capture checks after `cargo build --examples`:

```sh
node scripts/check_incremental_append_reference.cjs
python3 scripts/check_incremental_append_captures.py --append-only
cargo run --release --example append_allocations
```

`target/incremental-append/` holds the raw validation logs, capture images,
allocation diagnostics and frozen profilers. It is intentionally ignored by Git.
The checked-in report and timing summary record the reviewable outcome.

## Timing protocol

The final comparison freezes baseline `6c743ca` and engine `27b543e`, using
byte-identical profiler sources (tooling checkpoint `6033815`) in both builds.
The archive's package name differs only to prevent Cargo artifact collisions;
its library name and engine source are preserved. Four immutable executables and
the two harness files are SHA-256 identified in the manifest. A startup sentinel
also verifies a new tail returns four rows on baseline and one on the new engine.

Three serial sets alternate baseline/current order, pinned to CPU 5. Every case
has 20 warmups and 100 measured samples. Inputs are constructed outside the timer;
public chart update, snapshot preparation and glyph conversion are measured
separately. The viewport is fixed at logical 20..100, 917×870.80005, ratio 1.25;
last prices are fixed to avoid conflating append with a required global scale
change. No renderer, Cargo build, test job or graph indexing runs alongside it.

The matrix covers Line/Candlestick/Histogram/typed Custom at 1,200/20,000/100,000
rows and 1/16 sources, plus mixed owners, a 90,000-point whitespace tail, bursts of
eight, middle insertion, reload, navigation, static and latest replacement.
Each append case updates one owner with the indicated total source count; the
separate allocation diagnostic appends all owners to a shared timestamp. These
are different workloads and their totals must not be interchanged.

All 204 case executions, 612 phase summaries and 61,200 warmed timing samples
are retained, including outliers. First-update timings are reported separately;
allocator diagnostics count capacity events rather than silently filtering them.
An established work item above 10% is investigated, while the append history
screen requires both median and p95 growth to stay at or below 10% from 1,200 to
100,000 points in every paired set. The threshold has not been relaxed.

## Final timing results

All values below are milliseconds. Ranges span the three run medians; p95 is the worst of the three per-run p95 values. Full phase summaries are in [the CSV](incremental-append-measurements.csv).

| Kind | Sources | Baseline update median range | Current update median range | Current worst p95 |
| --- | ---: | ---: | ---: | ---: |
| Line | 1 | 32.728532–33.776382 | 0.004695–0.004847 | 0.005373 |
| Line | 16 | 482.048012–536.691488 | 0.017259–0.017979 | 0.083164 |
| Candlestick | 1 | 34.233357–34.693786 | 0.004920–0.005201 | 0.005488 |
| Candlestick | 16 | 541.472127–589.844988 | 0.021489–0.030474 | 0.049085 |
| Histogram | 1 | 33.205366–33.528947 | 0.004583–0.004761 | 0.005602 |
| Histogram | 16 | 476.297329–532.181826 | 0.019452–0.025208 | 0.044190 |
| Custom | 1 | 17.303700–20.196328 | 0.004915–0.005109 | 0.006300 |
| Custom | 16 | 229.338840–233.400246 | 0.028006–0.029703 | 0.061148 |

**The performance gate remains open: 19/24 history screens pass.** These five fail the unchanged 10% requirement:

| Run | Kind | Sources | Median growth | p95 growth |
| ---: | --- | ---: | ---: | ---: |
| 2 | Line | 16 | +13.69% | +134.83% |
| 3 | Line | 16 | +14.20% | +10.65% |
| 2 | Candlestick | 16 | -10.69% | +11.89% |
| 2 | Histogram | 16 | +34.15% | +8.70% |
| 3 | Custom | 16 | +11.09% | -2.81% |

The earlier `b70fd9f` full comparison also passed 19/24 screens. The later suffix-search correction fixes the reproduced index-visit excess, but does not close latency acceptance. The earlier comparison, partial pre-tail run, immutable binaries and all diagnostics remain separately labeled in `target/incremental-append/before-suffix-search/` and `pre-tail-lookup/`.

The exact cause of the remaining latency changes is not isolated. Payload/key/index guards pass, and the allocation probe reports the same requested bytes at both histories; these checks do not measure allocator latency, cache residency, every tree lookup or scheduling. CPU pinning prevents migration but does not reserve the core. No sample was removed and no failure is attributed to noise as an established fact.

### Other workloads and phases

| Case / stage | Baseline median range | Current median range | Baseline worst p95 | Current worst p95 |
| --- | ---: | ---: | ---: | ---: |
| Line mixed (100,000/16) / update | 523.437841–568.997785 | 0.025520–0.039817 | 620.786849 | 0.081537 |
| Line whitespace (100,000/1) / update | 16.673546–17.787322 | 0.004288–0.004390 | 22.190364 | 0.008645 |
| Line burst (100,000/1) / update | 270.377745–281.764347 | 0.014727–0.015224 | 321.148799 | 0.015777 |
| Line middle (20,000/2) / update | 7.571815–7.827503 | 7.093117–7.537946 | 9.442354 | 12.055676 |
| Line reload (20,000/2) / update | 19.283887–20.289597 | 18.113986–19.474831 | 26.026571 | 20.535602 |
| Candlestick static (20,000/16) / snapshot | 5.893585–5.929768 | 5.754487–5.899216 | 6.081310 | 6.015863 |
| Candlestick latest (20,000/16) / update | 0.019306–0.030291 | 0.018978–0.039382 | 0.044318 | 0.102112 |
| Candlestick latest (20,000/16) / snapshot | 5.794857–5.882953 | 5.735198–5.910978 | 6.006017 | 6.271526 |
| Line navigation (100,000/16) / snapshot | 4.893375–5.001897 | 4.911662–5.106990 | 5.255138 | 5.272166 |
| Line append (100,000/16) / snapshot | 4.126797–4.154505 | 3.233186–3.263056 | 4.664304 | 3.566976 |
| Line append (100,000/16) / glyph | 0.096050–0.097249 | 0.081633–0.086054 | 0.126220 | 0.151954 |
| Custom append (100,000/16) / snapshot | 5.843130–5.856212 | 4.799517–4.805190 | 6.314228 | 5.052406 |
| Custom append (100,000/16) / glyph | 0.086483–0.087443 | 0.074960–0.077000 | 0.107659 | 0.100331 |

Middle insertion and reload remain full rebuilds and their final update medians improve across all three sets. Latest replacement is not uniformly improved: Candlestick run 3 increases from 0.030291/0.044318 ms median/p95 to 0.039382/0.102112 ms (+30.01%/+130.41%). Middle insertion run 3 also has a p95 increase from 8.344673 to 12.055676 ms (+44.47%), despite its slightly improved median. Static/navigation and glyph tails also vary; all phase results remain in the CSV. These observations keep the established-workload performance gate open as well.

For example, run 1 Line at 100,000 points/16 sources separates 0.017259 ms update, 3.240708 ms snapshot and 0.081633 ms glyph conversion. The first update is 0.840917 ms; the one-source first update is 1.768187 ms versus a warmed median of 0.004756 ms. These are first-update/capacity observations, not worst-case bounds or full UI frame times.

### Artifact identities

The ignored local manifest contains these SHA-256 identities; frozen baseline/current harnesses are byte-identical:

```text
baseline-profile 02848a85c58a503d007a9449669cc06be9ef11e76f317431f1f492cde1c24199
baseline-custom-profile 66be7a657f3b3f13451659a438673862c1a94a43a61a8fd61b6866ba8216e3af
current-profile bd91410436403e26dfc37ae4be3fd36a7b767e5761039848e16124d54816ac9f
current-custom-profile cc7d94ed82add340e4b3ac6f0d8b04e8558165dbfa1e2952b46fb77b55bfd004
examples/preparation_profile.rs f6e1286a69140757fbc086494e97893a7a39fcb8910fd5eeef759bd6db67229d
examples/custom_series_profile.rs 8a829ac0a0b4e5d8e42228dafa01b54f222590d11fc8978c324763be7299decb
```

To repeat the runner, build the same profiler sources against the two engine revisions in separate target directories, copy the four release executables to the manifest names under `target/incremental-append/`, then run `python3 scripts/profile_incremental_append.py`. Preserve previous run directories before starting another comparison.

## Acceptance status

The engine and automated behavior/source/renderer validation are implemented. The append history and established-workload performance gates remain open. Physical mouse/touchpad/touchscreen acceptance during live appends, box editing and half-size/high-DPI operation remains unchecked. The milestone is not marked accepted, and conflation rendering is not started.

The next performance task is to isolate the remaining 16-source append tails and the latest/middle outliers with per-phase CPU/allocation/cache evidence, then rerun the unchanged screens. Avoid adding another cache or changing map representation without a reproduced cost. Device acceptance can proceed independently using the checklist.


## Acceptance follow-up (2026-10-15)

The [live-append investigation](live-append-acceptance.md) preserves the historical
results above and records subsequent cache fixes, matched measurements and native
live-feed controls. The latest complete comparison passes **23/24** unchanged
history screens; one Histogram16 p95 pair and physical-device acceptance remain open.

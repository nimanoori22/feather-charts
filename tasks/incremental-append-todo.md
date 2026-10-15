# Incremental live-data append checklist

Plan: [incremental-append-plan.md](incremental-append-plan.md).
Foundation: `6c743ca`. Engine checkpoint: `27b543e`. Status: engine implemented; functional validation complete; performance acceptance open (23/24 history screens pass; see [follow-up](../docs/porting/live-append-acceptance.md)); physical acceptance in progress (GPU mouse checks passed; see device record).
Earlier hardware gates and append-tail exceptions remain independently recorded.

## 0. Source contract and design

- [x] Read complete upstream components/dependencies and trace real consumers.
- [x] Check graph coverage and read current source where incomplete or stale.
- [x] Classify startup, shared tail append, existing point and shared middle insertion.
- [x] Compare delta/weight/membership designs; record ownership and Rust signatures.
- [x] Verify the proposed handoff against every important consumer category.

## 1. Baseline and failing guards (depends on 0)

- [x] Freeze fresh release baseline/artifact identity, workload and timing budgets.
- [x] Add failing behavioral/copy/visit guards for full-history append work.
- [x] Record phase/public-route timings, mixed sources and whitespace tails.
- [x] Measure cold/capacity events separately from warmed steady state.

## 2. Append handoff (depends on 1)

- [x] Add the chosen explicit append protocol; keep legacy producers working.
- [x] Extend TimeScale/coordinator without replacing the existing point prefix.
- [x] Adapt response constructors/fixtures and validate invalid/stale descriptors.
- [x] Pass existing full-update and focused handoff tests.

## 3. First built-in path (depends on 2)

- [x] Route public Line tail append through existing changed-row application.
- [x] Stage row/conversion validation before shared mutation.
- [x] Preserve old indexes/metadata and correct startup/whitespace/base behavior.

## Checkpoint: first public append

- [x] Line append works end to end with retained history and valid labels.
- [x] Record unresolved membership/weight/cache costs; keep final gate open.

## 4. Mixed built-in/custom owners (depends on 3)

- [x] Extend the shared route to all six built-ins and typed custom data.
- [x] Verify two custom datum types, builder errors and stale/foreign handles.
- [x] Avoid copying unrelated owner rows or full custom index vectors.
- [x] Preserve exact original time/metadata and retained-frame safety.

## 5. Membership and base index (depends on 4)

- [x] Apply point membership changes without collecting/copying the full union.
- [x] Keep overlapping sources and value/whitespace transitions correct.
- [x] Avoid history scans for long whitespace-tail appends.
- [x] Rebuild auxiliary state correctly on removal/pop/full mapping changes.

## 6. Weights and tick caches (depends on 5)

- [x] Preserve native first-point/calendar/intraday weighting with bounded context.
- [x] Preserve a correct fallback for arbitrary horizontal behavior.
- [x] Update tick buckets at the append boundary and measure selection rebuilding.
- [x] Verify global priority, uniform distribution, whitespace and edge alignment.
- [x] Record required global work and allocator limits explicitly.

## Checkpoint: engine deltas

- [x] Built-in/custom append handoffs preserve shared history and only changed rows.
- [x] Operation guards cover weighting, base index and fulfilled membership.
- [x] Remaining global cache work is measured and behaviorally justified.

## 7. Fallback transitions (depends on 4–6)

- [x] Preserve shared middle insertion and historical restrictions.
- [x] Test append -> reindex/replacement/remove/pop -> append transitions.
- [x] Compare interleaved built-in/custom event sequences with upstream source.

## 8. Chart integration (depends on 7)

- [x] Verify follow-latest/fixed-range and whitespace-shift policies.
- [x] Verify price modes, autoscale/manual scales, labels, markers and primitives.
- [x] Verify crosshair/exact-index payloads, hits and retained frames.
- [x] Exercise injected updates during mouse/touch/kinetic/edit gestures across panes (device acceptance below).
- [x] Pass live-update captures on both renderers at four ratios and narrow sizes.

## 9. Performance gate (depends on 8)

- [x] Pass bounded old-row/index/weight/membership operation guards.
- [x] Run three isolated alternating baseline/current sets with 20 warmups/100 samples.
- [ ] Pass the unchanged 10% steady-state history-growth screen for eligible updates (23/24 pass; one Histogram16 p95 pair fails; three focused repeats pass).
- [x] Complete the predefined larger Histogram investigation: six 2,000-sample pairs pass; add a repeatable guard and preserve the original exception ([evidence](../docs/porting/histogram-tail-follow-up.md)).
- [x] Investigate short-window flags with same-history controls and 8,000 fresh counter samples; report confounded attribution without filtering failures ([cause and fix](../docs/porting/histogram-window-cause.md)).
- [x] Adopt explicit update/frame acceptance rules and measure real presentation during append/zoom/pan/price-axis input ([results](../docs/porting/native-frame-acceptance.md)).
- [x] Fix duplicated pane preparation; preserve all geometry/hits and 984 identical renderer captures.
- [x] Profile software drawing and eliminate fully occluded opaque rectangle work; preserve pixels ([follow-up](../docs/porting/render-update-follow-up.md)).
- [x] Add optional allocation/hardware diagnostics to the actual update fixture, excluded from acceptance.
- [ ] Pass revised acceptance: latest isolated update study 5/6; native 63/64 cohorts across two sets, including all 32 software cohorts. One GPU presentation failure stops set three. Exception remains open.
- [x] Investigate latest/middle tails with thread/hardware counters and three matched comparisons; earlier outliers do not recur, all evidence retained.
- [x] Report phase/public-route, source-count, cold/capacity and global-cache costs.

## 10. Final validation/report (depends on 9)

- [x] Pass fmt, all-target tests/check/strict Clippy and example builds.
- [x] Pass all existing and new source-reference scripts and renderer regressions.
- [x] Recheck original consumers, ownership and connected code.
- [x] Publish design, exact changes, behavioral tests, measurements and limits.
- [x] Update README/checklist; close only performance gates actually demonstrated.

## Physical acceptance

Mouse only is available. GPU regular/half-size checks passed by user observation;
a software problem was reported and awaits diagnosis. See the [device record](live-append-mouse-acceptance.md).

- [ ] Live feed while using mouse/touchpad zoom, plot pan and price-axis controls.
- [ ] Live feed during box editing, commit/cancel and fresh gestures afterward.
- [ ] Touchscreen pan/pinch/tracking/flick during append and interruption.
- [ ] Half-size/high-DPI checks on each available backend with device/scale/results.
- [ ] Mark milestone accepted only after its automated, performance and physical gates pass.

## Follow-on

- [ ] After append acceptance, investigate and plan conflation rendering separately.

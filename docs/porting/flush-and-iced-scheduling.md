# Flush isolation and real-Iced scheduling

Acceptance remains **OPEN**. This continuation localizes update tails further,
rejects a combined-query optimization, and records the normal Iced application.
No production engine, public API, runtime scheduling or acceptance policy changed.
The [curated summary](flush-and-iced-scheduling-summary.json) retains measurements
and hashes. The source checkpoint is `2f1f58a`.

## Update operation isolated

Two serial, diagnostic-only studies each completed six alternating history pairs:
1,200/100,000 rows, 16 Histogram sources, fixed last price, CPU 5, 20 warmups and
2,000 measured appends. The first split model flush into marker synchronization,
visible-range calculation, pane recalculation, invalidation and crosshair refresh.
The second split pane recalculation into scale lookup and source-range calculation.
Each execution retains 2,020 whole updates; the first has 20,200 phase rows and
the second 36,360. Trace storage was touched before timing. RDPMC snapshots and
clock reads perturb timing, so these are not acceptance measurements.

The slow cohort is the 100 slowest **whole updates** in each execution, rather
than a different selection for each phase. Across the twelve runs:

| Operation, median within each cohort | All updates, range across runs | Slow cohort, range across runs |
| --- | ---: | ---: |
| Pane recalculation wall time | 1.475–1.598 µs | 5.715–7.283 µs |
| Pane recalculation user instructions | 7,055 | 7,055 |
| Pane recalculation generic cache misses | 30–80 | 390.5–538.5 |
| Scale source-loop wall time, finer probe | 0.997–1.081 µs | 2.883–4.002 µs |
| Scale source-loop user instructions | 6,231 | 6,231 |
| Scale source-loop generic cache misses | 23–52 | 196–270 |

The scale's loop over visible series—first value, autoscale data and range/margin
aggregation—is the largest isolated part of pane recalculation. Crosshair refresh
is approximately 0.12 µs normally and 0.33–0.50 µs in the slow cohort of this
no-pointer update fixture. This does not describe crosshair cost with an active
pointer or custom callbacks. Marker-spacing synchronization remains smaller than
pane recalculation; its previously rejected cache was not retried.

The increased cache-miss counts support investigating memory/cache behavior in
that loop. They do not prove a particular stalled instruction, cache level or
causal engine defect. The two probes have different code layouts and overhead;
their phase medians cannot be subtracted to derive exact component costs.
The earlier fault/kernel evidence and its limits remain in the
[timing attribution report](timing-resolution.md).

## Combined source query: rejected

The tested alternative adds a default synchronous `autoscale_snapshot(start, end)`
trait method returning owned `(FirstValue, AutoscaleInfoImpl)` values. It preserves
visibility → first-value → autoscale callback order and the same early returns,
then lets the compiler specialize that sequence behind one dynamic dispatch.
It introduces no cross-update cache or extra ownership. Caching entire autoscale
results instead would require preserving dynamic provider and primitive callbacks;
that broader change was not attempted.

A copied release binary contains both baseline and combined modes. Before
measurement, the protocol fixed six alternating mode pairs at **each** history,
24 executions total, using the original uninstrumented update fixture. Keeping
it required at least 5% median improvement in five of six pairs at both histories,
nonoverlapping median ranges, no median/p95 regression above 10%, and green tests.

It failed. Small-history median changes ranged from 2.88% faster to 8.21% slower;
large-history changes ranged from 13.87% faster to 5.70% slower. Both ranges overlap
their baselines, and two paired p95 results regressed by more than 10%. The isolated
large gain does not override the fixed decision rule. The trait method and branch
remain only in the ignored experiment copy; neither enters the public API.

All 388 copied-library tests passed in combined mode, including a new trait-object
test for hidden sources, absent first values, absent autoscale data, returned
values, interval forwarding and callback order. Correctness passed; performance
was insufficient to retain the change.

## Normal Iced application recordings

The exact locked Iced/winit sources were inspected, with source fallback where
generic call-graph resolution was incomplete. The copied `public_chart` runs
through the actual Iced event loop, application update, UI layout, chart preparation,
canvas drawing, renderer and presentation. It retains the normal two-source,
120-initial-row Histogram demo, primitives/markers and 250 ms live feed. A separate
16.666667 ms diagnostic subscription injects 480 gesture messages, alternating plot
and price-axis dragging every 120 ticks. These are synthetic application messages,
not physical mouse events or the native 16-source acceptance workload.

The trace adds timer receipt, redraw requests and redraw entry to the existing
render/submit/presentation hooks. Frame association uses the first redraw after a
scripted mutation completes. Coalesced gestures remain in the gesture analysis;
they are not silently removed. This is a client-side association, without the
privileged compositor/DRM identity join used by earlier observer captures.

The first four sessions requested two sizes but were tiled at 941×508/950×517.
Those captures are retained as unsuitable for the requested size comparison.
A corrected copy uses fixed-size diagnostic windows, obtaining stable 917×871
and 420×320 dimensions at scale 1, with reported monitor refresh 60 Hz. No desktop
configuration changed. The feed stops after the final scripted gesture to allow
feedback to drain before automatic exit.

| Fixed-size session | Request to redraw p95 ms | Redraw to submit p95 ms | Redraw to presentation p95 ms | Script receipt to presentation p95 ms |
| --- | ---: | ---: | ---: | ---: |
| Software 420×320 | 4.648 | 2.652 | 32.991 | 38.045 |
| GPU 917×871 | 1.719 | 2.475 | 32.936 | 35.846 |
| GPU 420×320 | 7.627 | 2.023 | 32.945 | 41.252 |
| Software 917×871 | 5.676 | 11.068 | incomplete | 39.929, qualified below |

The regular software run contains **910 native cursor-move events**, no button or
wheel events, and two unresolved final feedback records among 505 submissions.
It is not an isolated script comparison. All 480 scripted mutations do have a
following presented frame, but that does not make the full recording complete or
remove the cursor-activity limitation. The other three fixed-size sessions have
zero native cursor input and complete feedback: 479, 451 and 473 submissions,
respectively. No fixed-size session reports a discarded frame.

The frame columns retain startup/shutdown frames; the gesture column excludes
the original 20 warmup ticks per 120-tick block. The table's percentiles are
distinct distributions; adding them does not give an end-to-end p95. Requests map to the earliest request since the previous redraw,
so repeated requests cannot hide the initial wait. Script receipt excludes the
preceding timer delivery delay. The locked Tokio-backed Iced subscription emits
**scheduled tick Instants**, with missed-tick behavior Skip. Its elapsed payload
therefore includes startup/backlog, not merely message transport. GPU regular's
first payload is already about 484 ms old during graphics startup; this is not
evidence of a steady-state half-second event-loop stall. Raw startup samples and
the original 20 warmups per gesture block are retained unchanged.

The actual Iced path exhibits request/gesture-to-redraw delay plus the subsequent
presentation wait. This does not establish an additional Iced scheduler defect
or justify bypassing winit frame callbacks. The earlier direct-dispatch and
callback-only candidates remain rejected. These diagnostic results neither pass
nor replace the unchanged native acceptance cohorts.

## Verification and decision

Passed: the 388 copied-library tests, strict release Clippy on the finer pane
probe, focused existing Python update-probe tests, raw phase/sample and percentile
checks, strict JSON/provenance checks and Git whitespace validation. Experimental
Iced dependency builds retain their existing warnings; no strict-Clippy pass for
those patched dependencies is claimed.

Only reports and the task note are retained. Raw captures, source snapshots,
protocols, build attempts and binaries remain locally under
`target/flush-isolation-study/` and `target/iced-scheduling-study/`; ignored raw
artifacts are not distributed by Git. The summary hashes the modified Iced sources
as well as the manifests, scripts and recordings.

No tested production remedy demonstrated an improvement. The fixed policy requires
fixing the measured operation before another acceptance set, so no acceptance
retry followed this rejected candidate. Update INCONCLUSIVE, native 31/32 first-set
result, remaining device checks and conflation remain open. The requested profiling
and candidate evaluation are complete; a production timing fix is not established.

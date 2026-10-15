# Timing experiments after verified attribution

Acceptance remains **OPEN**. Two candidate optimizations were tested in isolated
copies of commit `3479208`; neither met its predefined keep rule. No production
engine, Iced runtime, desktop setting, priority or acceptance budget was changed.
The [summary](timing-experiments-summary.json) records results and artifact hashes.

## Earlier frame dispatch: rejected

The exact locked winit 0.30.13 Wayland event loop defers a requested redraw while
a frame callback is outstanding. A copied native fixture compared that existing
redraw path with drawing directly when the original fixed timer became due.
Both modes used the same executable, period, workloads, 120 frames per phase and
20 warmups. The candidate remained explicitly diagnostic; this was a native-host
experiment, not a modification to the normal Iced application.

A protocol written before measurement required three paired software-pan p95
improvements of at least 10%, nonoverlapping run ranges, no discarded/unresolved
feedback, no reduced rate and no repeated GPU regression. Three alternating pairs
per backend completed serially: software/1,200 rows/regular and GPU/100,000
rows/narrow, 16 sources. All 12 sessions resolved their 480 feedback records.
Latency was measured from the original timer deadline, so changing drawing start
could not manufacture an improvement by shifting the measurement origin.

| Software pan p95, due to presentation | Existing redraw ms | Direct dispatch ms |
| --- | ---: | ---: |
| Pair 1 | 44.553 | 35.997 |
| Pair 2 | 29.448 | 35.832 |
| Pair 3 | 31.092 | infinite: 22 discarded frames |

The GPU candidate also discarded 12 frames in pair 2 and three in pair 3.
Existing redraw discarded none across all six baseline sessions. All failures
remain in the analysis; discarded feedback is not dropped from percentiles.
Earlier dispatch reduced dispatch wait but did not reliably improve presentation.
The candidate was rejected. Previously rejected callback-only pacing was not retried.

## Update execution: narrower attribution, no proven stall

A copied phase probe split model validation, coordination, effects and flushing.
Each boundary recorded wall time, thread CPU time, rusage, user instructions,
user cycles and user reference cycles. An empty interval measured probe overhead.
Reference-cycle time used a bounded startup TSC calibration, approximately
2.904 GHz on this machine. This is an estimate based on this machine's counters,
not a portable conversion of arbitrary hardware events into elapsed time.
See the [Linux perf ABI](https://man7.org/linux/man-pages/man2/perf_event_open.2.html)
and [Intel performance monitoring guide](https://cdrdv2-public.intel.com/671378/335279-performance-monitoring-events-guide.pdf).

Six alternating history pairs completed: 1,200/100,000 rows, 16 Histogram sources,
fixed last price, CPU 5, 20 warmups and 2,000 measured append updates. Every run
retains 2,020 whole updates and 16,160 phase records. Instrumented medians were
20.455–26.350 microseconds. Boundary syscalls substantially perturb timing; these
numbers neither reproduce the original slow state nor count as acceptance.

For large-history pair 3, the flush phase illustrates the result. Comparing all
updates with the 100 slowest whole updates:

| Median quantity | All updates | Slowest 100 |
| --- | ---: | ---: |
| Flush wall time ns | 3,009 | 7,505 |
| Flush thread CPU time ns | 3,024 | 7,511.5 |
| Estimated user execution ns | 1,833 | 6,229 |
| User instructions | 8,282 | 8,282 |
| User cycles | 7,073 | 24,161.5 |
| Cycles/reference cycles | 1.338 | 1.338 |

This phase had zero recorded faults or context switches. The empty interval
remained approximately 1,363–1,368 ns with 448 instructions and about 625 cycles.
The increased active user cycles with stable instruction counts and clock ratio
weaken kernel-wait and frequency-change explanations for these captured tails.
They do **not** identify a cache miss, memory stall or other specific engine defect.
System-time accounting granularity also prevents treating a zero rusage delta as
proof of no kernel activity. Earlier small-history append faults remain real and
explain only part of the tails.

## Repeated marker-spacing writes

Every model flush writes unchanged bar spacing through each series projection.
A second isolated candidate cached the synchronized spacing and invalidated it
when either built-in or custom series registration completed. Every flush still
checked the actual current spacing. This avoids missing coordinator-driven scale
changes; reacting only to selected scale effects would be less robust. The cache
adds only an optional value, with no new shared ownership or public API.

A behavioral test verifies spacing after zoom, append and both registration
paths. One executable compares cached and baseline modes, with identical layout
and no per-boundary counter probes. Its protocol fixes six alternating mode pairs
at each history size, 20 warmups and 2,000 measured updates, serially on CPU 5.
Keeping it requires at least 5% median improvement in five of six pairs **at both
histories**, nonoverlapping median ranges and no median/p95 regression over 10%.
The candidate failed that rule. Small-history median gains were 0.62%, 4.06%,
2.31%, 1.58%, 0.89% and 19.15%; large-history gains ranged from a 0.23%
regression to a 2.29% improvement. Both median ranges overlap. Three paired p95
comparisons regressed by more than 10%. The isolated 19.15% gain does not override
the other pairs. The cache was rejected and no cache code was retained.

## Acceptance and retained scope

The isolated scheduling example passed strict Clippy. The final update probe
passed strict Clippy and its counter arithmetic test. The marker synchronization
behavior test passed. Experimental build warnings and corrected source snapshots
remain in the local artifacts; no lint suppression was introduced.

Only this report, its curated summary and the task note are retained in the
repository. Raw logs, protocols, binaries and copied source manifests remain under
`target/frame-start-study/`, `target/update-spike-study/` and
`target/marker-spacing-study/`. Those ignored files are local evidence and are
not included in a Git checkout; summary hashes identify the retained recordings.
The update probe's final source snapshot is authoritative: its build helper alone
does not include the final type-alias/layout corrections.

No demonstrated production remedy means the unchanged acceptance sets were not
rerun to seek a pass. The previous update INCONCLUSIVE and native 31/32 first-set
result remain open. Normal mouse passes remain recorded; touchpad, touchscreen
and high-DPI checks remain untested. Conflation has not started.

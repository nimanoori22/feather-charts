# Live-update timing attribution and final mouse checks

Date: 2026-10-15. Starting checkpoint: `01391cb`. Automated acceptance remains
**OPEN** under the unchanged [policy](frame-acceptance-policy.md). This continues
[the implementation report](live-update-interaction-acceptance.md). No new engine
optimization or acceptance pass is claimed.

## Final normal-build mouse acceptance

The user tested the normal release `public_chart` with Histogram, the editable
box and live feed, separately with `ICED_BACKEND=tiny-skia` and `ICED_BACKEND=wgpu`.
Both sessions exited normally. For each backend the user reported:

> All checks passed at all three sizes; short-height recovery passed; window closed

The checks cover regular, half and the earlier problem size: wheel zoom, diagonal
plot dragging, price-axis wheel/drag, box moving/resizing, cancelling an edit,
release outside the chart, and shrinking to a short chart then enlarging it.
These size labels are user-observed; the normal build does not record their exact
pixel dimensions. This closes the available mouse interaction checks, including
the original software lag and short-height recovery. Touchpad, touchscreen,
high-DPI physical coverage and external input-to-photon measurement are untested.
No unavailable device is counted as a pass.

## Diagnostic build defect and correction

An isolated update-profiler copy requested a different sampling period, but Cargo
reused the earlier executable in the shared target directory. Its output still
reported the previous period. This invalidates that attempted comparison; its
logs, copied sources and executable remain in `target/acceptance-resolution/`.

Copied manifests now receive a unique diagnostic package version derived from
their project path. Only the copied manifest/lockfile changes. The IP diagnostic
prints its compiled package version, which the study runner verifies against the
build manifest. The existing real-Iced diagnostic builder uses the same helper.
Production manifests, dependencies and acceptance policy are not changed.

Diagnostic example targets also have distinct names, so building one cannot
replace the normal example executable in the shared target directory. A real
two-example build regression verifies that both executables keep their expected
output; disabling the name isolation makes it fail. The earlier normal update
executable was atomically restored from its frozen candidate and verified.

A two-project executable regression reproduces Cargo returning the old value when
package isolation is disabled, with source timestamps deliberately preserved. It
returns both expected values with package isolation enabled. This tests actual
build output, rather than merely checking that generated strings differ. RED and
GREEN logs are retained. This is a tooling correction, not an engine speedup;
it does not retrospectively change any earlier acceptance outcome.

## Cache-miss instruction sampling

Six alternating 1,200/100,000-row pairs exercised the existing public update
fixture: 16 Histograms, 20 warmups, 2,000 measured updates, fixed last price,
CPU 5. Counter enable/disable brackets the original timed update. Preparation and
glyph conversion remain outside it. The isolated executable carries
`diagnostic_only=true`, which the acceptance reader rejects. No root benchmark
or production API is changed.

The Linux x86_64 probe owns its descriptor and mapped ring by RAII. It copies only
committed records before publishing the tail, checks record bounds and overruns,
and retains non-sample records. The build verifies mmap/ioctl offsets against the
installed Linux perf header. Unit coverage includes wraparound, malformed/truncated
records, unexpected sample layout and overrun. File output uses fresh paths.

Precise-IP mode returned zero samples despite nonzero counters in both the actual
fixture and an independent continuous-memory probe. The usable study explicitly
requests non-precise sampling at a fixed period of 512 cache misses. It retains
3,375 measured IP records, including 13 kernel-mode skid records, plus 12 throttle
and 12 unthrottle records across the executions. Throttling and interrupt skid
prevent treating this as complete or exact instruction attribution. No records
are silently discarded from the raw logs. Enabled/running counter totals are
retained separately.

Instrumented update medians range from 6.489 to 7.003 microseconds and p95 from
10.313 to 11.685 microseconds. Allocator/library activity and several existing
update operations appear in slow-sample locations; no single actionable engine
operation is established. These new runs did not recreate the previous much
slower same-history state. They neither prove its cause nor replace the failed
uninstrumented comparisons.

Symbol resolution uses each executable's ELF PT_LOAD segments together with its
recorded process mappings; file offsets are not assumed equal to virtual
addresses. An initial offset-only interpretation was wrong for this executable's
text segment and is retained as `invalid-file-offset-*`. Corrected interpretations
are in `ip-study/{analysis,symbols}.json`, with the analyzer source beside the
study. Non-precise IPs remain hotspot leads, not faulting-load addresses.

## Correlated native scheduling

Two diagnostic executions used the already-frozen normal frame executable:
software/1,200 rows/regular and GPU/100,000 rows/narrow. No timer, drawing operation,
thread priority, CPU affinity, desktop setting or acceptance budget changed.
A parent observer attaches a per-thread Linux software-dummy perf event with
context-switch records and CLOCK_MONOTONIC timestamps. It briefly stops only its
new child before attaching, then resumes it before chart initialization. It never
stops the compositor or another application.

Raw scheduling records, frame feedback and identities are retained, including
final unmatched switch-out state. Unknown/lost records and invalid ordering fail
analysis. The output metadata is explicitly marked `diagnostic_draw=schedule`,
and the unchanged acceptance reader rejects it. Main-thread off-CPU intervals
are intersected with each measured frame's work interval; event-loop waiting after
submission is not misclassified as rendering cost.

The software trace contains 3,611 switch records and 1,805 complete off-CPU
intervals. Three measured pan frames exceed the presentation budget individually:

| Frame | Work ms | Thread CPU ms | Off-CPU during work ms | Presentation ms |
| --- | ---: | ---: | ---: | ---: |
| 274 | 17.777286 | 13.146717 | 4.469785 | 46.458550 |
| 324 | 9.873642 | 9.814027 | 0 | 36.592730 |
| 331 | 9.011655 | 8.869769 | 0.015088 | 36.597901 |

Frame 274's 4.47 ms off-CPU interval total is explicitly marked involuntary
preemption and accounts for most of its wall/CPU gap. The trace does not identify
which competing task preempted it. Frames 324/331 complete work within about
10 ms with almost no off-CPU time, so main-thread preemption does not explain
all presentation delay. Compositor/driver timing beyond submission is still
unattributed. Exact locked Iced wgpu source already requests maximum frame
latency 1; reducing a presumed default of 2 is not an available fix.

The GPU diagnostic contains 7,217 switch records, 3,608 complete intervals and no
measured presentation misses. Its pan work/presentation p95 values are
4.742011/13.536015 ms. A diagnostic reproduction without a miss cannot explain or
close the prior failing GPU sample or satisfy the three-set acceptance gate.
The software diagnostic's three individual misses likewise do not replace the
failed acceptance cohort's percentile result.

## Decision and reproduction

The build-reuse defect is fixed and mouse checks pass on both available backends.
No additional engine or frame-scheduling defect has been established. No
speculative engine change was retained and the fixed acceptance studies were not
rerun to obtain a passing result. The previous INCONCLUSIVE update result and
31/32 native first-set result remain the current automated acceptance outcomes.
Conflation stays behind those gates.

Build the IP diagnostic separately from all timing runs:

```sh
python scripts/profile_update_ips.py --output target/ip-diagnostic-fresh
FEATHER_IP_MAPS=target/ip-run.maps taskset -c 5 target/ip-diagnostic-fresh/update-ip-diagnostic --rows 100000 --series 16 --kind histogram --phase append --fixed-last-price --samples 2000
```

The map file and output directory must be fresh. This requires usable user perf
hardware events. For a native scheduling diagnostic (also not acceptance):

```sh
python scripts/profile_frame_schedule.py --binary target/live-update-acceptance/frozen-frame-profile --output target/frame-schedule-fresh --backend tiny-skia --rows 1200
```

Local raw artifacts, failed attempts, manifests, RED/GREEN logs and physical app
logs are under `target/acceptance-resolution/`. The next unresolved diagnostic
boundary is compositor/driver presentation timing for fast-work delayed frames,
and uninstrumented update cache/allocator variation that reproduces the original
slow state. Neither currently justifies a production code change.

## Validation and retained summary

Passed: 35 Python tests; the sampler's standalone wrap/record regression and
strict all-target Clippy; a fresh isolated fixture build and smoke recording that
verifies the executable's compiled identity, 120 raw update records and usable
IP records; normal-example preservation and execution after a diagnostic build;
formatting and diff checks. Both retained scheduling traces also pass
the tightened switch-order validator. Full engine test results remain recorded
at the earlier implementation checkpoint; this slice changes diagnostic tools
and records the normal-build physical results.

The [retained summary](timing-attribution-summary.json) records the observations
and hashes for the local protocols, raw scheduling records, interpretations,
diagnostic builds and normal physical-test executable. Every original automated
failure remains linked through the implementation report and historical studies.

## Further presentation and update diagnostics

The [follow-up report](presentation-update-follow-up.md) records bounded client
commit timing, a privileged compositor observation with an unresolved identity
gap, and pinned RDPMC whole-update/phase studies. No new production cause or fix
was established. Acceptance remains open; the earlier failures above stand.

## Verified continuation

The [verified timing report](timing-resolution.md) records the corrected PID
handshake, two captures with verified chart identity and reproduced late frames,
monitor-render/output timing, and a 30-execution page-fault study. The trace-buffer
fault artifact is fixed. Production timing acceptance remains open; no engine
remedy or repeated acceptance pass is claimed.

# Controlled autoscale cache study

Acceptance remains **OPEN**. A model-only reproduction demonstrates that data-cache
eviction can cause an autoscale slowdown at both tested history sizes. Moving
built-in series options into a Box made that operation slower and was rejected.
No production code or acceptance threshold changed. The
[summary](autoscale-cache-study-summary.json) retains measurements and hashes.

## Independent reproduction

An isolated copy of `44753a6` constructs DataLayer and ChartModel directly: 16
Histogram sources, histories of 1,200/100,000 rows, an 800×500 model and logical
range 20..100. It creates no public Chart, widget, layout engine, renderer, glyph
work or window. It repeatedly recalculates the right price scale, with immutable
series data. Every result must equal the original finite price range.

The diagnostic collects cache-line addresses of selected live allocations before
measurement. It validates x86 CLFLUSH support and a 64-byte line size. Each trial
first warms the actual operation, optionally evicts the selected data lines, then
measures the original operation. Eviction, prewarming and result assertions stay
outside the timed/counter interval. Prewarming is a diagnostic control, not a
proposed production fix. Live allocations remain owned for the entire recording.

The fixed protocol uses six alternating history pairs, CPU 5, 20 warmups and
2,000 measured observations **per condition**. Seven conditions rotate their
starting order each repetition; each execution retains 14,140 observations.
All twelve executions completed serially. The predefined causal screen required
all-cold median >=2× warm in every execution at both histories. All twelve met it,
with identical ranges and identical median user-instruction counts (6,940,
including counter-bracket overhead).

| Evicted data group | Median autoscale time across runs, µs |
| --- | ---: |
| None: warm control | 0.530–0.536 |
| Scale-level options/range metadata | 0.600–0.608 |
| Source adapters and handle array | 1.530–1.824 |
| Marker/primitive headers and marker spacing | 1.496–1.723 |
| Visibility, provider, type and Histogram baseline fields | 1.672–1.969 |
| Series range/first-value caches and PlotList header | 1.532–1.747 |
| Union of all selected data lines | 2.723–3.162 |

Warm median generic cache misses are zero; deliberately cold trials have many
more. This intervention establishes a cache effect in this reproduction, beyond
the earlier observational correlation. Its magnitude is consistent with part of
the earlier autoscale tails. It does **not** establish that these lines were cold
in every original failure, explain all whole-update variation, or establish a
history-size regression. Larger history is not a prerequisite for this effect.

The groups overlap physical lines. For example, the small-history smoke fixture
has 28 shared lines between the range-cache and decoration groups, and 12 between
options and decoration. Complete counts/overlap matrices remain in each raw log.
The groups therefore cannot be added together or interpreted as independent costs
of individual fields. CLFLUSH also creates an artificial cache state; it is not a
measurement of a normal frame's exact cache residency. Software cache contents and
price-range results are preserved; only hardware cache residency is disturbed.

## Storage-layout candidate: rejected

Two frozen copies compare inline options with `Box<SeriesOptionsMap>` for built-in
Series. Moving the large options value was intended to make the main series object
more compact, without duplicating state or caching provider results. Each series
would own its box, which owns the existing options/callback handles; all existing
Rc ownership, borrowing, callback order and drop relationships remain intact.
Public method signatures and custom-series storage are unchanged. The alternative
of a new shared autoscale cache would require broader invalidation and callback
semantics, so it was not attempted.

The candidate's extra allocation and pointer indirection are real tradeoffs. They
must earn their cost in measurement. The fixed comparison uses six alternating
layout pairs at each history: 24 executions, the same seven conditions and 2,000
measured samples per condition. Keeping it requires >=5% cold-median improvement
in at least five of six pairs **at both histories**, nonoverlapping median ranges,
and no >10% warm median/p95 regression. Before recording, the protocol also requires
a separate normal update comparison before any production retention or acceptance.

The candidate fails the first gate. At 1,200 rows it is 8.91–36.01% slower in all
six cold-median comparisons. At 100,000 rows it is 23.83–37.73% slower in five pairs,
with one 3.41% improvement retained. Warm medians are 2.41–4.51% slower across all
twelve pairs. No failed pair or baseline outlier was removed. The extra indirection
and changed layout did not produce the intended benefit; no production change is
justified. The earlier combined-query and marker-spacing candidates were not retried.

All 387 library tests pass in the copied boxed implementation, including provider,
primitive, option replacement, visibility, custom-series and range-cache tests.
Five test-only direct option accesses needed explicit Box dereferences; assertions
and behavioral expectations were preserved. Initial build/test failures and their
corrections remain in the raw artifacts. No suppression or skipped test was used.

## Validation and retained scope

Raw identities, sample counts, cache-group conditions, result equality, counter
completeness, reported percentiles and source/binary/protocol hashes are validated.
The frozen release build manifests precede test-only dereference adaptations;
the summary separately hashes that final test source. The diagnostic builds retain
unused-helper warnings from their copied probe modules; no strict-Clippy pass is
claimed for them.

Only this report, its summary and the task note are retained in Git. Copied source,
build attempts, frozen binaries, protocols and raw logs remain locally under
`target/autoscale-cache-study/`. Those ignored artifacts are not distributed in a
Git checkout. The frozen copied sources/manifests are authoritative. Builders have
experiment-specific paths and must be adjusted to fresh paths for reproduction.

Per the predefined gate, the rejected layout did not advance to the normal update
comparison or unchanged acceptance. No qualified production remedy means no
acceptance rerun. The earlier update INCONCLUSIVE/native 31/32 outcome, remaining
device checks and conflation remain open. This study establishes cache sensitivity
and rejects one concrete storage change; it does not claim a timing fix.

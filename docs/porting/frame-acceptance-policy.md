# Live-append performance acceptance policy

Adopted for the user-authorized acceptance follow-up, 2026-10-15. This explicitly
replaces the decision procedure for the historical timing exception; it does not
change the 10% growth limit or erase any previous measurement.

## Update timing

Use the frozen release fixture: 16 Histogram sources, 1,200/100,000 starting rows,
fixed viewport/last-price formatting, 20 warmups, six alternating history pairs,
2,000 measured updates per execution. Preserve every raw sample and artifact hash.
The normal original three-run/100-sample matrix remains a historical screen.

- PASS requires all six larger-sample update median and p95 growth ratios <=10%.
  Complete 2,000-sample same-history controls must be recorded, using consecutive
  run pairs. Controls qualify attribution when a history screen exceeds its limit;
  they do not add a noise allowance to the limit.
- A larger-sample exceedance with a same-history control exceedance is
  INCONCLUSIVE about history attribution, never a pass. Preserve the raw failure.
- An exceedance without a failing same-history control is FAIL for this acceptance
  pass and requires profiling. The control does not establish causality by itself.
- Missing, nonfinite, truncated, or mismatched data is INCONCLUSIVE, never a pass.

The fixed repetitions and complete controls are an empirical acceptance rule,
not a claim of 95% statistical confidence or zero latency variation. Short
100-sample windows remain reported alongside their same-history controls; they
cannot alone establish a history-size regression.

## Native chart frames

The target is 60 Hz: frame work p95 <=16.666667 ms and update/scripted-input to
compositor-reported presentation p95 <=33.333334 ms. These budgets represent one
frame of work and at most two frame periods of event-to-presentation latency.
The actual monitor refresh rate, dimensions and native scale must be recorded.
This target does not claim performance at a faster monitor's maximum refresh.

Use the same chart/widget draw and Iced renderer/compositor as the demos in a
minimal native host. Measure CPU preparation, canvas geometry, render and
submit, plus Wayland presentation feedback for each submitted chart frame.
A redraw event, present() return, screenshot or GPU submission is not presentation.
The fixture excludes the demos' surrounding control-panel layout.
A four-pixel-square frame stamp forces visible damage on every submission, even
when an offscreen append leaves the historical viewport's formatted labels unchanged.

Before acceptance runs, freeze the release binary and protocol. Run three sets
with alternating backend/history order, both available renderers, starting histories
1,200/100,000, regular and narrow sizes, 16 Histogram sources. Each execution has
four phases: append, wheel zoom, plot pan and price-axis wheel, each with 20 warmup
and 100 measured frames. Every frame appends a bar. Scripted input uses the same
ChartProgram event handling as the native widget. Preserve warmups and all results.
Complete each set, then stop if it fails or is inconclusive; further sets require
investigation. A passing acceptance decision requires all three complete sets.

- Each phase/cohort must meet both p95 budgets using sorted index floor(n*.95).
- A compositor-discarded frame counts as infinite presentation latency; it cannot
  be removed from the percentile. Report the discard count.
- Missing feedback, wrong sample counts, unavailable presentation protocol,
  unexpected resize/scale changes, or backend fallback is INCONCLUSIVE.
- A measured budget exceedance is FAIL; investigate and fix the measured operation
  before another predefined acceptance set. Retain earlier failed attempts.

## Closing the exception

Close the historical timing exception only when the update rule and every native
frame cohort PASS. Link all original and follow-up evidence and record the decision
explicitly. FAIL or INCONCLUSIVE leaves the exception open. Physical mouse,
touchpad/touchscreen feel and input-to-photon testing remain separate hardware
acceptance; this policy does not mark those tests passed.

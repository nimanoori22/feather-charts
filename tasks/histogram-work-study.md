# Histogram work-count investigation

Date: 2026-10-17. User-authorized follow-up to the phase profiling report.
Scope: distinguish extra Histogram refresh/projection work from differing
execution time in the captured fixed-visible-range CPU fixture.

- [x] Trace Histogram refresh/preparation consumers, check coverage, and read source.
- [x] Add thread-local counters only to an isolated library; test reset, source
  separation, inactive capture and thread isolation.
- [x] Verify a one-series/16-series smoke capture and complete per-source rows.
- [x] Validate isolated library tests and analysis tests before timing.
- [x] Freeze reference/off/on order: three repetitions, both histories, 18 total
  executions, 20 warmups and 2,000 measured samples each on CPU 5.
- [x] Compare the exact per-series count vectors of the same total-tail frames
  against normal frames; preserve all raw data.
- [x] Report every >5% overhead-control flag and all uncertainty. Do not retry
  acceptance or claim a production performance remedy from diagnostic timings.

Counter scope: snapshot preparation only; refresh attempts, preparation calls
(including early exits) and successfully projected points. This does not measure
font/cache behavior, instruction counts, GPU work or actual presentation.

The private diagnostic module uses per-thread Cells with 16 fixed source slots,
not atomic globals. This matches the synchronous fixture and avoids synchronization
and per-bar instrumentation. Only an isolated source copy exposes the module;
production engine types, APIs, ownership and behavior stay unchanged. Records are
preallocated and written after the run. Counter collection is optional; the
unmodified frozen reference also measures the hook footprint absent in off/on
comparisons of a single instrumented executable.

The finite batch is diagnostic. A control flag leaves unperturbed timing
qualification INCONCLUSIVE. Exact observed work counts can still describe these
captures; they cannot establish the historical regression's cause or absence of
extra work elsewhere in preparation.

Result: all 12,000 measured frames have the same per-series vector; 0/12 overhead
flags. See [report](../docs/porting/histogram-work-study.md). Historical cause and
acceptance remain open. No production engine change or acceptance retry.

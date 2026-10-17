# Next feature: conflation rendering

Date: 2026-10-17. Development priority after the bounded
[timing/load check](../docs/porting/histogram-timing-load.md).

Detailed implementation roadmap: [conflation-plan.md](conflation-plan.md).
Ordered execution checklist: [conflation-todo.md](conflation-todo.md).

The user asked to move toward remaining features. Further timing investigation
is deferred; existing acceptance failures, budgets and device gaps remain OPEN.
This supersedes the old roadmap's requirement to finish timing acceptance before
starting conflation development. It does not close that milestone. The diagnostic change did not include conflation implementation. It has now been
implemented separately; see [the execution report](../docs/porting/conflation-rendering.md).

1. **Establish the source contract.** Inspect complete upstream conflation code,
   real preparation/rendering consumers and tests with graph coverage/source
   fallback. Record which series actually support it, aggregation rules, pixel
   bucketing, whitespace, extrema, crosshair/hit identity and update invalidation.
   Inspect existing Rust support before defining new types. Do not choose
   aggregation semantics or a first series from assumptions.
2. **Implement the first supported series end to end.** Compare viable Rust
   designs and ownership, define the API, then implement the smallest derived
   render representation required by upstream behavior. Preserve original data
   and exact update/query semantics. Keep preparation synchronous and retain
   current behavior when conflation is disabled.
3. **Complete supported-series and interaction integration.** Extend only to
   upstream-supported families and trace real consumers for markers, primitives,
   hit tests, crosshair, autoscale and retained frame snapshots. Test dense and
   sparse data, whitespace, viewport/scale changes, append and historical updates.
   Follow the repository's full porting workflow before each implementation slice.
4. **Verify rendering and reduced work.** Use reference-derived behavioral tests,
   both rendering backends and matched dense-pixel fixtures. Measure work saved
   and timing with fixed controls. Preserve fidelity and identities; do not claim
   an optimization from changed work that violates the source contract. Format,
   test, check and run Clippy, then document supported limits and remaining gaps.

Each slice must be reviewable on its own. Keep historical performance acceptance
separate; feature completion must not relabel it. Resume the deferred diagnosis
for reproducible interaction lag, incorrect engine work or a repeatable matched
regression, and use the existing acceptance policy for any eventual closure.

# Autoscale invalidation parity

Status: implementation and validation in progress. Timing acceptance remains OPEN.

## Source and required behavior

The inspected source components are `src/model/price-scale.ts` and `pane.ts`, with
real callers in `chart-api.ts`, `chart-model.ts`, `series.ts` and `chart-widget.ts`.
Source graph searches/traces were followed by direct reads for unresolved generic
calls and changed Series coverage. The relevant Rust counterparts are PriceScale,
Pane, ChartModel, ChartDataCoordinator and the public Chart owner.

ChartApi sends all updates to the model and recalculates all panes, including
appends outside the viewport. Series.setData also invalidates its pane and updates
the crosshair. Pane recalculation invalidates scale ranges. PriceScale stores the
latest visible interval and marks it invalid; `_makeSureItIsValid` consumes that
invalidation before invoking source callbacks. Range, emptiness and coordinate
queries force calculation. Frame axis/series preparation and an active crosshair
are real consumers of those queries. Multiple invalidations without a query
coalesce. Reading between updates remains a synchronization boundary.

This preserves dynamic provider/primitive callbacks at the next demand, even for
offscreen appends. It does not justify skipping an update solely because its bar
is offscreen. The baseline Rust implementation computed all ranges immediately during flush; that
is a demonstrated timing/ordering mismatch, independent of the cache experiments.

## Design and ownership

Two alternatives were considered: defer at ChartModel with an explicit mutable
flush barrier on every consumer, or defer inside PriceScale and resolve from its
existing read methods. The first would require callers to synchronize manually
and risks stale values through existing immutable queries. The second preserves
query signatures and matches real upstream consumers, so it is selected.

PriceScale owns a `Cell<Option<RangeImpl<TimePointIndex>>>` for pending work and
Cells for copyable range/margin state. It consumes pending work before callbacks,
matching upstream recursive-read behavior. Existing Rc/RefCell source ownership
and callback ownership stay intact; no new references, subscription cycles,
threads or asynchronous runtime are introduced. Marks keep their borrowed-slice
API, with a dirty flag to clear/rebuild their cache on mutable access. Marks
invalidation/generation must remain observable through existing queries.

The important internal operations are:

```rust
pub(crate) fn invalidate_price_range(&mut self, visible: RangeImpl<TimePointIndex>);
fn ensure_price_range(&self);
pub fn price_range(&self) -> Option<PriceRangeImpl>;
pub fn coordinate_transform(&self, base: f64) -> PriceCoordinateTransform;
```

The public explicit recalculation methods retain their eager boolean contract.
ChartModel's normal model-wide pass now invalidates each pane. Public
Pane.recalculate and explicit reset/forced scale operations remain eager. No callback identity, JS shared mutable range
objects, manual destruction or asynchronous abstraction is added.

## Behavioral verification

Ten new regressions cover unread updates and final-state demand, reads between
updates, active crosshair demand, unchanged sources after offscreen appends,
custom series and primitives, recursive reads, source removal/drop, latest
viewport selection, manual-mode transitions and marks/margins. Three of the
initial four model tests failed against the baseline before implementation.
The standalone `scripts/check_lazy_autoscale_reference.cjs` exercises actual
local upstream PriceScale methods and verifies callback ordering/coalescing.

`cargo fmt`, `cargo test --offline --all-targets` (397 library tests plus target
suites), `cargo check --offline --all-targets` and
`cargo clippy --offline --all-targets -- -D warnings` pass. Both rendering
backends pass all 768 cached-versus-direct pixel comparisons, including short
heights and fractional scale ratios. All 24 before/after live Histogram captures
(regular, narrow and burst, four ratios, both backends, primitives attached)
are byte-identical. GPU device access required an escalated
rerun after the sandbox could not initialize the renderer.

## Files and scope

- `src/model/price_scale.rs`: pending interval, lazy range resolution and marks
  invalidation; existing explicit eager recalculation APIs preserved.
- `src/model/pane.rs`: internal autoscale invalidation pass.
- `src/model/chart_model.rs`: normal flush defers range calculations.
- `src/model/chart_model/tests.rs`, `src/model/chart_model/tests/lazy_autoscale.rs`,
  and `src/model/price_scale/lazy_tests.rs`: behavioral regressions.
- `scripts/check_lazy_autoscale_reference.cjs`: original-source contract checks.

No dependency, thread, async runtime or additional cache of provider results was
added. This ports lazy invalidation semantics, not all remaining PriceScale
behavior. Existing Rust empty-range behavior and logarithmic calculations remain
outside this change. Full update/frame timing acceptance and untested physical
devices remain separate gates; conflation remains gated on their resolution.

## Timing evidence

A normal release baseline was frozen at `e148fde` before engine edits; candidate
source and executable hashes are stored alongside it in
`target/lazy-autoscale/{baseline,candidate}/manifest.json`. The predefined
comparison uses six alternating baseline/candidate pairs at both histories,
2,000 samples per execution, CPU 5, 16 Histogram sources and fixed last-price
formatting. It retains update, snapshot and glyph samples plus sample-aligned
total work. The first runner stopped on a phase-name parser assertion; its log
is preserved in `comparison-parser-abort/`, and the corrected complete run uses
the same thresholds and run order in `comparison/`.

The performance rule is **NOT MET** at either history. At 1,200 rows only 3/6
pairs improve update median by at least 10%; at 100,000 rows only 4/6 do, and
six-run median ranges overlap at both histories. Update median ranges in
microseconds are:

| Starting rows | Baseline | Candidate |
|---|---:|---:|
| 1,200 | 6.742–8.425 | 5.735–10.118 |
| 100,000 | 6.505–8.525 | 5.689–10.288 |

Sample-aligned total-work median changes range from -5.17% to +8.99%.
One 100,000-row pair has a +32.10% total p95 regression; the other five total
p95 changes at that history are negative. This result is retained without a
causal claim. The repeated-total-regression guard does not fire (0/6 flags at
1,200, 1/6 at 100,000), but the primary improvement rule fails. This is evidence
for retaining a behavior correction, **not a demonstrated performance remedy**.
No acceptance retry follows this comparison; the previous update and native
frame failures and unchanged thresholds remain in force. The per-frame fixture
reads a range after every update, so coalescing unread updates does not eliminate
its one required autoscale calculation per frame.

The checked-in [summary](lazy-autoscale-summary.json) records decisions and raw
artifact hashes. Raw samples and frozen binaries remain in the artifact root.
The existing [acceptance policy](frame-acceptance-policy.md) is unchanged.


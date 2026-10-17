# Series drawing order

Date: 2026-10-17. Status: implemented on 2026-10-18; automated checks pass,
source and capture verification pass; native mouse acceptance is pending.
Checklist: [series-order-todo.md](series-order-todo.md).
Baseline: `6503c80` (completed dynamic pane management).

Feature-specific files follow the repository's planning convention. Earlier
`plan.md`, `todo.md` and their unfinished historical acceptance records remain intact.

## Outcome

Applications can query and change an existing series' position within its current
pane. Built-in and custom series work across calendar, strike and maturity charts.
Overlapping geometry, annotations, selection and price-axis formatting respond
consistently, without recreating series or copying their historical data.

Scope includes runtime API methods, lifecycle integration, demo buttons, behavioral
tests, source comparisons and software/wgpu acceptance. Mouse-driven arrangement,
batch ordering APIs, persistent layouts, new events and export are separate work.
No new dependencies, threads or asynchronous engine machinery are expected.

## Findings and evidence limits

Preliminary codebase-memory searches, bidirectional traces and direct source reads
covered the files below. Coverage metadata reported no recorded issues for these
paths; this is best-effort evidence, not proof that all consumers were discovered.
Dynamic TypeScript calls need direct source searches as well as graph traces.

Upstream local Lightweight Charts 5.2.1:

- `src/api/{iseries-api,series-api}.ts`: public zero-based query and setter.
- `src/model/pane.ts`: setter clamps the target, removes/inserts the existing
  source, renumbers z-orders, invalidates caches and refreshes side-scale formatters.
- `src/gui/pane-widget.ts`: rendering can temporarily raise the hovered series;
  filled-series background and foreground passes have distinct behavior.
- `src/model/pane-hit-test.ts`: selection uses reverse stable source order, with
  distance, priority and primitive-layer arbitration. Temporary hover raising must
  not make equal-distance selection sticky.
- `tests/e2e/graphics/test-cases/api/change-series-order.js`: reorder three areas,
  assert indices, and change a price-scale assignment without changing pane order.
  Different custom price formatters make the formatter consequence visible.

Existing Rust foundation:

- `src/model/pane.rs` already implements `set_series_order`, ordered-source caches,
  z-order renumbering and formatter refresh. Its unit tests exercise mixed sources.
- `src/model/price_scale.rs` selects the visible lowest-z-order formatter source.
- `src/api/pane_management.rs` and
  `src/model/chart_model/pane_management.rs` coordinate existing owner movement.
- `src/api/chart.rs` owns the per-pane view lists and frame preparation.
- `src/ui/multi_pane_frame.rs` currently builds snapshot layers by iterating the
  supplied views. Changing model order alone would leave the displayed order stale.
- `src/ui/{hit_testing.rs,line_chart/series_drawing.rs}` and
  `src/api/primitives.rs` already implement hover raising, layer-based annotation
  drawing and reverse stable selection. Preserve their existing arbitration.
- `src/api/yield_curve.rs` explicitly forwards mutations and owns private support
  data. Its support-series index semantics require a source probe before API freeze.

Full source components, imports and real consumers must still be inspected in
Task 1 before production changes. This plan is not a claim of completed parity.

## Proposed contract and API

Use the existing chart-owned `SeriesId` interface:

```rust
impl<B: ChartBehavior> Chart<B> {
    pub fn series_order(&self, id: SeriesId) -> Result<usize, ChartError>;
    pub fn set_series_order(&mut self, id: SeriesId, order: usize)
        -> Result<(), ChartError>;
}
```

Calendar and OptionsChart receive the generic implementation. YieldCurveChart
gets explicit forwarding with private support protected. The signatures are
proposed until Task 1 freezes the maturity index contract.

- Order is pane-local, zero-based and mutable after insertion, removal or transfer.
  Larger indices normally draw later; hover raising and primitive layers remain
  independent rules. No user-facing global z-index is introduced.
- Clamp oversized targets, including `usize::MAX`, to the final valid position.
  Rust's unsigned index excludes JS negative/fractional/NaN inputs. Unknown or
  removed public IDs return the existing typed error rather than JS's `-1` sentinel.
- Validate before mutation. Identity and clamped-identity calls preserve owners,
  active captures and frame state; native no-op behavior is explicitly documented.
- Existing data, IDs, custom handles, options, scales and attachment ownership
  survive reordering. Adding/removing/transferring a series maintains the resulting
  source-compatible order. Reordering does not move panes or alter horizontal data.
- Refresh order-dependent formatters and invalidate settled layout when axis text
  changes. Verify left/right and overlay behavior against source; document any
  existing native extension rather than silently calling it exact source parity.
- Rebuild stable hit/layer associations before the next frame. Hovered-on-top
  changes presentation without changing the persistent queried order.
- Preserve immutable prepared frames across reorder/removal/drop.
- Freeze an active-edit policy: prefer existing cancellation machinery for an actual
  order mutation if a capture references a layer or stale hit; reject/no-op calls
  do not cancel. Cancellation must happen once and never commit an edit implicitly.

The completed source probe confirms that maturity support occupies an order slot.
The implementation counts it while rejecting its ID at the public API boundary.
See the [porting report](../docs/porting/series-order.md) for the final contract.

## Design and ownership

Two viable approaches:

1. **Model-owned order with coordinated view permutation (preferred):** reuse the
   existing pane operation, consume its effects through ChartModel, then align the
   chart's existing view list by ID. All downstream frame paths get the same stable
   order. Mutation costs depend on series count, not row count.
2. **Model-owned order with preparation-time lookup:** keep view storage unchanged
   and traverse it by model order during preparation. This avoids permuting storage
   but requires careful lookup/borrowing and separate handling of every path that
   assumes the first view is the first layer.

Choose the smallest approach after tracing all consumers in Task 1. Do not add a
second independently mutable z-order registry. Avoid sorting on every input event
or scanning historical rows. Any ID lookup/permutation is bounded by pane sources.

```text
Chart owns ChartModel -> Pane -> existing series/price-source adapters
Chart owns per-pane views -> existing custom view adapters
Chart owns attachments -> stable series/pane owner IDs
Prepared frame owns immutable geometry/render snapshots
```

Only ordering references/entries changes. No new owners, strong cycles, shared
locks, handle reconstruction or manual destroy protocol are needed.

## Implementation sequence

### Task 1 — Freeze behavior and baseline

Read complete upstream series API, pane ordering, render/hit consumers and relevant
price-scale/cache dependencies. Trace construction, add/remove, scale changes and
pane transfer. Inspect existing Rust equivalents and relevant original tests.
Read exact local Iced/Flowsurface code only if new widget/event handling is needed;
the expected implementation reuses the current demo host and controls.

Extend the existing real-browser source-probe approach to record initial order,
front/back/middle moves, oversized targets, multiple panes, transfer/remove/add,
custom series, hover raising and formatter leadership. Probe maturity support
slots, detached IDs and same-position operations. Pin source hashes and preserve
any source defect observations separately from deliberate native behavior.
Record alternatives, ownership and final API before implementation.

**Files:** new `docs/porting/series-order.md`, source/state fixture JSON and
`scripts/check_series_order_reference.cjs`; reuse existing browser tooling.
**Depends on:** none. **Size:** medium.
**Exit:** source contract and frozen baseline are reproducible; unsupported or
ambiguous behavior is explicitly resolved rather than guessed.

### Task 2 — Deliver calendar ordering end to end

Start with a failing public regression for two overlapping built-in series. Add
ChartModel coordination, query/setter and view-order alignment using existing pane
logic. Consume light-update effects, refresh formatters and invalidate stale hit
state. An order change must reach rendering on the next normal prepare call.

**Likely files:** `src/api/pane_management.rs`,
`src/model/chart_model/pane_management.rs`, `src/model/pane.rs` if needed,
new `tests/series_order.rs`.
**Depends on:** Task 1. **Size:** medium.
**Exit:** query/set/order, oversized clamp, unknown ID and no-op cases pass; the
rendered overlap and source state fixture agree. No data/owner reconstruction.

### Task 3 — Complete custom and numeric chart integration

Verify generic OptionsChart support and explicitly forward YieldCurveChart methods
with the frozen support-slot policy. Test custom handles, existing data, updates,
price-scale reassignment, target-pane arrival order and index shifts after removal.
Verify reorder-to-front/back followed by pane move/swap/transfer/removal/reset.

**Likely files:** `src/api/yield_curve.rs`, `tests/series_order.rs`,
`tests/pane_management.rs`; shared API/model files only if these tests expose gaps.
**Depends on:** Task 2. **Size:** medium.
**Exit:** all three chart families and built-in/custom owners preserve source
ordering semantics; hidden support cannot be mutated through a guessed public ID.

### Checkpoint A — Public behavior

Source/state comparisons and focused ordering/pane tests pass. Review the API and
ownership diff before continuing; split further if shared changes become large.
Save a local implementation checkpoint. No push is included.

### Task 4 — Verify drawing, hit selection and formatting

Add behavioral tests for equal-distance overlaps, non-tied priority/distance,
hover raising on/off, filled-series passes, primitive top/normal/bottom layers,
ordinary/temporary markers and price lines. Test differently formatted series on
left/right/overlay scales, hidden/empty series and reorder after warm caches.
Test real active box edits, stale tokens and viewport captures under the chosen
mutation policy, plus old snapshots after reorder and chart drop.

**Likely files:** `tests/series_order.rs`, new
`tests/series_order_interaction.rs`, existing hover/primitive regression suites.
Only fix shared UI/model code when a failing behavioral case proves it necessary;
separate such fixes into focused slices before broadening verification.
**Depends on:** Task 3. **Size:** medium per test/fix slice.
**Exit:** visible order, selection, labels and attachments agree with the source
contract; no stale layer index, retained capture or custom-view destruction.

### Task 5 — Extend the existing pane demo

Reuse `examples/pane_management` rather than build another host/editor. Add selected
series Back/Forward/To back/To front controls and display its current pane/order.
Include overlapping filled and custom geometry, distinct formatter examples and a
hover-raising toggle so consequences can be checked. Preserve existing live feed,
markers, pulse, watermark, pane controls and shared box editor.

**Likely files:** `examples/pane_management.rs`,
`examples/pane_management/host.rs`, `examples/pane_management/captures.rs`, README.
**Depends on:** Task 4. **Size:** medium; split scene changes from controls if needed.
**Exit:** controls work in calendar/strike/maturity modes, including after transfer
and removal; stale selected IDs fail safely or select an existing owner explicitly.

### Task 6 — Capture and native acceptance

Freeze a matrix before changing capture expectations: three chart families,
tiny-skia/wgpu, regular/narrow/short sizes, DPR 1/1.25/1.5/2, and four states
(initial, reordered, hovered, transferred-then-reordered): 288 feature images.
Reuse the previous 96 before-image comparisons for unchanged fixtures. Assert the
actual backend; inspect representative overlaps, formatter changes and clipping.
Reuse capture infrastructure and split script changes from fixture additions.

Ask the user to exercise software then wgpu demos: reorder built-in/custom series,
toggle hover raising, transfer/reorder, remove/reset, test live zoom/diagonal pan,
both price-axis gestures and box move/resize/Escape at regular/narrow/problem
sizes, then shrink/enlarge. Record each response; do not infer physical acceptance
from generated captures. Hardware not available remains explicitly untested.

**Likely files:** existing capture script, capture fixtures, verification report.
**Depends on:** Task 5. **Size:** medium per capture/native slice.
**Exit:** frozen matrix and old baselines pass; both renderer mouse runs pass or
the milestone stays open with exact unresolved observations.

### Task 7 — Final verification and report

Run focused tests and source probe during each slice; once implementation settles:

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo test --doc
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
git diff --check
```

Run existing source comparisons affected by pane, hover, markers, primitives,
formatters and numeric/custom behavior; run the complete existing comparison set
for the final checkpoint. Preserve original acceptance criteria and failed evidence.
Do not change timing thresholds or rebaseline unexplained visual differences.

Complete the report with source components, consumer requirements, contract,
discarded JS conventions, alternatives, ownership, exact files, test/capture/native
results and remaining dependencies. Update checklist and README. Save local commits;
push only when requested.

**Depends on:** Task 6. **Size:** small documentation/verification slice.
**Exit:** automated and available mouse acceptance pass with evidence linked.
Historical timing exceptions and unavailable touchpad/touchscreen/high-DPI physical
checks retain their prior status; simulated DPR captures do not close device gates.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Model/view order diverges | One coordinated mutation path; inspect snapshot IDs and overlap pixels. |
| Formatter changes cause axis jitter | Warm-cache tests, settled layout and narrow/short recovery captures. |
| Hover raising changes stable selection | Separate stored order from transient draw passes; tie and non-tie tests. |
| Attachments retain old layer indices | Rebuild layer mappings; exercise markers, primitives and frozen frames. |
| Support series shifts maturity indices | Freeze actual source behavior before public API implementation. |
| Reorder destroys/rebuilds a custom view | Identity/drop/copy counters and updates after reordering. |

No user preference blocks planning. Maturity support indexing and exact edit
cancellation behavior are engineering investigations assigned to Task 1.

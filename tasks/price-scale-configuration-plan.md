# Public price-scale controls and runtime configuration

Date: 2026-10-18. Status: proposed; implementation not started.
Baseline: `ea7198a`. Checklist: [price-scale-configuration-todo.md](price-scale-configuration-todo.md).
This uses the repository's feature-specific planning convention; existing plans
and independent unfinished acceptance records remain intact.

## Outcome

Applications can inspect a pane's price range, choose a manual vertical range,
enable or disable autoscaling, and update the supported chart presentation and
navigation settings without rebuilding the chart or its series. Calendar,
strike/options and maturity/yield charts expose the same core capabilities.

Example workflow: read the blue series' current pane and scale, set a fixed range,
continue live updates without losing it, change grid/font/axis settings, then
restore autoscale. Existing series IDs, data, typed handles, annotations, plugin
owners and retained frames survive every configuration operation.

New public events, screenshots/export, attribution-logo rendering, arbitrary
horizontal behavior factories and mouse-driven pane rearrangement are separate
milestones. Renderer color space and custom parser construction are not runtime
settings. Browser DOM sizing/listeners do not become Rust engine APIs; Iced owns
window bounds. Earlier drawing-order, timing and untested-device exceptions remain
open independently.

## Preliminary investigation

Graph discovery/traces and direct source reads establish the following starting
points, rather than a completed option-by-option parity audit:

- Upstream `src/api/price-scale-api.ts` was read completely. `setVisibleRange`
  disables autoscale and sets a custom range; `getVisibleRange` can return null.
  In logarithmic mode it converts the internal range and rounds to minimum price
  increment precision. `setAutoScale` routes through scale option application.
- Upstream `src/model/chart-model.ts` applies chart patches to existing panes and
  the time scale; price-scale option application also affects stored defaults.
  Per-pane axis operations and chart-wide patches have different target scopes.
- Upstream `src/api/chart-api.ts` rejects runtime color-space/parser changes in
  development. `src/gui/chart-widget.ts` additionally handles wheel listeners,
  pane layout, time-axis visibility and browser sizing. Model and widget share
  configuration in JavaScript; Rust should not reproduce that aliasing.
- Original tests include `tests/unittests/price-scale.spec.ts` (normal/log reads),
  graphics `price-scale/set-visible-range.js` and `toggle-auto-scale.js`, and
  coverage `price-scale/change-auto-scale.js`.
- Rust `PriceScale` already implements custom-range state, autoscale, lazy range
  calculation, conversions and option patches. Reuse it rather than create a
  parallel range model. `price_range_impl.rs` already defines public `PriceRange`.
- `Chart::apply_price_axis_options` already validates margins before cancelling
  viewport interaction and routes through `ChartModel`. Time-scale patches and
  crosshair replacement also exist. `ChartModel::set_layout_typography` already
  propagates font size and invalidates rendering.
- `PaneOptions` already includes left/right/overlay scale defaults; existing
  `PriceScalePosition` represents left, right and named overlays. Reuse that
  selector where practical. YieldCurveChart explicitly forwards mutating methods.
- Layout and grid option types exist; this work must connect supported runtime
  fields to rendering and cached measurements, not merely store new values.

Graph coverage reported no recorded issue/metadata match for the cited Rust and
source production files. Rust graph call edges for the public patch method were
sparse; actual implementations were read directly. This is bounded planning
evidence, not proof of complete consumer coverage. Task 1 completes the full
source/consumer/design workflow before production changes.

## Contract to freeze before implementation

1. Range reads target the requested current pane/scale and return owned values or
   `None` when no range exists. They may resolve the existing lazy range cache but
   do not prepare a frame or create a new viewport update.
2. Manual range setting disables autoscale and survives data updates, horizontal
   navigation and resize until a source-defined reset, gesture or mode transition
   changes it. Re-enabling autoscale must permit subsequent recalculation instead
   of leaving the custom-range guard stuck. Disabling autoscale freezes the
   appropriate current range; it is not synonymous with fitting data once.
3. Probe setter and getter independently in Normal, Logarithmic, Percentage and
   IndexedTo100 modes. Do not assume the source getter's log conversion is also
   the setter's conversion, or that percentage/indexed units are raw prices.
   Pin mode transitions, rounding and zero/negative values to real source output.
4. Validate pane/scale and range before any mutation or interaction cancellation.
   Proposed Rust policy: finite endpoints with strictly increasing bounds; typed
   errors for nonfinite, reversed or zero-width ranges. Confirm and document any
   deliberate difference from source permissiveness, including extreme finite
   spans that overflow calculations. Unknown overlays must not be created by reads.
5. Range targets are pane-owned scales. Moving a series to another pane adopts
   that destination's scale state; a manual range must not follow series data as
   though it were an annotation. After pane reorder, indices resolve current panes.
6. Option patches preserve unspecified fields. A patch with several sections is
   preflighted as a whole: invalid input must not leave some sections applied.
   Commit through existing invalidation/effect paths after validation; avoid one
   full recalculation/flush per field. Equal/empty patches preserve active state.
7. Appearance-only updates preserve captures and active edits when coordinates
   remain valid. Viewport/layout changes use the existing cancellation policy,
   stop stale kinetic motion and reject stale hit/edit tokens. Freeze the precise
   categories in Task 1; do not cancel every gesture just because text color changed.
8. Chart-wide scale defaults reach existing and future panes as demonstrated by
   source consumers. Per-pane scale updates affect their target; document whether
   and how their source default side effects map to native creation defaults.
9. Font, background, grid and axis changes refresh all affected pane, axis, label,
   watermark and crosshair snapshots. Existing immutable frames remain drawable
   and unchanged after new options, pane removal and chart drop.
10. No data replacement, series recreation, new async machinery, dependencies or
    shared mutable configuration ownership is needed.

## Proposed API and design

Signatures are provisional until Task 1 freezes naming, errors and source units.
Reuse existing exported range/selector/patch types, re-exporting where needed:

```rust
impl<B: ChartBehavior> Chart<B> {
    pub fn visible_price_range(&self, pane: usize, scale: &PriceScalePosition)
        -> Result<Option<PriceRange>, ChartError>;
    pub fn set_visible_price_range(&mut self, pane: usize,
        scale: &PriceScalePosition, range: PriceRange) -> Result<(), ChartError>;
    pub fn set_price_scale_auto_scale(&mut self, pane: usize,
        scale: &PriceScalePosition, enabled: bool) -> Result<(), ChartError>;
    pub fn price_scale_options(&self, pane: usize, scale: &PriceScalePosition)
        -> Result<PriceScaleOptions, ChartError>;
    pub fn apply_chart_options(&mut self, patch: ChartOptionsPatch)
        -> Result<(), ChartError>;
}
```

Keep existing axis/time/crosshair/navigation setters compatible and route common
behavior through shared implementation. Do not expose `&mut ChartModel` as a
shortcut. Provide owned configuration readbacks for exactly the new supported
patch sections, without promising an easily cloned copy of construction callbacks.

Alternatives:

| Design | Tradeoff | Choice |
|---|---|---|
| Chart methods with pane/scale selector | Existing ownership and error conventions; resolve live state per call; no retained pane pointer | Preferred |
| Long-lived scale handles | Convenient chained calls, but new identity/lifetime and pane-removal semantics would be needed | Defer |
| Separate setters only | Simple individual calls, but no atomic multi-section updates | Retain compatibility; use shared validation/commit for aggregate patches |
| Replace complete construction options | Resets unspecified values and mixes renderer setup with runtime state | Reject |

Ownership remains `Chart -> ChartModel -> Pane -> PriceScale`. Patches and query
results are owned values. Existing typed series handles and frame snapshots keep
their current ownership; no new Rc/RefCell graph or callback owner is introduced.
Explicit YieldCurveChart forwarding protects its scaffold and avoids exposing
mutable dereferencing. Numeric horizontal precision/formatters retain existing APIs.

## Runtime configuration scope

Task 1 publishes a field-level table: source field, native field, current support,
new setter/patch/readback, affected caches, validation and applicability. The
following sections are the deliverable, not an unbounded promise to port every
browser option:

| Section | Runtime capability |
|---|---|
| Layout appearance | Solid/gradient background, text color, font size/family |
| Grid | Independent horizontal/vertical visibility, color and line style |
| Price scales | Existing mode, inversion, margins, labels/visibility patches; chart defaults plus explicit pane/scale targeting |
| Horizontal scale | Reuse existing patch type; preserve numeric behavior and maturity scaffold policies |
| Crosshair | Reuse existing typed settings; refresh line, labels and hit snapshots |
| Navigation | Reuse existing mouse/touch/kinetic option types and setters; fill public routing gaps found by the audit |
| Pane layout | Reuse stretch/preserve APIs; propagate supported separator styling/resize configuration, with actual widget use verified |

Construction-only fields are absent from runtime patches. Attribution-logo drawing
and renderer color-space switching are explicitly excluded; unsupported fields must
not be exposed as successful no-ops. Exact public patch composition follows the
audit, preserving existing setter semantics and distinguishing per-chart defaults
from one-pane changes. Any newly discovered independent feature is recorded as a
separate dependency, not silently counted as implemented.

## Ordered implementation slices

Each slice adds focused behavioral tests and a verified local checkpoint. Suggested
file groups are estimates; split any slice that grows beyond about five files.

### 1. Freeze source behavior and API contract — no dependencies; medium

Read complete relevant APIs and direct dependencies, trace actual consumers across
model/widget/public/plugin layers, and inspect original tests. Complete the option
inventory and source unit/rounding probes above. Check exact Iced widget and
Flowsurface patterns only where option propagation reaches input/layout/rendering.
Pin source hashes and before-change compatibility captures. Publish the chosen
API, alternatives, ownership, validation, invalidation and gesture policy.

Likely files: new `docs/porting/price-scale-configuration.md`, source/state JSON,
`scripts/check_price_scale_configuration_reference.cjs`, these plan/checklist files.
Verification: real local 5.2.1 browser probes produce reproducible stored results;
review covers empty/log/special modes and existing/new pane defaults. No production
changes until the contract and native deviations are explicit.

### 2. Public range read/set — depends on 1; medium

Use existing public PriceRange and scale lookup/state. Add model mutation routing,
typed validation and invalidation, then public read/set methods for built-in and
existing overlay scales. Verify manual retention, both sides, empty targets,
shared scales, invalid target/range and lazy/log reads.

Likely files: `src/api/price_scale.rs` (new if useful), `src/api/mod.rs`,
`src/model/chart_model.rs`, `src/model/price_scale.rs`, `tests/price_scale_configuration.rs`.
Verification: focused tests fail before implementation, pass afterward; compare
deterministic values and source scenes. Existing axes tests remain passing.

### 3. Autoscale transitions and frontend parity — depends on 2; medium

Expose explicit enable/disable and options readback, ensure custom-range guards and
lazy recalc recover correctly, and forward all mutations through YieldCurveChart.
Test updates/pop/append under manual and automatic modes and all three frontends.

Likely files: new public scale module, `src/api/yield_curve.rs`, existing scale/model
files only if proven needed, public range tests. Verification: source transitions,
double-click restore, mode changes, maturity support and subsequent updates pass.

Checkpoint A: APIs, errors, source units and owner lifetimes reviewed; save a
tested local commit before adding broader configuration.

### 4. Runtime appearance patches — depends on 1 and 3; medium

Add typed layout/grid patches and readback, preflight validation and model commit
routing. Propagate typography and color changes to all current panes and future
defaults; refresh grid/background/axes. Reuse existing color validation and caches.

Likely files: new `src/api/configuration.rs`, layout/grid option modules,
`src/model/chart_model.rs`, new `tests/chart_configuration.rs`.
Verification: partial updates preserve other fields; mixed invalid input leaves
state and snapshots unchanged; gradients/fonts/grid visibly update; old frames stay
identical. Split patch-type work and rendering integration if this exceeds scope.

### 5. Aggregate scale/crosshair routing — depends on 4; medium

Connect existing horizontal/price-scale patches and crosshair settings to the
aggregate API and readbacks. Prevalidate all sections before committing; retain
per-pane methods and verify the source's current/future pane default behavior.

Likely files: public configuration module, chart/model routing, yield forwarding,
configuration tests. Verification: aggregate and corresponding individual operations
produce equivalent supported state; invalid later sections do not mutate earlier
ones; repeated patches do not accumulate margins or reset viewport/data.

### 6. Navigation and pane widget propagation — depends on 5; medium

Wire supported navigation and pane-layout options through existing typed controls
and widget snapshots, filling audited gaps only. Test style-only preservation,
geometry-change cancellation, disabled gestures, kinetic stop and fresh re-enable.
Verify separator styling/resize options affect the widget rather than only state.

Likely files: configuration module, existing navigation/pane widget modules located
by Task 1, configuration/interaction tests. Verification: actual Iced event tests,
correct capture ownership, no stale gesture/edit resurrection. If pane presentation
and input require separate file groups, land them as two tested sub-slices.

Checkpoint B: all runtime sections/readbacks and three frontends reviewed against
the inventory; no exposed no-op fields; save tested local checkpoints.

### 7. Lifecycle regressions — depends on 3, 5 and 6; medium

Test ranges/configuration after pane reorder/removal and series transfer/order,
built-in/custom empty/pop/update, live pulses/markers, watermarks, crosshair and
box editing. Verify scale ownership, no payload copying on configuration, no series
recreation, retained-frame immutability and short-height recovery.

Likely files: public configuration tests, existing pane interaction/edit/overlay
tests. Verification: focused integration tests plus unchanged compatibility suite;
invalid/no-op calls preserve valid active state under the frozen policy.

### 8. Demo controls — depends on 7; medium

Extend the existing generic pane-management host with an opt-in configuration mode.
Include pane/scale selection, range readout/set/narrow/widen, autoscale on/off,
mode/inversion/margins, background/font/grid/crosshair, navigation and supported pane
controls. Reuse shared host and scrollable toolbars in all three chart modes.

Likely files: `examples/pane_management.rs`, new configuration submodule, host and
capture submodules, README. Verification: demo tests cover controls, reset,
invalid targets/ranges, live updates and recovery; both backend builds succeed.

### 9. Captures and physical mouse acceptance — depends on 8; medium

Freeze capture scenes before final replay: automatic, manual, live update while
manual, restored autoscale and a combined appearance patch, plus log/mode-specific
deterministic cases. Exercise calendar/strike/maturity, actual tiny-skia/wgpu
backend assertions, regular/narrow/short sizes and DPR 1/1.25/1.5/2. Reuse the
existing capture runner; record the exact count/protocol rather than moving the
baseline after a failure. Rerun unchanged compatibility captures and all existing
source comparisons alongside the new oracle.

Likely files: capture helper/wrapper, demo captures and evidence report/manifest.
Verification: inspect representative images and retained-frame comparisons. Ask
the user to test both backends in all three modes: manual range survives feed,
autoscale restores, options redraw immediately, wheel/diagonal drag/price-axis
wheel/drag and box move/resize/Escape work at regular/narrow/problem sizes; test
short shrink/enlarge and invalid-input recovery. Record actual responses and
normal exits. Unsupported physical devices remain untested, explicitly.

### 10. Final verification and documentation — depends on 9; small

Run `cargo fmt --check`, `cargo test --all-targets`, `cargo test --doc`,
`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings` and
`git diff --check`. Complete the porting report with source/consumers/contract,
discarded TS conventions, ownership/alternatives, exact files, test evidence and
remaining dependencies. Review unchanged consumers and document public examples.
Save final local commits; push only when requested.

## Completion gates and risks

The milestone is complete only when all supported patch sections have real effects,
range/auto semantics match frozen source expectations or explicitly documented
native validation, compatibility checks pass unchanged, and available mouse
acceptance passes on both backends. Never relax prior thresholds to close it.

| Risk | Mitigation |
|---|---|
| Source getter/setter units differ by mode | Probe independently before API implementation; retain actual expected values |
| Custom range prevents future autoscale | Test enable/disable/mode transitions after update, pop and resize |
| Stale font/axis/grid snapshots | Propagate through existing invalidation and compare real output/readback |
| Some patch sections apply before another fails | Whole-patch validation and behavioral no-partial-mutation tests |
| Pane index changes target the wrong scale | Resolve current pane/scale each call; test reorder/transfer/removal |
| Broad patch scope grows without limit | Field inventory and explicit construction-only/excluded fields; split slices |
| Configuration unnecessarily cancels edits | Distinguish appearance from coordinate changes; assert both policies |
| Defaults differ for future panes | Compare source chart-wide/per-pane routing and assert creation behavior |

No installation is planned: reuse the existing Rust, browser oracle and capture
tools. No human design input is currently required; implementation must resolve
source edge cases in Task 1, and physical checks require the user's mouse later.

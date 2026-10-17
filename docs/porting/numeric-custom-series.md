# Custom series on numeric horizontal axes

Status: implemented; automated verification and software/GPU mouse acceptance passed
on 2026-10-17. Baseline 2a32573. Plans:
[numeric-custom-series-plan.md](../../tasks/numeric-custom-series-plan.md) and
[checklist](../../tasks/numeric-custom-series-todo.md).

## Source and real consumers

Investigated complete custom view/renderer contracts, custom definition construction,
ChartApi.addCustomSeries -> _addSeriesImpl, inherited YieldChartApi construction,
custom pane preparation/hits, DataLayer custom writes and DataConflater context
creation. Numeric upstream type tests require original numeric items, typed custom
options and rejection of calendar or built-in data mismatches. Exact source revision
and working-tree hashes are in numeric-custom-series-source.json. Graph coverage
was checked; changed upstream icustom-series.ts was read directly. No upstream
working-tree edits are made.

Custom bars use logical indices; original data/events preserve prices or maturities.
Value builders drive autoscale/current price; callbacks receive current visible data,
options and optional price conversion. Custom payload and renderer hit data remain
separate. Existing synchronous ownership and mixed-series ordering are retained.

Yield addCustomSeries calls _addSeriesImpl directly, bypassing overridden addSeries:
custom defaults keep last-value/price-line visibility true, unlike yield Line/Area.
Chart-wide yield localization still takes precedence over series price format.
Custom writes must settle the private whitespace scaffold and preserve source extent
semantics. Native finite keys and the existing scaffold bound remain deliberate
validation policies applied before canonical mutation.

## Design and ownership

Keep CustomSeriesHandle<V> and infer item/internal types from V. An alternative
CustomSeriesHandle<V,B=TimeBehavior> was rejected because it repeats types already
fixed by the view and complicates caller inference. The public methods match V to
Chart<B>; pointer accessors match V::Item to PointerEvent<T>. Private cache rows use
the same view-associated types, cloning internal items instead of requiring Copy.
The existing erased UI adapter, model and widget are reused without a second engine.

Chart/model and pane adapter strongly own the model; adapter strongly owns OwnedView.
External handles own model data but hold a weak view. Frames own frozen renderers
and hit geometry. No new owner back-reference/cycle is introduced. Model borrows
are dropped before view callbacks; destroy remains a resource hook invoked once by
OwnedView::drop. Rust ownership replaces shared JS options/lifetime conventions.

## Contract and compatibility checkpoints

The new actual-source runner executes 18 numeric custom DataLayer states (nine per
family), real custom definition/API methods and inherited yield defaults. Its base
chart observation boundary is a narrow stub, not browser DOM integration. Source
state fixtures preserve union membership, whitespace, historical/append writes,
shortening, peer removal and clear. Existing non-associative reducer comparisons
remain part of the unchanged conflation oracle.

Pre-change evidence: target/numeric-custom-series/before, 96 images: 24 each calendar
built-ins, calendar custom (normal/hover/narrow at four DPR on two backends), price
built-ins and yield built-ins. Native source/binary/fixture hashes are retained.
Feature matrix is frozen at 240 images: two numeric modes, two backends, three sizes,
four DPR and five scenes (sparse, mixed, updates, dense, empty).
Work limits retain the existing 100,000-row visible-payload bound (170 copies) and
small-update bound (four), plus bounded precompute/reducer counters.

Behavioral RED: price Chart initially had no custom add/set operations. Generalizing
the API exposed an empty-data regression in calendar preflight (Time(EmptyData));
empty replacement must skip converter selection, as the original DataLayer does.
That was fixed without weakening the existing calendar tests.

## Additional compatibility findings

Calendar converter preflight preserves the original Data(Behavior(...)) error
variant. The regression was demonstrated before the mapping fix; empty replacement
also keeps its original converter-selection behavior.

Yield defaults had a pre-existing exact-half formatting mismatch: 0.0625 produced
0.062% in Rust rather than the source 0.063%. The new custom localization regression
exposed it. The existing price-chart fixed-decimal formatter was moved unchanged
into src/formatters/fixed.rs and reused by default yield localization for finite
values. Explicit caller localization overrides remain intact; the former nonfinite
fallback is retained. Both signs and two halfway values are checked against source.
This also corrects the shared default for existing Line/Area curves and support.

The demo's dense-to-sparse reset exposed the retained yield domain squeezing the
shorter curve into the left edge. Reset now focuses the current data with the
existing logical-range API; the engine's source-compatible retained domain is
unchanged. Both demo mode tests guard this viewport behavior.

## Files changed

Production Rust:

- src/api/custom_series.rs: view-derived handle/adapter types, behavior-generic
  operations/preflight/errors, typed event accessors and wrong-family doctests.
- src/api/custom_series/conflation.rs: generic cache/job rows and cloned internal
  context items, preserving the existing reducer/order/work logic.
- src/api/yield_curve/custom_series.rs: explicit custom operations, domain dirty
  marking and chart-wide localization without the Line/Area visibility override.
- src/api/yield_curve.rs: module registration and exact default fixed-decimal format.
- src/formatters/{mod.rs,fixed.rs}, src/model/horz_scale_behavior_price.rs: share the
  already source-tested formatter across price and yield defaults.

Examples/checks/documentation:

- examples/numeric_custom_series.rs: one UI with price/maturity modes and controls.
- examples/numeric_custom_series/{host.rs,view.rs,captures.rs}: local explicit
  public-API forwarding, one immutable interval renderer and deterministic captures.
- tests/numeric_custom_series.rs and numeric_custom_series/support.rs: source-state,
  pointer/hit, invalid write, ownership, default/localization, reducer and work tests.
- tests/public_custom_series.rs: calendar error compatibility regression.
- scripts/check_numeric_custom_series_reference.cjs and numeric-custom-series-
  {source.json,states.tsv}: actual-source contract and native comparison.
- scripts/capture_numeric_custom_series.py: both modes/backends and old-family pixels.
- README.md, this report and tasks/numeric-custom-series-{plan,todo}.md.

No dependency, vendor renderer, general widget or generic model rewrite was needed.

## Behavioral checks

Nine numeric integration tests cover both families, 18 source DataLayer states,
custom-only yield growth, retained source extent on clear/removal, typed original
pointer and hit payloads, whitespace, transactional invalid/order/foreign/detached
writes, cleanup with retained handles, formatter/visibility overrides and exact
halfway rounding. Custom defaults preserve ordinary last-price/price-line visibility.
Wrong-family charts and pointer access are rejected by two compile-fail doctests.

Dense checks include a non-associative reducer, original/internal numeric contexts,
latest replacement, missing-reducer fallback and dense/sparse transitions. Existing
calendar conflation/source comparisons remain unchanged. Numeric warmup is bounded
at 128 reductions per preparation, including invalidated partial work. The numeric
100,000-row case retains limits of 170 visible payload copies and four update copies.
Generic implementations add no Copy/Send/Sync requirements or persistent chart borrow.

Both demo mode tests exercise hover payloads, latest/history/whitespace, options,
clear/reset/remove/re-add, dense/sparse and short-height recovery. The example's
intervals implement custom drawing/hits; existing primitive editing stays on its
separate attachment API. Shared widget input and the built-in reference peer/pulse
are reused. Captures verify immutable pixels after updates/removal/chart drop.

## Validation and remaining acceptance

The final production full suite passed 692 tests across 66 suites, with no failed
or ignored tests (`target/numeric-custom-series/all-targets-tests-final.log`). Two
compile-fail doctests passed. Both demo tests passed again after the viewport fix.
Formatting, all-target compilation and strict Clippy checks pass. All 29 source
oracles pass (`target/numeric-custom-series/oracles-final/results.json`).

A prior concurrent full-suite run exited on SIGTERM (status 143) after 623 passing
tests and no assertion failure; its log is retained. No engine cause was established
from that interruption.

The final capture matrix passes: 240 feature images and 96 exact pre-change
compatibility comparisons. Immutable snapshots retain their pixels after update,
removal and chart drop. Source/binary/fixture/font identities and per-image hashes
are in `target/numeric-custom-series/final-viewport/{protocol,results}.json`.
Physical mouse acceptance on 2026-10-17:

| Backend | Strike mode | Maturity mode |
| --- | --- | --- |
| tiny-skia | User reported all checks passed; normal exit | User reported all checks passed; normal exit |
| wgpu | User reported all checks passed; normal exit | User reported all checks passed; normal exit |

Checks cover sparse latest/history/whitespace, style/labels, empty/reset,
remove/re-add, original/hit values, dense-to-sparse reset, live-feed wheel zoom,
diagonal pan and price-axis wheel/drag at regular, narrow and the earlier problem
window size, plus short-height recovery. The toolbar scrolls horizontally. History
while conflated retains its intentional source-compatible rejection. The custom
intervals expose hits rather than editable primitives; no box editor is added.
Native logs are `target/numeric-custom-series/mouse-{price,yield}-{tiny-skia,wgpu}.log`.
Historical timing acceptance and unavailable touchpad/touchscreen/physical high-DPI
checks remain separately open.

No related component blocks the planned numeric custom-series scope. Arbitrary
horizontal-behavior factories, non-unit common metadata and custom-specific pulse
or up/down-marker extensions remain separate features. Existing calendar behavior,
numeric built-ins and the built-in peer pulse remain supported. No dependencies
were added, and these local checkpoint commits have not been pushed.

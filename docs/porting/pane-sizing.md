# Persistent pane sizing (milestone E)

Baseline: `917c28b`. Implementation in progress; physical acceptance pending.

## Source and consumers

The complete `src/api/pane-api.ts` delegates `setHeight` to
`ChartModel.changePanesHeight`. The source model converts the requested height to
stretch allocation using the current total pane height and total stretch. It
clamps the target between 30px and the total minus 30px per other pane. Other
panes share the difference sequentially in pane order, with the same clamps.
This order matters when a neighbor reaches its minimum; it is not proportional
redistribution. `ChartWidget._adjustSizeImpl` subsequently applies those factors,
rounding to device pixels and giving the last pane the remainder, with a 2px
layout floor. The source graphics test `panes/change-height.js` and pane tutorial
exercise the public setter after initial layout. Pane move/swap retains each
pane's allocation; removal redistributes space according to remaining factors.

Graph discovery, model traces and exact snippets were supplemented with actual
source consumers because caller edges for `changePanesHeight` are sparse.
Coverage checks for all relied-on source and native paths reported no recorded
issues (best-effort evidence, not a completeness claim).

## Design, alternatives and ownership

Two viable implementations were considered: retaining explicit pixel constraints
in the layout owner, or converting each request into the existing stretch factors.
The latter preserves source behavior on window/divider resizing and pane
reorganization. The former would require conflict resolution and would pin heights
on resize instead of scaling them. Calling the engine's existing `set_pane_height`
alone is insufficient: `MultiPaneChartFrameOwner` overwrites it with weighted
allocation during prepare.

The public API is `Chart<B>::set_pane_height(&mut self, pane: usize, height: f64)
-> Result<(), ChartError>`, with a mutable forward on `YieldCurveChart`. Calendar,
strike and custom domains use the same generic owner. The chart owns panes and
their existing stretch factors; temporary calculated factors are validated before
mutation. No references, callbacks, retained wrappers, threads or async machinery
are added. Source wrapper identity and manual lifecycle management are unnecessary.

## Contract and native limits

Requests use the last settled layout; applications prepare once before requesting
pixel sizing. The next prepare applies the new allocation; subsequent window
resizing scales its proportions. Finite negative heights clamp like source.
Invalid pane indices or non-finite inputs return errors before mutation or gesture
cancellation. A single pane is unchanged. Exact factor no-ops are unchanged.
Changed allocation cancels active viewport interactions, animation and box previews
through the existing owner boundary. Ordinary divider behavior is untouched.

Before first layout, or when total height cannot leave positive allocation after
neighbor minimums, sizing is inert. This avoids the source's zero-division or
negative-factor state at collapsed heights, and preserves allocation for recovery.
The source's ordinary 30px clamp and sequential redistribution remain unchanged
for positive feasible allocations. This native safety adaptation is explicit;
short-window results are not claimed to be exact source parity.

## Verification

The source probe bundles the actual local 5.2.1 implementation, freezes watched
source hashes and records actual heights through change, resize, clamp, move and
removal. Native tests compare the same states, with an explicit initial 2:1:1
allocation matching the source fixture. Red API compilation evidence is retained
under `target/pane-sizing/red.log` before implementation. Behavioral tests cover
persistence through repeated prepare, resize, structure changes, finite validation,
negative/oversized clamps, unprepared/collapsed recovery and navigation cancellation.

Files: `src/api/pane_management.rs`, `src/api/yield_curve.rs`,
`tests/pane_sizing.rs`, `scripts/check_pane_sizing_reference.cjs`, and source
fixtures `pane-sizing-{source,states}.json`. Demo and renderer evidence follow.

Previous timing, formatting-GPU, dedicated drawing-order and untested physical
device exceptions remain independent. E does not implement metadata, locales,
configured parsers or Display-P3 rendering.

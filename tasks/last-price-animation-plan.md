# Last-price animation implementation plan

Date: 2026-10-17. Status: implemented; automated checks and mouse acceptance pass.
Results: [port report](../docs/porting/last-price-animation.md).
Starting checkpoint: `cdf80a4`. Execution checklist:
[last-price-animation-todo.md](last-price-animation-todo.md).

## Outcome and scope

Implement the source library's last-price center dot and expanding pulse for
Line, Area and Baseline. Honor the existing Disabled, Continuous and OnDataUpdate
options. Keep canonical prices, data access, autoscale, pointer events and
conflation unchanged. Use the existing native redraw/Tick mechanism and keep
the engine synchronous.

Deliver a working Line slice first, then the two filled families, lifecycle
coverage, an executable demo and native verification on TinySkia and WGPU.
Candlestick, Bar, Histogram and custom-series animation are outside this source
feature. New public animation options, Tokio tasks, threads, callback registries
and a general animation framework are unnecessary.

The historical timing exception stays OPEN/deferred with unchanged thresholds.
This feature's animation checks do not close it. Unavailable physical devices
remain explicitly untested.

## Investigated source and current foundation

The complete upstream files read for this plan are:

- `src/views/pane/series-last-price-animation-pane-view.ts`: state, update
  handling, visibility, phase interpolation, canonical endpoint projection.
- `src/renderers/series-last-price-animation-renderer.ts`: center dot, fill,
  outer stroke, pixel-ratio rounding and drawing order.

Important direct consumers/dependencies were inspected in `model/series.ts`,
`model/data-layer.ts`, `model/colors.ts` and `gui/pane-widget.ts`. Graph traces
connect Series.setData to the update trigger and Series.topPaneViews to stage
invalidation. Direct reads supplement graph results: Series/DataLayer/Colors
coverage metadata reports changed files, so indexed bodies alone are insufficient.
The local upstream HEAD at planning time is
`8a7c94ff8ddffde68955db61ef8b58098f239de0`, with working-tree edits in dependencies.
Before implementation, finish the full consumer/lifecycle audit and freeze the
actual working-tree revision and hashes; do not silently substitute another
upstream revision.

In Rust, modes/options and SeriesUpdateInfo already exist. Series stores update
metadata and supports incremental/full data application. Chart::prepare_snapshot
receives an Instant; ChartMessage::Tick also carries one. The native widget
already schedules redraw deadlines. These are integration points, not evidence
that price animation itself is implemented.

Two current details constrain the design:

1. Chart::update currently routes Tick through full prepare. Adding a pulse to
   is_animating alone would repeatedly prepare chart geometry and axis text.
2. ChartProgram::draw deliberately clears its geometry cache and returns one
   chart geometry/damage boundary. It is not currently an unchanged-base cache.
   Do not blindly split the pulse into separate Canvas layers or remove that
   behavior; previous software interaction fixes depend on the rendering path.

## Behavioral contract to preserve

### Timing and modes

- Disabled draws neither dot nor pulse and requests no price-animation ticks.
- Continuous repeats a 2,600 ms cycle from its monotonic origin.
- OnDataUpdate starts a cycle when the source update flag qualifies. During an
  active cycle, ordinary updates do not restart it. If remaining time is positive
  and strictly less than 650 ms, extend the end by 2,600 ms, retaining the origin.
  When remaining time is zero or negative, a qualifying update starts anew.
- Source activity uses an inclusive end boundary. Test the terminal frame and
  subsequent clearing so the last pulse cannot remain stuck on screen.
- Cycle stages occupy 25%, 27.5% and 47.5%: radius 4→10, 10→14, then 14;
  fill alpha .25→0, then 0; stroke alpha .4→.8, .8→0, then 0.
- When OnDataUpdate expires, the center dot remains visible. Clearing all data
  removes it. Do not equate inactive pulse with absent marker.

### Events and identity

- Follow SeriesUpdateInfo.last_bar_updated_or_new_bars_added_to_the_right once
  per accepted data mutation. Do not reread a stored true flag on every Tick.
- The current TS DataLayer sets this flag for fulfilled historical updates too;
  full replacements can qualify through first/last time comparisons. Initial
  population without prior data does not automatically qualify. Whitespace and
  clear have distinct behavior. Freeze these cases in the source oracle instead
  of inferring behavior from the flag's name.
- Retain existing historical-update rejection at effective conflation factor >1.
  Rejected updates must not change animation state.
- Use the canonical global last price/index and its resolved line color, even
  if the last drawn conflated row uses another index or representative extreme.
- Draw only when the canonical last index lies in the strict visible range and
  the series has usable data/price conversion. Preserve first-value semantics in
  percentage/indexed modes; use source line color/width and row overrides.

### Rendering and ownership

- Center radius is max(2, line width × 1.5). Source pixel rounding corrects x,
  leaves y unrounded and uses horizontal DPR for circle radius. Stroke width is
  max(1, floor(horizontal DPR)); source alpha multiplies original color alpha.
- Preserve dot → pulse fill → pulse stroke order, clipping and top-pane order
  relative to crosshair, other series, raised hover outlines and top primitives.
  A pulse is decorative, not a new hit target or an autoscale contributor.
- Previously retained prepared frames stay frozen after time advances, data
  updates, option changes and removal. Drawing never samples a clock or model.
- Empty/hidden/offscreen series need not request useless native redraws, but
  suppressing work must retain the source's elapsed phase when visible again.
  Browser timeout identity/cleanup is incidental; visible behavior is required.

## Rust design and alternatives

**Preferred:** small synchronous timing state owned by each eligible Series;
owned circle commands in prepared frames; native Chart drives redraw deadlines.
This covers low-level model preparation and public Chart without duplicate
controllers. Record time at mutation/creation boundaries and pass explicit
monotonic timestamps to state transitions and sampling. Keep tests independent
of sleeps and wall-clock scheduling.

**Alternative:** a Chart-owned map keyed by SeriesId. This simplifies the initial
demo but duplicates model lifecycle rules and leaves low-level consumers without
equivalent state. Choose it only if the completed consumer audit proves a concrete
advantage and explains those consumers. Do not maintain both owners.

For rendering, compare a dedicated frozen animation payload with reusing existing
circle/stroke commands. Reuse primitives where their semantics fit; do not put
animation into crosshair-marker storage just to avoid a field. Audit public frame
struct literals before adding fields. A separate uncached Canvas overlay is a
viable alternative only after pixel/order and native damage behavior are verified.

```text
ChartModel -> existing Series owner -> optional timing state (values only)
Chart -> native Tick/deadline adapter
Prepared frame -> owned projected endpoint/style + sampled circle commands
Retained frame -> independent frozen commands; no reference to live timing state
```

No new Rc/Weak relationships or closures are needed. Removing the series drops
its timing state; retained frames own their old output. Do not introduce an
async runtime or new public clock/subscription API.

### Proposed private interface (confirm after audit)

```rust
struct LastPriceAnimationState { /* monotonic origin and end */ }
struct AnimationSample { radius: f64, fill_alpha: f64, stroke_alpha: f64 }

impl LastPriceAnimationState {
    fn new(now: Instant) -> Self;
    fn on_update(&mut self, info: Option<SeriesUpdateInfo>, empty: bool,
                 mode: LastPriceAnimationMode, now: Instant);
    fn is_active(&self, mode: LastPriceAnimationMode, now: Instant) -> bool;
    fn sample(&self, mode: LastPriceAnimationMode, now: Instant) -> AnimationSample;
}
```

Keep timing independent of Iced, prices and colors. Existing projection/color
utilities resolve a canonical endpoint; a pure renderer prepares dot/ring
commands. Preserve current public option names/defaults. Resolve the exact
clock handoff through Series update targets before changing signatures; prefer
private helpers to new public timed-update methods or generic clock services.

## Ordered delivery

```text
0 source/consumer contract -> 1 actual-source oracle / RED cases
  -> 2 model timing state -> 3 circle rendering
  -> 4 public Line slice -> 5 native scheduling / stage-only tick path
  -> 6 Area + Baseline -> 7 lifecycle and mixed-scene regressions
  -> 8 demo and bounded work checks -> 9 final validation / report
```

Tasks, dependencies, scope and verification commands are in the execution
checklist. Each task should touch roughly 2–5 files; split it before coding if
the actual impact grows. Module exports and downstream struct literals count.
Keep overlapping model, frame and widget work sequential. No delegation is needed.

Checkpoints: source contract after 0–1; pure timing/rendering after 2–3; native
Line after 4–5; series/lifecycle coverage after 6–7; acceptance after 8–9.
These are verification gates, not additional permission prompts.

## Verification strategy

1. Execute the actual TS state/view/renderer with fake performance.now and a
   recording canvas. Pin source hashes. Compare transition sequences, durations,
   alphas, radii, canonical center/color and drawing commands to Rust fixtures.
   Test stage boundaries and ±epsilon, cycle end/extension boundaries and bursts.
2. Test model update flags, clear/reload, option switches, rejection, whitespace,
   multiple series/panes and owner removal with explicit timestamps. Do not use
   elapsed-time sleeps as correctness assertions.
3. Verify all three families with raw/conflated data, dense-to-sparse zoom, pan,
   resize/short-height recovery, left/right and manual/inverted/log/percentage/
   indexed scales. Include markers, price lines, crosshair, top primitives and
   retained frame independence. Disabled output must remain unchanged.
4. Capture fixed animation times on TinySkia/WGPU at DPR 1/1.25/1.5/2. Compare
   source-derived geometry/color, within-backend repeatability and disabled
   reference pixels; do not require equality between different rasterizers.
5. Prove work before timing: animation-only ticks must not rebuild canonical
   history, conflation levels, autoscale, tick selection or glyph paths when
   nothing else changed. Work should scale with animated endpoints, not history.
   Verify coexistence with kinetic motion, touch deadlines and conflation warmup:
   ticks that also have real model/layout work must still take the full path.
6. Run one fixed diagnostic batch: Line/Area/Baseline, 8,192/100,000 rows, animation
   disabled/active, 800×500 and 300×300 sizes, both renderers, release profile. Use 24 matched pairs (48 serial executions), alternating disabled/Continuous
   order, with conflation enabled in both members, DPR 1, 20 warmups and 80
   measured frames at explicit 32.5 ms clock steps. Freeze commands, binary/source
   hashes and counter definitions before measuring;
   collect preparation/geometry/raster phases separately. Retain every result;
   no retries just to get a pass and no changes to prior acceptance budgets.
7. Run fmt, all-target tests/check and strict Clippy, affected source oracles,
   backend captures and final diff/ownership review. Repeat physical mouse zoom,
   pan and price-axis gestures with active animation/live feed on both backends.
   Record unavailable touch/high-DPI hardware as pending.

## Risks and decision gates

| Risk | Required handling |
|---|---|
| Trigger every Tick or miss incremental updates | Consume flags at the shared mutation boundary, once; audit both apply paths |
| Stop drawing when pulse expires | Separate center-dot visibility from pulse activity; render a terminal clearing frame |
| Move marker to a conflated extreme | Project canonical last data; contrast it with aggregate geometry in tests |
| Full prepare on every 16 ms pulse | Add a narrowly guarded stage-only path; prove work counts before accepting |
| Restore a known software damage regression | Preserve one-boundary behavior unless native evidence justifies changing it |
| Reorder top primitives/hovered series | Source-derived mixed-scene ordering tests before choosing overlay placement |
| Hidden/offscreen busy loop or phase reset | Suspend unnecessary native work, preserve elapsed origin, test reappearance |
| Nondeterministic clocks or stale frames | Explicit Instant inputs in core state; frozen drawing data and retained-frame tests |

Task 0 must settle clock propagation, frame placement and fast-path eligibility
before production edits. These are code investigation decisions; no user
preference or tool installation is required to start. Existing native profiling
fixtures and installed perf are available if the new animation reproduces lag.

## Completion and report

Publish `docs/porting/last-price-animation.md`: source revision/hashes, consumer
categories, required/incidental behavior, selected API/ownership, alternatives,
exact files, tests, backend/work results and remaining coverage. Link a demo
command from README. Mark implementation, automated parity, physical acceptance
and the separately deferred timing exception independently.

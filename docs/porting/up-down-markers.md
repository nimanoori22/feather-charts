# Up/down update markers port

Implementation complete; automated and software/GPU mouse acceptance pass.
Baseline `16f3622`. Source hashes are pinned in
[up-down-markers-source.json](up-down-markers-source.json).

## Source and required contract

The seven complete built-in plugin files and wrapper/adapter dependencies were
read. The tutorial and graphics/coverage fixtures seed a curve, then update
historical values relative to that initial data. The excluded coverage subtree
was read directly. This milestone uses date/time charts, not yield-curve scales.

Preserve seed-relative comparisons, unchanged baseline on update, untracked
appends, marker refresh insertion order, five-second default expiration, immediate
zero-duration expiration, indefinite manual markers and baseline reset without
implicit clear. Source source-over circle/chevron geometry, DPR correction and
normal series primitive ordering are required. Marker values are explicit, not
conflated/nearest values. No hit/autoscale/axis contribution.

Discard JS truthiness for a zero price baseline (Option distinguishes absence),
stale captured option objects (native colors update), and mutation before failed
data writes (native writes are transactional). Source duration=0 documentation
disagrees with executed timers: immediate expiration follows the code. These are
separate source/native fixtures, not silent parity claims. Invalid marker times
are rejected; valid but absent time keys are safely skipped while drawing.

## Design alternatives and API

Choose a Chart-owned controller with a typed ID backed by PrimitiveId, explicit
managed set/update calls, optional monotonic deadlines and existing series attachment
lifecycle. A host-owned wrapper was rejected: idle expiry and removal would require
repeated application work. No generic plugin registry or async engine machinery.

Private ownership: Chart owns a Controller (series ID, baseline time/value map,
strong Rc<RefCell<MarkerState>>). Its ordinary drawing attachment owns another strong
reference to MarkerState. Neither points back to Chart. Preparation copies only
visible coordinates/colors into a frozen renderer; frames retain no mutable state.
Generic replacement/removal of that attachment must also remove the Controller.
Dropping Chart/series/controller releases live state; old frames keep old geometry.

This narrow shared state avoids copying active marker vectors on every data update.
The baseline map contains only seeded key/value pairs, never duplicate full rows.
There is one active marker per key with insertion order, not a key-sorted renderer.
Deadlines are derived from active entries without a stale timer heap.

Public types: MarkerSign, UpDownMarker, UpDownMarkersOptions, UpDownMarkersId.
Chart methods: add_up_down_markers(series,options), set_up_down_data(id,data),
update_up_down_data(id,item,historical,now), up_down_markers(id,now),
set_up_down_markers(id,markers), clear_up_down_markers(id),
replace_up_down_marker_options(id,options), remove_up_down_markers(id).
UpDownMarkersId exposes its underlying primitive ID for generic lifecycle use.
All mutators validate before changing existing data/state.

Ticks expire markers before the price-pulse fast path. Preparation also advances
expiration for headless hosts. A public next_up_down_expiration() supplies the next
explicit deadline. The widget merges it with touch/animation requests; a waiting
marker alone does not enable 16-ms animation polling.

## Consumer mapping

Source wrapper setData/update maps to explicit managed Chart operations; ordinary
Chart data writes bypass the baseline tracker just as direct source series writes
do. Source manual/query/clear maps to typed marker commands. Browser timers become
monotonic deadline/Tick state. Source view uses the existing primitive context's
series price/time conversions; owned prepared geometry supplies both backends.
Series removal, generic primitive replacement/removal and pane movements reuse
existing attachment ownership and must clean managed tracking.

Validation, exact files, work bounds and native acceptance will be recorded here.
Historical timing acceptance stays OPEN/deferred; other device gaps remain separate.

## Verification and work bounds

Actual pinned TypeScript executes under a nonzero fake wall clock with fake timers,
a recording bitmap canvas and the real factory/adapter. The oracle compares 54
renderer cases (including fractional/anisotropic DPR and negative rounding) and 14
shared timeline states. Additional source-only probes identify the three native
corrections above. Factory, Line/Area, refresh order, manual persistence, whitespace,
appends, exact expiry and detach are exercised.

Behavioral REDs preceded rendering, managed updates, exact expiry, idle scheduling
and stale typed-ID cleanup. A stale typed ID must not remove a generic replacement
primitive. Compiling failures are retained separately and are not behavioral REDs.
Public tests cover invalid colors/data/times, unsupported series, hidden owners,
pane-index shifts, multiple controllers, series removal, frozen frames, duration
replacement/zero/overflow, idle scheduling and pulse fast-path coexistence. A dense
historical update is rejected without changing markers or their deadline; enabling
conflation alone does not reject sparse historical updates.

A 10,000-key baseline test repeatedly refreshes one key without replacing its
baseline storage or accumulating expired timer entries. New update work searches
the seeded map and active markers; expiry checks cached controller deadlines and
visits active entries only when due. No canonical history scan or duplicate row
storage is added to updates/ticks. Refresh currently scans active markers; indexing
is deferred until a measured large-active-set cost warrants it. Existing canonical
data-update costs are outside this claim. Frozen frames retain no live controller
state (verified with Weak after removal). Software updates plus image watermarks
retain one damage region and fresh pixels at three acceptance sizes.

Validation: 654 tests passed across 60 all-target suites, with no failures or ignored
tests. All-target check, strict Clippy and formatting pass. The final capture-only
viewport adjustment was then executed on both backends and checked/Clippy-checked.
Affected source comparisons also pass: conflation (182 aggregation and 60 selection
states), price animation (58 timing, four renderer and 12 endpoint/DPR cases), text
watermarks (270 states) and image watermarks (112 placement states).

The final native capture matrix passes 120 feature images and 24 byte-identical
default-off comparisons: two actual backends, DPR 1/1.25/1.5/2 and sizes
800x500/300x300/300x90. Each retained frame is drawn twice and again after clearing
live state. Expired output equals cleared output. Nondegenerate feature scenes
must differ from cleared output. Representative software/GPU direction, Area
historical, dense mixed-content and short-height images were visually inspected.
Direction/history/refresh/expiry scenes zoom to the last 40 logical intervals for
inspection; the mixed scene retains dense conflation. Historical Area edits disable
conflation explicitly. The default-off scene is unchanged.

Capture metadata and validation logs are in [evidence/up-down-markers](evidence/up-down-markers).
Raw captures remain under `target/up-down-markers/final`. Baselines reuse retained
pre-feature images only after verifying the exact original binary identity; the
baseline protocol records that provenance. The first runner used an incorrect
capture flag and both subprocesses timed out at the unchanged 120-second limit.
Those logs remain under `target/up-down-markers/after`; the corrected dense-view
matrix and final closer-view matrix both pass. This was a harness error, not a
chart timing failure. No acceptance threshold was relaxed.

## Files and related dependencies

- `src/api/up_down_markers.rs` and its private tests: typed API, baseline, state and
  lifecycle; `src/api/mod.rs` exports it.
- `src/renderers/up_down_markers_renderer.rs` and `src/renderers/mod.rs`: immutable
  circle/chevron geometry and native drawing.
- `src/api/chart.rs`, `primitives.rs`, `interaction.rs`, `widget.rs`: ownership,
  cleanup, explicit-clock expiry and idle deadline scheduling.
- `src/model/horz_scale_behavior_time/horz_scale_behavior_time.rs`: crate-visible
  reuse of the existing validated time-key constructor.
- `tests/up_down_markers.rs`, `tests/software_canvas_damage.rs`: public behavior,
  scheduling, mutation errors, lifecycle and software damage gates.
- `examples/public_chart.rs`, `examples/public_chart/up_down_markers.rs`: opt-in
  controls and deterministic native capture fixture.
- `scripts/check_up_down_markers_reference.cjs`, `scripts/capture_up_down_markers.py`:
  actual-source oracle and finite native matrix.
- `README.md`, this report/source manifest/evidence and the feature plan/checklist:
  public usage, provenance and acceptance status.

No new dependency, async runtime or vendored renderer patch is required. Existing
series primitives, time/price transforms, ordinary data mutations, Tick handling,
conflation and rendering supply the dependencies. Numeric/yield-curve charts and
other series types remain separate features. No public series-move operation exists;
this port verifies current pane-index changes/removal instead of adding one.

## Physical acceptance

On 2026-10-17 the user reported all checks passed on both explicitly selected
renderers: marker controls, wheel zoom, diagonal plot dragging, price-axis wheel/
drag and box edits at regular, narrow and the earlier problem size; short-height
recovery; five-second expiration without mouse movement after pausing feed/pulse;
and indefinite manual markers. Both windows closed normally (exit 0).

The first window was launched without an explicit backend and is excluded from
renderer-specific acceptance. Replacement windows used `ICED_BACKEND=tiny-skia`
and `ICED_BACKEND=wgpu`. The reported software/GPU results and scope are retained in
[evidence/up-down-markers/native-acceptance.json](evidence/up-down-markers/native-acceptance.json).
Touchpad, touchscreen and physical high-DPI checks remain untested. Historical
timing acceptance remains OPEN/deferred. Local implementation commits are
`e53198f` and `1d3b361`; no push was requested or performed.

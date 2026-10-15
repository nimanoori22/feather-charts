# Live-update interaction acceptance implementation

Work follows [the roadmap](../../tasks/live-update-acceptance-plan.md).
Acceptance remains OPEN. Historical measurements and failures are retained in
[the previous report](render-update-follow-up.md).

## Short-height source contract and design

Inspected upstream `model/price-scale.ts` coordinate conversion and
`model/series/{line-pane-view-base,histogram-pane-view}.ts`, plus
`renderers/histogram-renderer.ts`. The base pane view converts visible series
points through the price scale; the Histogram view independently converts its
baseline. Last-value/crosshair consumers and custom renderer adapters also use
price-to-coordinate conversion. Both point arrays and single-point upstream
conversion use the affine coefficient `internalHeight() - 1` even when negative.
Source-required clipping handles finite off-plot coordinates.

Rust's owned `PriceCoordinateTransform` adds an early NaN for a nonpositive
coefficient. This rejects otherwise finite small-height coordinates; Histogram
preparation correctly rejects the resulting NaN, which clears the prepared frame.
The failing coefficient regression reproduces this mismatch before modification.
The public two-pane test also checks fresh small-size startup and resize recovery.

Two viable fixes are suppressing each series view at small internal height or
preserving upstream affine conversion in the shared transform. Suppression adds
parallel policies, loses geometry unexpectedly and does not match upstream.
Choose shared affine conversion, retaining missing/degenerate range and invalid
input handling. Scrolling/inverse gestures retain their separate usability guards;
a finite forward projection does not imply an invertible gesture at zero span.

No public API or ownership change: the scale owns its options/range and produces
an owned coefficient snapshot, borrowed by synchronous geometry and hit consumers.
Existing `price_to_coordinate(&self, price: f64, base: f64) -> Coordinate` and
snapshot method signatures remain. Rust does not reproduce TS mutable class
identity or a lazy callback merely to implement the same formula.

## Actual-demo diagnostics design

Instrumentation must preserve the actual demo
controls and two panes, distinguish physical runtime integration from a diagnostic
host, and never pass submission timing off as compositor presentation.

The diagnostic integration now uses temporary source copies selected by Cargo
metadata (`iced_debug` 0.14.0 and the actually locked `iced_winit` 0.14.1), not
installed registry edits. `scripts/profile_public_chart.py` copies this project,
patches existing disabled timing hooks and the real pre-present callback, and
builds an isolated executable. Normal manifests/lockfile and acceptance binaries
are not patched. A replacement UserInterface/winit host was rejected because it
could change the event pacing under investigation. A permanent custom renderer
wrapper was rejected because it broadens production interfaces unnecessarily.

The copied runtime calls the already-existing presentation probe through a small
diagnostic helper crate. It retains `Arc<Window>` until guest Wayland objects drop,
with no owner cycle. This keeps native unsafe code outside Iced's safe runtime
crate. The trace is bounded, buffered and main-thread-only; overflow is explicit.
Both the span clock and compositor clock must be Linux CLOCK_MONOTONIC. Forward
projection, chart geometry, UI layout/draw, actual rendering/present calls, raw
app-observed mouse events, physical/window scale and chart/plot bounds are recorded.
An input's next submitted surface is only a candidate association, not proof that
that frame displays the input; no physical input-to-photon claim is made.

The first physical trace and initial smoke attempt were invalid: recorder struct
update syntax constructed a temporary with a Drop implementation, which flushed
an empty file during initialization. A file-enabled RED regression demonstrated
this; explicit field initialization fixes it. The main-thread ownership guard
also excludes renderer worker spans. The corrected six-frame startup smoke saves
five resolved presentations and one unresolved final submission (intentional
immediate diagnostic exit); raw records retain that unresolved frame. No smoke or
instrumented run is an acceptance result. All failed builds/traces remain under
`target/live-update-acceptance/`.

## Physical trace and bounded canvas damage design

The corrected actual-demo trace reproduces lag at 941×1030 physical pixels,
not the guessed 300×500 replay size. Of 1,858 submissions, 1,857 feedbacks resolve,
zero discard and one remains unresolved at close. Rendering/present CPU p95 is
117.512 ms (wall 117.836 ms), versus chart preparation p95 1.488 ms. The worst
425.174 ms present span contains 279 renderer spans totaling 421.012 ms. Input
arrival to the next submitted surface has a 211.473 ms candidate p95; that
association is not proof the surface contains the input. Raw results remain
instrumented diagnostics, not acceptance.

Exact Iced tiny-skia 0.14.1 `Layer::damage` expands live geometry groups into
per-primitive bounds; `Renderer::draw` iterates every damage region and redraws
intersecting groups. Changed bars/axis content can fragment damage and repeatedly
rasterize a canvas. Cached geometry instead contributes the cached group's bounds.
`iced_graphics::geometry::Cache::draw` creates that existing representation.

Alternatives are changing Iced's global damage grouping, retaining cached chart
geometry keyed by a new invalidation scheme, or regenerating geometry through
an existing canvas cache each draw. Choose the last: keep one canvas-state-owned
cache and clear it on every draw. This bounds chart damage without suppressing
any preparation, mutation, hover/primitive/divider change or draw callback.
No speculative revision keys or cross-frame model references are introduced.
The state owns encoded geometry through Iced's cache; returned geometry retains
its backing storage and drops independently. No closure is retained, no cycle,
new public method or engine thread is introduced.

A test uses actual tiny-skia damage diff/grouping after several viewport changes,
asserting one chart damage region and fresh raster output. A direct development
dependency on the already-locked renderer exposes its damage algorithm to this
behavioral work guard; no new package or runtime dependency is needed.

## Measured fix and user observation

The full-demo damage regression fails before the cache change with 15 damage
regions and passes afterward with at most three total UI regions (the chart is
one region; controls/status may contribute others). A single-chart guard also
checks one region and fresh pixels over repeated viewport changes.

The user repeated the physical software gestures and reported "Smooth now".
At the same 941×1030 surface, the after trace contains 1,049 submissions, all
resolved and zero discarded. Render/present wall p95 is 11.266 ms and CPU p95
11.179 ms, versus 117.836/117.512 ms before. These are manually exercised
sessions of different lengths, not identical scripted sample sequences. The
regression and renderer source analysis support the repeated-rasterization cause;
these recordings alone are not the fixed acceptance matrix.

A release calibration of the span collector averages 1,381 ns per completed span.
The probe overhead is material for microsecond updates, but far too small to
explain the hundreds of milliseconds in the slow rendering trace. Main-thread
spans exclude worker CPU. The diagnostic window owner is explicitly released on
CloseRequested before Iced waits for native window destruction; the corrected
after session exits successfully.

## Update attribution follow-up

A separate diagnostic build instruments validation, DataLayer update, model
application and the enclosing actual `Chart::update_data` call. Six alternating
1,200/100,000-row pairs use 20 warmups and 2,000 measured calls, CPU 5. Full traces
contain 2,020 records for each phase per execution; no phase was inferred by
subtracting unrelated fixtures. The phase hooks add significant overhead and
cannot establish acceptance. `diagnostic_only=true` logs are now explicitly
rejected by the update acceptance reader, with a RED/GREEN regression.

Across these diagnostic pairs, total p95 is 16.955–27.801 µs; model application
p95 is 5.800–11.562 µs and data-layer p95 4.319–7.484 µs. Variations affect all
phases in both histories. No phase consistently worsens with history size and
no specific additional update defect is established or fixed. These results do
not replace the prior uninstrumented failure or justify claiming it was merely
noise. Raw phase summaries/protocol are under
`target/live-update-acceptance/update-stage-study/`.

## Pixel checks and rejected experiment

The new cache-versus-direct comparison runs the actual public ChartProgram and
independent direct frame drawing, using a retained ChartState and repeated data,
primitive and label updates. Its matrix covers all six built-in families, four
sizes (including 300×100 and 300×180), four ratios and both backends. Results
are exact: 384 comparisons per backend, 768 total, all passing.

Expanding the earlier rectangle-cropping experiment to short heights failed pixel
equality. That experiment is retained in `program-pixels-software.log` and was
not adopted. Its original regular/narrow-width diagnostic remains available;
its legacy assertion was not relaxed. Cache-versus-direct comparisons are a
separate mode (`--cached-geometry-check`) and require exact pixels at every size.

## Frozen-candidate automated acceptance

Candidate source, policy and binary hashes are retained in
`target/live-update-acceptance/frozen-candidate.json`; copied executables keep the
measured build independent of later diagnostic experiments. No budgets, sample
counts, warmups or failed records were changed. Timing studies ran serially.

The six fixed uninstrumented update pairs are **INCONCLUSIVE**. Four of six
history comparisons pass; runs 2 and 6 flag. Three of six complete same-history
controls also flag. This prevents attributing the difference to history growth,
but does not establish a cause or turn the failures into passes. Raw results and
the unchanged-policy decision are in `update-acceptance/` and
`update-decision.json` under the artifact root. No additional engine update fix
is claimed.

The fixed native matrix stopped after its first set according to policy:
**31/32 cohorts pass**. All 16 GPU cohorts pass in that set. The remaining failure
is software, 1,200 rows, regular size, pan: work p95 **10.534813 ms** passes, while
presentation p95 **43.722392 ms** exceeds 33.333334 ms. Feedback resolves with zero
discards. Sets 2/3 were not run after this failed set. The previous GPU miss is
retained in the historical report; one passing set does not close that gate.
Raw feedback and results remain in `native-acceptance/manifest.json`.

Six failing-pan samples exceed the presentation budget. Some contain wall/CPU
work gaps, but others complete work in about 10 ms and still present after about
43.7 ms. This establishes a remaining presentation delay, without establishing
an engine defect or distinguishing compositor scheduling from profiling-host
pacing. A four-execution old/new pilot also showed variable presentation phases;
it is diagnostic and does not replace the failed matrix.

## Validation and remaining scope

The normal build passes 576 Rust tests across 52 targets, all-target checking,
strict Clippy, 30 Python tests, formatting and 20 TypeScript reference scripts.
The subsequently broadened marker/short-height regression also passes. Pixel
comparisons cover all six built-in series, four sizes, four scale ratios and four
updates on each backend, with a retained ChartState.

The mouse-only diagnostic session confirms the reported software plot/axis lag
is resolved at the recorded problem size. Full regular/half-size checks of zoom,
box editing/cancel, capture/release and short-height recovery on the final normal
build remain pending. Earlier GPU mouse passes remain historical evidence.
Touchpad, touchscreen and high-DPI physical checks are untested. Input-to-photon
latency has not been measured.

No conflation work starts while these gates remain open. Production changes are
limited to `src/model/price_scale.rs` and `src/api/widget.rs`; supporting example,
diagnostic, regression and acceptance-reader files provide reproduction and
verification. No public API, async engine machinery or TS identity/lifecycle
pattern is introduced. Remaining dependencies are acceptance diagnosis and
hardware coverage, rather than another required source component for these fixes.

## Rejected callback pacing hypothesis

A diagnostic-only prototype removed the profiling host's fixed timer in favor of
Wayland callback pacing. The first attempt stopped because monitor metadata was
not yet available at window startup; its log is retained. The corrected four-run
comparison checked actual presentation refresh (16,666,666 ns) and frame cadence
for software/1,200-row regular and GPU/100,000-row narrow cases. Both pacing modes
retained roughly 16.66–16.70 ms median frame intervals; presentation p95 stayed
roughly 33 ms. There was no demonstrated presentation improvement. The callback
software run also discarded one warmup presentation, retained in the raw data;
resolved-only phase summaries do not erase that failure.

The prototype is rejected and removed from repository code. Its exact source,
patch, binary, hashes, raw feedback and summary remain in
`target/live-update-acceptance/pacing-diagnostic-2/`. It provides no acceptance
pass and no evidence that double pacing caused the remaining miss. Production
and acceptance pacing remain unchanged. No repeated acceptance was run afterward.

## Files and reproducible checks

| Files | Purpose |
| --- | --- |
| `src/model/price_scale.rs` | Finite short-height affine projection and coefficient regression |
| `src/api/widget.rs` | State-owned, regenerated canvas cache for bounded damage |
| `Cargo.toml`, `Cargo.lock` | Already-locked tiny-skia renderer exposed to development tests |
| `examples/public_chart/tests.rs`, `tests/software_canvas_damage.rs` | Full-demo damage, fresh pixels, short-height/resize/live-update regressions |
| `examples/public_chart.rs`, `examples/public_chart/software_drag_profile.rs` | Exact cached/direct pixel fixture |
| `examples/support/runtime_profile_native.rs`, `runtime_profile_spans.rs` | Isolated runtime input/render/presentation recorder and regression |
| `scripts/profile_public_chart.py`, `scripts/analyze_public_chart.py`, `scripts/test_profile_public_chart.py` | Diagnostic build, strict trace analysis and tests |
| `scripts/frame_acceptance.py`, `scripts/test_frame_acceptance_cli.py` | Explicit diagnostic-marker rejection and regression |
| `README.md`, this report, `tasks/live-update-acceptance-{plan,todo}.md`, `tasks/incremental-append-todo.md`, `tasks/live-append-mouse-acceptance.md` | Roadmap, evidence and remaining gates |

Normal validation uses `cargo fmt --all --check`, `cargo test --all-targets`,
`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and
`python -m unittest discover -s scripts -p 'test_*.py'`. Focused RED/GREEN logs,
reference-script logs and exact pixel results remain under the artifact root.
The final Python recheck passes all 30 tests after the pacing prototype is removed.
The production diffs were reviewed against the source contract and actual renderer
ownership/damage behavior; no additional blocker was identified in those fixes.
The unresolved acceptance outcomes above remain blockers for the milestone.

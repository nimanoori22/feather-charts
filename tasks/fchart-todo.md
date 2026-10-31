# .fchart implementation checklist

Status: implemented; automated verification passed. Remaining graphical-session acceptance is recorded below. Design: [fchart-plan.md](fchart-plan.md).
User decision: true proportional linear numeric axes from v1; the executable is
`feather-charts`, the app package is `feather-charts-app`, and the library package
remains `feather-charts`. The previous evenly spaced proposal is superseded.
Each task includes its focused tests; paths below are proposed unless marked existing.
Keep implementation slices small; the files listed are responsibility boundaries,
not permission to refactor surrounding code. No commits, publication or installation
on the user's machine are included in the planning request.

## Task 1: Freeze the document contract

**Dependencies:** none. **Scope:** M, documentation/fixtures only.
**Files:** `docs/fchart-format.md`, `examples/fchart/minimal.fchart`,
`examples/fchart/benchmark.fchart`, `tasks/fchart-plan.md`.

**Acceptance:**
- [x] Freeze the example/schema fields, defaults, strict version behavior, finite
  numbers, increasing x-keys, units, empty-state policy and reader budgets in the plan.
- [x] Specify linear numeric distances shared across series. Include x=1,2,100
  fixtures and require gap ratio 1:98; specify histogram bar_width in x units.
- [x] Verify line/histogram construction, formatting and host event APIs against
  current public chart and exact Iced 0.14 sources; identify any API gap before code.

**Verification:** parse both fixtures with Python's JSON module and an assert-based
contract check; manually trace a point from fixture to proposed chart input. Resolve
any construction gap and record the required continuous-axis extension in Task 2A.

## Task 2: Deliver the headless reader/writer

**Dependencies:** 1. **Scope:** M.
**Files:** existing `Cargo.toml`, `Cargo.lock`; new
`crates/feather-chart-file/Cargo.toml`, `src/lib.rs`, `tests/document.rs` under that crate.

**Acceptance:**
- [x] Add the format package as a workspace member, retain root default package,
  patches/profiles, and add direct Serde/serde_json dependencies only to this package.
- [x] Implement public document types and bounded read/write helpers with the same
  validation on both paths; report version, location/field and I/O errors usefully.
- [x] Cover the frozen fixtures, round trips, empty series, bad versions/types,
  unknown fields, null/nonfinite values, ordering, budgets and failing reader/writer.

**Verification:** `cargo test -p feather-chart-file`; inspect `cargo tree -p
feather-chart-file` to confirm no Feather Charts/Iced dependency; root
`cargo check --all-targets` and `cargo package -p feather-charts --list` stay valid.

## Checkpoint A: Portable file

- [x] A benchmark producer can generate and validate a file without any GUI dependency.
- [x] Headless codec does not change library defaults; v1 fixture tests pass.

## Task 2A: Design the opt-in continuous numeric axis

**Dependencies:** 1. **Scope:** M, investigation/design only.
**Files:** `docs/fchart-numeric-axis.md`, `tasks/fchart-plan.md`, this checklist.

**Acceptance:**
- [x] Trace projection/inverse, data lookup, ticks, visible ranges, fit, series
  views, interactions, primitives and export through codebase-memory and exact
  source/coverage checks. Consult the local Iced source for changed host behavior.
- [x] Compare shared opt-in projection versus dedicated numeric owner/scale;
  record ownership, API signatures, mode-specific semantics and consumer mapping
  before engine edits. Existing domains keep their behavior; no synthetic rows.
- [x] Define range padding, empty/single-point behavior, overflow/precision policy,
  continuous pointer coordinates versus sample lookup, nearest ties, numeric tick
  placement and histogram widths. Break any slice exceeding five files further.

**Verification:** walk x=1,2,100 and two disjoint series through the whole proposed
pipeline. Review the affected-file list and targeted regression commands. The
chosen public API must support both file-backed viewing and ordinary library use.

## Task 2B: Implement numeric projection and ranges

**Dependencies:** 2A. **Scope:** M per slice; split by owner after the traced design.
**Likely files:** `src/model/time_scale.rs`, new numeric projection module,
`src/api/coordinates.rs`, selected public configuration/exports and focused tests.
Do not commit to exact module names before Task 2A establishes the boundary.

**Acceptance:**
- [x] Add opt-in linear numeric projection/inverse and visible numeric ranges;
  retain row indices only for data identity/search. No span-sized gap filling.
- [x] Implement finite empty/single-point fit, negative/fractional values, stable
  large-offset/tiny-span arithmetic and explicit unrepresentable-range errors.
- [x] Test gap ratio 1:98, cross-series coordinate equality, inverse round trips,
  bounds and unchanged existing logical-index coordinate semantics.

**Verification:** focused projection/coordinate tests and existing domain tests;
prepare a line scene and assert actual point coordinates, not only axis labels.

## Task 2C: Connect ticks and numeric interaction

**Dependencies:** 2B. **Scope:** M per slice; ticks and interaction are separate
commits/slices if they reach different owners or more than five files.
**Likely scopes:** numeric axis ticks/labels, visible-series searches, existing
interaction and crosshair owners, axis-focused public tests.

**Acceptance:**
- [x] Generate readable ticks at their numeric coordinates independently of sample
  locations; use numeric range searches for clipping and y-autoscale.
- [x] Pan, pointer-anchored zoom, Fit, resize and supported axis gestures operate
  on the numeric extent. Crosshair labels show continuous values; sample selection
  returns original data with defined nearest-point behavior.
- [x] Connected tests preserve the 1:98 ratio after each operation and verify
  tick positions, cursor anchoring, inverse conversion and disjoint series lookup.

**Verification:** focused interaction/tick tests plus an irregular-x native scene
on both backends. Existing calendar, strike, maturity and custom-domain interaction
regressions remain green. No mixed numeric rendering/index-based hit testing.

## Task 2D: Complete histogram geometry and regression gate

**Dependencies:** 2C. **Scope:** M per slice.
**Likely scopes:** histogram preparation/hit testing, numeric fit/clipping,
focused numeric-series tests and `docs/fchart-numeric-axis.md`.

**Acceptance:**
- [x] Render columns centered at x with positive finite bar_width measured in x
  units; project both edges, include their extent in Fit, and use zero baseline.
- [x] Numeric line/histogram hit tests, clipping, screenshot preparation and empty
  states agree with displayed geometry. Unsupported old index-based aggregation
  is disabled in numeric mode unless equivalence is demonstrated.
- [x] Existing domains, built-in series and available source-reference checks pass;
  record changed engine paths, acceptance evidence and remaining limitations.

**Verification:** geometry assertions with irregular/negative x and explicit widths;
software/GPU native captures and pointer checks; run the affected library regression
suite. Viewer mapping is blocked until proportional spacing works end to end.

## Checkpoint: Continuous numeric engine

- [x] x=1,2,100 has gap ratio 1:98 across draw, ticks, crosshair, pan, zoom and resize.
- [x] Histogram widths use numeric units and shared x-values align across series.
- [x] Existing domains retain their semantics; numeric support is opt-in.

## Task 3: Map documents through the continuous numeric chart API

**Dependencies:** 2 and 2D. **Scope:** M.
**Files:** `apps/feather-charts/Cargo.toml`, `src/main.rs`, `src/chart.rs`,
`src/chart_tests.rs` under that app, existing root `Cargo.toml`/`Cargo.lock` as needed.
Split manifest setup from adapter work if the diff grows beyond a focused session.

**Acceptance:**
- [x] Add viewer package with minimal executable scaffolding and app-owned adapter
  using the continuous numeric axis from Tasks 2A-2D, line/histogram data, fit and
  preparation APIs. Package is feather-charts-app; [[bin]] name is feather-charts.
- [x] Apply documented precision/colors; return a candidate chart plus legend/labels
  without mutating the current chart or serializing engine internals.
- [x] Tests verify exact keys/values, multiple/disjoint series, empty input,
  series kinds and construction errors. Keep formatting policy in the viewer.

**Verification:** `cargo test -p feather-charts-app`; prepare one fixture headlessly and
assert finite output geometry/expected data readbacks with existing public APIs.
Confirm missing behavior does not lead to a parallel rendering implementation.

## Task 4: Open a file in a usable native viewer

**Dependencies:** 3. **Scope:** M.
**Files:** app `src/main.rs`, `src/app.rs`, `src/load.rs`, `src/app_tests.rs`.

**Acceptance:**
- [x] `feather-charts results.fchart` opens a chart; no path opens an empty welcome
  state; help/invalid arguments and missing/malformed input have useful diagnostics.
- [x] Forward chart messages, resize, redraw and scale-factor changes using existing
  host patterns. Pan, zoom, crosshair and Fit preserve proportional numeric distances.
- [x] Show title, legend and axis labels/units with ticks at numeric positions.
  Use Feather Charts as the application title; remove financial-demo wording.

**Verification:** argument/load tests plus native fixture launch on TinySkia and
WGPU; inspect exact sample values, resize and Fit. Loading errors never panic.

## Task 5: Reopen and recover from errors

**Dependencies:** 4. **Scope:** M.
**Files:** app `src/app.rs`, `src/load.rs`, `src/app_tests.rs`,
`docs/fchart-viewer.md`.

**Acceptance:**
- [x] Add keyboard-accessible path/Open, Reload and OS file-drop handling. Read and
  parse off the UI thread; retain the old chart while validating its replacement.
- [x] Failed load shows an actionable error and preserves the last valid chart.
  Empty/all-empty documents show the defined empty state.
- [x] Handle multiple load requests deterministically: newest request wins; stale
  completion cannot replace the current chart. No shell commands execute paths.

**Verification:** tests for successful/failed replacement and out-of-order load
completion; native check with spaces in paths, invalid JSON and file drop. Confirm
visible focus, text contrast and usable controls at narrow window sizes.

## Checkpoint B: Interactive viewing

- [ ] Producer fixture -> CLI -> native chart -> reload works with both backends.
- [x] Proportional spacing, numeric ticks and units agree; failures preserve useful state.
- [x] Run integration checkpoint commands from the plan; record limitations.

## Task 6: Produce a chart from benchmark results

**Dependencies:** 2 and 5. **Scope:** M.
**Files:** `examples/fchart/write_benchmark.py`,
`crates/feather-chart-file/examples/write_benchmark.rs`,
`scripts/export_profile_fchart.py`, `scripts/test_export_profile_fchart.py`,
`docs/fchart-viewer.md`.

**Acceptance:**
- [x] Provide tiny Rust and Python examples that write valid documents without a
  GUI. Label synthetic demonstration data clearly; use create-new output paths.
- [x] Convert existing `frames.csv` to a chart of selected measured duration
  columns, with ns -> ms conversion, original frame keys and explicit warmup policy.
  Preserve phase identity, source field names and benchmark/run context in metadata.
- [x] Preserve input CSV/metadata and reject missing/malformed source fields. Do
  not reinterpret work time as GPU/presentation latency; do not put serialization
  in profiler timing sections. No new profiler dependency is required for this slice.

**Verification:** standard-library unittest covers unit conversion, warmup exclusion,
ordering and malformed rows. Feed both producer outputs and converter output to the
Rust reader; open a real retained profile output in the viewer. Confirm the original
profiler source/raw outputs remain unchanged. Document how future tests call the
writer directly to emit .fchart as their native result artifact.

## Task 7: Install and open files from Linux

**Dependencies:** 5. **Scope:** M.
**Files:** `packaging/linux/feather-charts.desktop`,
`packaging/linux/feather-chart.xml`, `packaging/linux/README.md`,
`docs/fchart-viewer.md`.

**Acceptance:**
- [x] Document `cargo install --path apps/feather-charts --locked` and the binary's
  requirements; package a desktop entry and MIME definition scoped to `*.fchart`.
- [x] Desktop Exec forwards a local file as an argument; handle paths with spaces.
  Explain opt-in default association, cache refresh and uninstall steps.
- [x] Validate in an isolated XDG data prefix first. Installation instructions
  preserve existing associations and install only user-owned assets.

**Verification:** desktop/MIME validators where installed; clean-prefix install and
uninstall; launch via desktop entry, then an actual file-manager open. Record what
requires user-session acceptance. Do not change this machine's defaults without
implementation-time authorization for installation.

## Task 8: Verify the complete feature and document limits

**Dependencies:** 6 and 7. **Scope:** M, acceptance/docs.
**Files:** `README.md`, `docs/fchart-viewer.md`, `docs/fchart-acceptance.md`,
`tasks/fchart-plan.md`, `tasks/fchart-todo.md`.

**Acceptance:**
- [x] Document build/install/write/open examples, proportional linear numeric axes,
  resource budgets, error messages and supported platforms without implying
  deferred types/panes/logarithmic x-axes are supported.
- [ ] Verify producer -> file -> installed viewer -> file-manager flow on Linux
  with both backends; record keyboard/mouse, resize/scale and error recovery checks.
- [x] Run final plan checks and a measured 100k-point loading/navigation smoke test;
  record file size, memory, load time, backend and machine instead of promising an
  unmeasured performance target. Existing library regression checks remain green.

**Verification:** final commands in the plan plus a reviewed acceptance record.
Physical touch/high-DPI, Windows/macOS, and unavailable desktop checks remain
explicitly unverified. Fix format/adapter defects in their owning package; engine
changes beyond Tasks 2A-2D require additional source/consumer investigation per
repository AGENTS.md.

## Checkpoint C: Complete

- [x] Version 1 contract, committed fixtures and behavioral tests agree; real numeric
  distances pass geometry and connected interaction tests.
- [x] Headless producers do not require Iced; library users do not require viewer.
- [x] Linux software can open .fchart files after the documented installation.
- [x] No unrelated plan, public library API, test threshold or benchmark evidence
  was weakened or overwritten. Review the final diff before any merge/release.

## Execution evidence

See `docs/fchart-acceptance.md` for commands, timings, renderer captures and
remaining manual acceptance. Checked items describe implemented/tested behavior;
unavailable physical or graphical-session checks are deliberately left open.
Desktop/MIME assets were validated only in disposable XDG prefixes.

Final automated outcome: 908 workspace/all-target tests passed (0 failures or
ignored tests), plus final focused codec (7), app (5), numeric reset/range (2)
and Python export (2) checks. Doc tests, strict Clippy, workspace check, release
build, library package listing and whitespace/format checks passed. Isolated
installation/reinstallation/uninstallation passed. See the acceptance record
for renderer timings, native startup/launcher evidence and manual checks.

## User-requested follow-up: chart-only view / data readout

The user confirmed native acceptance, then requested removal of the surrounding
viewer UI and an in-chart crosshair value/OHLC readout. This supersedes the earlier
permanent controls/title/legend/axis-label layout; file labels remain preserved.

- [x] Remove permanent viewer chrome; retain transient keyboard opener and errors.
- [x] Preserve Ctrl+O/Enter, Ctrl+R, Ctrl+0, Escape and file drop.
- [x] Add reusable opt-in top-left readout and enable it in built-in/OHLC demos.
- [x] Format selected OHLC/value rows; latest fallback, gap and hidden-series policy.
- [x] Test real overlay pixels, full viewer pixels and pointer pass-through.
- [x] Finish follow-up formatting, checks, Clippy, focused regressions and release build.

Follow-up results: readout integration tests 3/3; app tests 7/7; WGPU whole-view
pixel test 1/1; candlestick demo tests 16/16. Workspace all-target check and strict
Clippy passed, formatting/whitespace passed, release viewer rebuilt. Both software
and WGPU tests verify that the viewer pixels equal only the chart/readout widget.
Software captures were visually inspected. Native manual interaction was not
repeated for this presentation-only follow-up; no associations were changed.

## Release 0.2.0

- [x] Bump library to 0.2.0 and document low-level API migration.
- [x] Prepare headless/viewer 0.1.0 manifests with registry dependency versions.
- [x] Bundle package-local fixtures, licenses and README files.
- [x] Pass workspace tests (916), doc tests, checks, Clippy and release build.
- [x] Pass workspace publication dry run and extracted headless-package tests.
- [x] Verify GitHub push authentication; retain explicit dependency-audit limits.

Publication and pushed version tags are reported in the release execution result.

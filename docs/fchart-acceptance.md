# .fchart acceptance record — 2026-10-31

Implementation followed the revised documents copied from
`/home/nima/Documents/Codex/2026-10-31/i/outputs/` into the two feature task files
before feature code was edited. Initial working tree: only those two untracked
older task documents. Other plans and existing benchmark evidence were preserved.

## Automated evidence

- Headless codec tests cover committed fixtures, round trips, empty data,
  unknown fields/enums/versions, malformed/null/nonfinite points, ordering,
  byte/series/point/text/metadata budgets, histogram edges, unrepresentable
  extents, validation before output, and failing I/O. Explicit `bar_width: null`
  is rejected, including for lines, whose width field must be absent.
- Engine tests assert 1:98 using prepared line points and canvas paths, exact
  cross-series x alignment, linear Y conversion, numeric ticks between samples,
  continuous crosshair coordinates/labels, column rectangles and column gap
  picking. Scale tests cover pointer anchoring, pan, zoom, resize, ties,
  negative/fractional values, large offsets, tiny spans, empty/single points and
  invalid ranges. Existing index domains retain their defaults.
- App tests verify exact original data readbacks, mixed series, geometry,
  labels/precision in the retained document, path handling, missing-file errors,
  transactional replacement, stale completion and error preservation.
- Python standard-library tests verify unit conversion, original frame keys,
  warmup policy, phases, ordering and malformed rows. Python, Rust and real
  profiler output all passed the Rust reader. All three producers were tested
  against an existing output and refused to overwrite it.
- `cargo tree -p feather-chart-file`: Serde/serde_json only; no chart/Iced graph.
- Root package now uses `autobins = false`: its old Hello World placeholder
  otherwise collides with the viewer's binary in workspace builds. Cargo metadata
  verifies exactly one binary target: package `feather-charts-app`, executable
  `feather-charts`. The placeholder source was left untouched.

Verification commands (logs under `/tmp/fchart-*.log` during this session):

```sh
cargo fmt --all --check
cargo test -p feather-chart-file
cargo test -p feather-charts-app
cargo check --workspace --all-targets -j 2
cargo test --workspace --all-targets -j 2
cargo test --workspace --doc -j 2
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo build -p feather-charts-app --release -j 2
cargo package -p feather-charts --list --allow-dirty
python -m unittest discover -s scripts -p test_export_profile_fchart.py
python packaging/linux/check.py
git diff --check
```

`--offline` was used after the initial dependency resolution; `-j 2` limits build
contention. `--allow-dirty` is needed to inspect library packaging before committing.
No tests or lint thresholds were weakened. One concurrent intermediate run failed
an existing wall-clock double-click pairing assertion (click/click/double instead
of click/double/click). The unchanged `ClickSequence` has a 500 ms threshold;
the test uses real time. This is consistent with contention during overlapping
compilation/rendering. Its isolated rerun passed; the earlier complete workspace
run also passed all 906 tests. The timing-sensitive test was neither changed nor
skipped. Final command outcomes are recorded below after execution.

## Renderer and benchmark evidence

Artifacts are local, ignored outputs under `target/fchart-acceptance/`:
`software.png`, `wgpu.png`, `profile-plain-software.png`,
`100k-plain-software.png`, `100k-plain-wgpu.png` and corresponding PPM/log files.
Both backends completed actual rasterization with colored-data assertions; mixed
line/column software and WGPU captures and the retained profile capture were
visually inspected. Native chart rendering uses the same prepared geometry and
screenshot path. No viewer-local canvas or synthetic gap rows were introduced.

Real input: `target/presentation-follow-up/wire-study/3-tiny-skia/frames/frames.csv`.
Conversion produced 8 phase/column series and 800 measured points. Frame keys were
preserved, warmups excluded, ns converted to ms, metadata retained. Recorded
SHA-256 hashes confirm source CSV and metadata were unchanged. Profiler measured
sections and native profiler source were not modified.

100k input: 1,578,197 bytes; 100,000 original keys. Debug build, Linux,
Intel i5-9400F (6 logical cores), 15.5 GiB RAM, AMD Radeon RX 5500 family GPU.
Other regression builds/tests were running, so these are smoke observations,
not controlled performance claims. Timings below are cumulative from start except
navigation; memory is process VmHWM after construction, before rendering.

| Backend | Parse | Constructed | First preparation | Ten zoom preparations | Construction VmHWM |
| --- | ---: | ---: | ---: | ---: | ---: |
| TinySkia | 84 ms | 528 ms | 1.382 s | 737 ms | 316,768 KiB |
| WGPU | 61 ms | 474 ms | 820 ms | 1.196 s | 315,168 KiB |

The first WGPU 100k capture with per-point markers exceeded a 60-second smoke
limit. The viewer now uses the existing plain-line default; both captures above
completed. No index-based conflation is used in numeric mode. Resource caps bound
input, not peak RAM; chart construction remains synchronous after background parsing.

Reproduce captures with `cargo run -p feather-charts-app --example verify --
INPUT.fchart OUTPUT.ppm [--wgpu]`; output is create-new. The optional `--100k`
probe substitutes a synthetic dataset; the reported run instead read a real
100k-point JSON file, so parsing was included.

## Linux integration

- `desktop-file-validate` passed. XML declares the JSON subtype and scopes the
  glob to `*.fchart`. GIO identified `application/x-feather-chart` from an isolated
  MIME cache. Generic `xdg-mime` may use libmagic and report JSON on this desktop;
  the shared MIME/GIO path was explicitly tested.
- `cargo install --path apps/feather-charts --locked --root
  target/fchart-acceptance/install` succeeded. The isolated installed executable
  passed `--help` (0) and invalid-arguments (2) checks. Installation reported an
  existing locked, yanked Windows `clipboard-win` dependency; the lockfile was
  retained rather than updating unrelated dependencies.
- Native installed viewer startup with the real benchmark completed an eight-second
  smoke window on TinySkia and WGPU, ending only at the intentional timeout (124).
- `gio launch` used the unchanged desktop entry in a separate XDG prefix with a
  timeout wrapper around the isolated installed viewer. An argv receipt confirms
  the path `profile with spaces.fchart` reached it as exactly one argument.
- The repeatable `packaging/linux/check.py` creates and deletes its temporary
  asset prefix, refreshes caches and verifies no mimeapps default file was created.
  No user desktop assets/defaults were installed or changed; nothing was published
  or pushed.

## Remaining manual acceptance

Actual file-manager clicking/Open With, visual native UI focus/contrast at narrow
sizes, mouse/keyboard gestures, native OS drop/reload/error interactions, window
resize/scale changes, screen-reader behavior and physical touch/high-DPI hardware
were not manually exercised. Their state/geometry paths have automated coverage;
that does not establish physical/user-session acceptance. Windows/macOS installers
remain outside v1. Numeric inertia is intentionally disabled until its velocity
state is expressed in numeric units; direct dragging and zoom remain supported.

## Review scope

The engine extension touches horizontal capability/projection/ranges, public
coordinate/data boundaries, crosshair placement, histogram preparation/rendering
and picking. Legacy paths remain selected when numeric mode is absent. The
renderer data adds optional `numeric_width`; prepared hit geometry adds `Columns`
so authored widths cannot inherit index adjacency. Repository exhaustive matches
were updated in profiling examples. Consumers constructing those low-level public
structs/matching that enum must account for the new field/variant.

New production packages: `crates/feather-chart-file` (codec/validator) and
`apps/feather-charts` (adapter, loader and native host). New producer/export scripts
run after measurement. No profiler measurement code, locks, async chart-engine
state, test suppression, renderer fork or additional rendering implementation was
introduced. Source/graph coverage checks were exact-path and best-effort; direct
reads supplemented method edges and changed-source metadata.

## Final verification results

Final cached workspace/all-target run: **908 passed, 0 failed, 0 ignored**.
Codec suite: 7 passed. Final app suite: 5 passed, including empty/invalid
construction and keyboard shortcuts. Focused numeric scale tests: 2 passed,
including Reset -> Fit. Doc tests: 2 passed. Strict workspace Clippy,
workspace all-target check, formatting, release build, library package listing
and diff whitespace checks passed. Python export tests: 2 passed.

The isolated installed CLI passed after force-reinstall; `cargo uninstall`
removed only the test-prefix executable, and its absence was verified. The
last full run includes strict-null-width validation. Subsequent app-only checks
cover explicit keyboard shortcuts because this Iced version's buttons do not
provide focus handling. Ctrl+O focuses the path, Enter opens it, Ctrl+R reloads
and Ctrl+0 fits; visible help lists the bindings.

The only observed intermediate regression failure was the real-time double-click
pairing test described above; its isolated rerun and both complete workspace
runs passed without modifying or skipping it.

## Follow-up: chart-only viewer and crosshair readout

User acceptance confirmed the original viewer worked, then superseded the permanent
controls/labels requirement: loaded documents now show only the chart with a
compact top-left data readout. Open/reload/Fit remain reachable via Ctrl+O, Enter,
Ctrl+R, Ctrl+0 and file drop; Escape dismisses the temporary opener/error overlay.
The former standalone legend/title/axis-label/status widgets were removed.

The local upstream `website/tutorials/how_to/legends.mdx` and `legend-3line.js`
implement legends as host overlays driven by crosshair data, with latest-value
fallback. Compared a canvas/engine addition with an optional Iced widget overlay;
chose `Chart::element_with_readout()` to keep existing `element()` and screenshot
semantics intact. `crosshair_readout()` reads canonical rows with existing series
formatters. The chart owns data; the view borrows it and owns only formatted
strings. No callbacks, shared ownership, subscriptions, cleanup API or cycles are
introduced. Missing selected samples show a dash rather than stale/latest data.

Focused tests assert OHLC selection/formatting/latest fallback, sparse-series gaps,
empty/removal behavior, readout raster pixels, unchanged plot pixels/layout and
pointer pass-through when the cursor lies over the readout. A whole-view raster
test verifies that the normal viewer is pixel-identical to the chart widget with
its readout. A 16-test candlestick demo regression run passed. Final validation
results for this follow-up are recorded with the task checklist.

Follow-up outcome: 3 readout integration tests, 7 app tests, the explicit WGPU
whole-view pixel test and all 16 candlestick demo tests passed. Workspace
all-target check, strict Clippy, fmt, diff check and release build passed. The
software viewer raster was visually inspected. The readout's pointer test sends
real widget resize and cursor events over the overlay and confirms they reach
the chart. Full old workspace tests were not repeated for this opt-in presentation
change; the affected targets and shared demo were checked. Native manual
interaction and physical accessibility/hardware checks remain for user acceptance.

## 0.2.0 release preparation

The library is version 0.2.0 because low-level public renderer data and hit geometry
changed. The new headless `feather-chart-file` and native `feather-charts-app`
packages start at 0.1.0; the `.fchart` wire version stays 1. The changelog records
migration instructions. The app declares registry version requirements alongside
its local path dependencies. Each new package bundles its own fixtures and license
so extracted packages do not rely on files outside their crate directories.

Release validation passed: 916 workspace/all-target tests (run with two build jobs
and one test thread), 2 doc tests, strict workspace Clippy, all-target check,
release viewer build, explicit WGPU viewer pixel test, 3 Python export/producer
tests and isolated Linux asset checks. The extracted headless package passed
its 7 behavioral tests and its example compiled. A complete workspace
`cargo publish --workspace --dry-run` packaged and verified all three crates.
GitHub push authentication was checked with a dry run.

The release review added explicit flushing inside the Python producer's cleanup
block, so a buffered write failure removes only its newly created partial output.
A regression test injects that flush failure and verifies create-new/cleanup.

`cargo-audit` is not installed, so a RustSec dependency audit was not performed.
Cargo reports the pre-existing locked Windows-only `clipboard-win` 5.4.1 as
yanked; dependencies were not upgraded as part of this feature release.
No desktop defaults or user associations were installed by release validation.

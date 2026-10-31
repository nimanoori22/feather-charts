# Development plan: .fchart documents and Feather Charts application

Date: 2026-10-31. Status: implementation authorized by the current user request; revised specification imported before code changes.
Repository: `/home/nima/Desktop/dev/rust_dev/lw_chart_rs_port/feather-charts`.
Inspected baseline: `5499e40`; working tree was clean before planning.
Execution checklist: [fchart-todo.md](fchart-todo.md).
Repository destinations: `tasks/fchart-plan.md` and `tasks/fchart-todo.md`.

## Outcome

A benchmark or script writes `results.fchart`. An installed `feather-charts`
opens the document as an interactive native Feather Charts chart. Users can
pan, zoom, inspect values, and reset the view without rerunning the benchmark.
Linux users can open the file from their file manager after installing desktop
integration. The format is documented so any language can produce files.

The user requires true proportional numeric distances from version 1. Both axes
use linear numeric scales: x = 1, 2 and 100 must produce horizontal gaps in the
ratio 1:98. Pan, zoom, Fit and resizing preserve this relationship. The earlier
evenly spaced proposal is superseded. The executable and desktop application are
named `feather-charts`; the internal application package is `feather-charts-app`
to avoid a Cargo package-name collision with the existing library.

## Version 1 scope

- One chart/pane per document, with multiple named series sharing one y-unit.
- Continuous linear numeric x/y axes; line and histogram series. Reuse existing
  renderers after the engine can project numeric x-values proportionally.
  Histogram means Feather Charts' value-per-x columns, not automatic statistical
  binning. Producers calculate bins or summary statistics themselves.
- Title, x/y labels (with optional unit text), numeric precision, series names, and optional
  per-series color. A small string-to-string metadata map records run context.
- Embedded data only. The viewer does not load URLs, external data references,
  scripts, callbacks, plugins, or arbitrary renderer state from documents.
- Read-only viewer: CLI path, drag-and-drop, and an accessible path field with
  Open/Reload/Fit controls. Native file picker can follow if useful; it is not
  needed to open a document in this release.
- A headless Rust reader/writer and a standard-library Python producer example.
- Build/install instructions and Linux desktop/MIME integration assets.

Deferred: logarithmic x-axes, dates, categorical keys, additional series
types, multiple panes/units, uncertainty bands, annotations, chart editing and
Save As, live streams/watch mode, CSV import, benchmark-run databases, comparison
dashboards, compression/binary containers, native file pickers, web/cloud viewing,
Windows/macOS installers, and publishing packages/releases. Existing chart engine
modes retain their current behavior; the new continuous axis is opt-in. These
format limits apply to the new document format/viewer.

## Repository foundation and required extensions

| Existing path | Reuse or implication |
| --- | --- |
| `Cargo.toml` | Currently one package, with Iced 0.14 and root renderer patches/profile settings. Add workspace members without moving the published library. |
| `src/api/chart.rs` | `Chart` owns synchronous data/model state; `add_series`, `set_data`, `fit_content`, `prepare` provide the construction path. `set_data` already validates engine input. |
| `src/api/options_chart.rs` | `OptionsChart` aliases `Chart<HorzScaleBehaviorPrice>`; supports finite numeric horizontal items, but its index-based projection is insufficient for the required continuous axis. |
| `src/model/horz_scale_behavior_price.rs` | `HorizontalPrice::new` rejects nonfinite values; numeric label precision exists. Keep the wire format independent of these financial Rust names. |
| `src/api/widget.rs` | Existing `element` and chart messages supply interaction and drawing. Reuse the widget; numeric geometry, ticks and interaction need an engine extension. |
| `examples/public_chart.rs`, `examples/options_chart.rs` | Reuse application update/prepare, resize, scale-factor and subscription patterns; do not copy demo controls/capture machinery wholesale. |
| `docs/usage.md`, price-based horizontal charts | Explicitly documents logical-index spacing rather than continuous numeric distance. This is an engine gap that must be closed before the new viewer is accepted. |
| `examples/frame_latency_profile/native.rs` | `Running::save` writes `frames.csv` and metadata after collection. Exporting a chart must stay outside measured sections and preserve those outputs. |
| `tasks/*-plan.md`, `tasks/*-todo.md` | Feature-specific planning pairs are established. Existing `tasks/plan.md`/`todo.md` retain unfinished hardware checks and will not be replaced. |

Investigation used codebase-memory search, a `set_data` call trace/snippet,
exact-path coverage checks, and direct source reads. Coverage reported matching
metadata/no recorded issues for the evidence paths; this is best-effort evidence.
The call trace omitted method edges visible in source, so it is not a complete
caller inventory. No engine-wide audit or runtime verification was performed.

## Architecture and alternatives

Choose a small Cargo workspace in the existing repository:

```text
Cargo.toml                         existing feather-charts package + workspace
src/                               chart library + opt-in continuous numeric axis
crates/feather-chart-file/          format model, JSON codec, validation; no Iced
apps/feather-charts/                installable binary + adapter + Iced host
docs/fchart-format.md               normative v1 contract
docs/fchart-viewer.md               producer, viewing and installation guide
examples/fchart/                    valid JSON fixtures + tiny producer example
scripts/export_profile_fchart.py    frames.csv -> .fchart outside measurement
packaging/linux/                    desktop/MIME assets and installation guide
```

Dependency direction:

```text
Rust benchmark -> feather-chart-file -> serde / serde_json
Python benchmark -> json (standard library)
feather-charts-app -> feather-chart-file
feather-charts-app -> feather-charts (library) -> Iced
```

The chart library does not depend on the file crate or viewer. The format crate
contains no engine handles, widget state, callbacks or rendering types. The
viewer owns a validated document and a chart constructed from it; a new candidate
document/chart replaces the old pair only after validation and construction pass.
No new shared ownership, locks or asynchronous chart engine is needed. File reads
and parsing should run off the UI thread using supported Iced task mechanisms;
confirm exact Iced 0.14 APIs before implementation and avoid adding Tokio solely
for file loading. If a runtime feature is necessary, scope it to the viewer.

Alternative A: put serialization and a binary directly into the existing library
package. Smaller initial manifest change, but headless benchmark producers would
inherit the Iced dependency graph. Reject for this producer-focused use case.

Alternative B (chosen): separate format and application packages in the same
workspace. Keeps producers lightweight and related changes reviewable together.
Separate GitHub repositories would add coordination work without a demonstrated
need. Revisit only for independent maintainers/releases.

The application lives at `apps/feather-charts`, has Cargo package name
`feather-charts-app`, and declares `[[bin]] name = "feather-charts"`. Its CLI is
`feather-charts results.fchart`, desktop entry is `feather-charts.desktop`, and
application title is Feather Charts. The existing library package remains
`feather-charts` (Rust import `feather_charts`); package selection uses
`-p feather-charts-app` for the app and `-p feather-charts` for the library.

Keep the existing root package name/API, `[patch.crates-io]`, profiles and examples.
Set workspace `default-members = ["."]`; use explicit workspace-wide checks for
the new members. Cargo recognizes patches/profiles at the workspace root:
[Cargo workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html).
Add direct Serde/serde_json dependencies only to the file crate; derive the wire
types rather than hand-writing a JSON parser:
[Serde derive documentation](https://serde.rs/derive.html).

## Continuous numeric axis: required engine work

Changing labels or adding a custom horizontal behavior alone does not change
geometry. The inspected `TimeScale::index_to_coordinate` projects integer indices
with a fixed bar spacing; public horizontal conversions resolve through those
indices. A targeted caller trace reaches series views, annotations and frame
preparation. This is a shared engine boundary, not just a file-reader change.
Exact-path coverage for `src/model/time_scale.rs`, `src/api/coordinates.rs` and
`src/model/horz_scale_behavior_price.rs` reported matching metadata/no recorded
issues. A complete affected-consumer investigation remains Task 2A.

Use affine projection in numeric space: `pixel(x) = left +
(x - visible_min) / (visible_max - visible_min) * plot_width`, with checked,
numerically stable arithmetic and a matching inverse. Storage indices may still
identify rows, but cannot determine horizontal distance. Empty/single-x datasets
need a defined finite fallback extent; reject unrepresentable ranges explicitly.
Keep original x-values, preserve real gaps and do not add synthetic rows to imitate
a continuous axis. Sparse datasets must cost O(points), not O(numeric span).

Task 2A compares (1) an explicit continuous projection mode integrated into the
shared horizontal scale, and (2) a dedicated numeric owner/scale sharing existing
series preparation and widget code. Prefer (1) if it preserves current index-based
contracts cleanly; choose (2) if retrofitting would make those contracts ambiguous.
The choice and exact public Rust API are an early implementation design gate,
not a promise that a behavior trait override is sufficient. No viewer-local
canvas or independently drawn axis. Existing calendar, strike, maturity and
custom-domain defaults keep their established behavior.

The connected implementation must cover projection/inverse, visible numeric
ranges and data searches, ticks at numeric positions, fit/autoscale, pointer-anchored
zoom, pan/clamping, crosshair and hit testing, series geometry/clipping and
screenshot preparation. Tick marks need not coincide with stored samples.
Crosshair labels show the continuous coordinate; any selected sample is identified
separately and nearest-point selection uses numeric/pixel distance with defined
ties. Vertical scale remains the normal linear mode with the authored y-unit.

For histogram series, require a positive finite `bar_width` in x-axis units,
center each column on its x-value, and project its two numeric edges. Include
zero in the vertical fit for the zero baseline; reject nonfinite/overflowing edges.
This is a value-per-x column chart, not inferred statistical binning. Do not retain
an index-width calculation that can bridge or overlap gaps unexpectedly. Cover
both renderer backends and leave existing histogram defaults unchanged.

Numeric-axis regression tests must show that x=1,2,100 produces gap ratio 1:98
before/after zoom, pan and resize, and that equal x-values across disjoint series
share a pixel coordinate. Include negative/fractional x, large offsets, tiny spans,
empty/single-point inputs, inversion, tick placement, pointer anchoring and
boundaries. Existing domains' tests must continue to pass.

## Proposed file contract

UTF-8 JSON with `.fchart` suffix. Example to freeze and test in Task 1:

```json
{
  "format": "feather-chart",
  "version": 1,
  "title": "Preparation benchmark",
  "x_axis": {
    "label": "Sample",
    "spacing": "linear",
    "precision": 0
  },
  "y_axis": {
    "label": "Duration",
    "unit": "ms",
    "precision": 3
  },
  "series": [
    {
      "name": "Prepare",
      "kind": "line",
      "color": "#38BDF8",
      "points": [[1, 0.421], [2, 0.438], [100, 0.409]]
    }
  ],
  "metadata": {"backend": "wgpu", "warmups": "excluded"}
}
```

Proposed public file-crate surface, finalized in Task 1:

```rust
pub struct ChartDocument { /* explicit v1 wire fields */ }
pub enum SeriesKind { Line, Histogram }
impl ChartDocument {
    pub fn validate(&self) -> Result<(), DocumentError>;
}
pub fn read_document(input: impl std::io::Read)
    -> Result<ChartDocument, DocumentError>;
pub fn write_document(output: impl std::io::Write, document: &ChartDocument)
    -> Result<(), DocumentError>;
```

Use ordinary public value types and a validator rather than parallel builder and
validated-type hierarchies. Both read/write entry points validate; derived Serde
serialization alone does not establish semantic validity. Path-opening and
overwrite policy belong to callers, not the stream codec.

Freeze these invariants before implementing the codec:

- Require correct format marker/version; reject unsupported versions, enum values
  and unknown structural fields with useful errors. Optional documented fields
  have explicit defaults. The metadata map is the only open-ended extension area.
- Every point has exactly two finite JSON numbers. Reject nulls, NaN/infinity,
  duplicates, decreasing x-keys and values that overflow conversion. Do not sort
  data or drop malformed points silently. Validate representable numeric extents
  and histogram edges before engine construction. Keys increase within each series;
  series may have different keys, but all use the same numeric x-coordinate system.
- v1 numbers use finite f64 semantics; document precision limits and advise
  relative/normalized keys instead of large integer nanosecond timestamps. Do not
  promise exact arbitrary-size integer storage.
- Empty series are legal; require at least one named series. An all-empty chart
  displays an explicit empty state. A legend maps names/colors to plotted data.
- x spacing is explicitly `linear`; axis precision is an integer from 0 to 12;
  color, if present, is `#RRGGBB` only. Histogram series require positive finite
  `bar_width` in x units; line series reject that field. Labels are plain text.
- Initial reader budget: 32 MiB of input bytes, 32 series, 250,000 total points,
  1 KiB per title/label/name, and 64 metadata entries of at most 4 KiB each.
  Enforce the byte limit during reading (limit + one byte), not just from file
  metadata; validate the remaining budgets before engine construction. Preserve
  serde_json's recursion limit. These caps bound input, not a promised RAM ceiling.
- Accepted v1 fixtures remain readable throughout v1. A breaking semantic or
  structural change requires a new format version; never serialize engine structs
  directly. File format version and package version are separate concepts.
- Writers validate before emitting bytes and return I/O errors. Producer examples
  use create-new output so existing results are not silently overwritten. A failed
  stream write can leave a partial file; examples must report failure and clean up
  only the partial output they created. A successful return means a complete file.

## Delivery sequence

1. Freeze the v1 contract and irregular-x fixtures. Trace the shared horizontal
   projection consumers and decide the opt-in numeric-axis API (Task 2A).
2. Add the headless file package/workspace and test reading, writing and validation.
3. Deliver the continuous axis in bounded slices: numeric projection/ranges, ticks
   and interaction, then histogram geometry and existing-domain regressions
   (Tasks 2B-2D). Prove the 1:98 spacing requirement before integrating the viewer.
4. Map a document into the new continuous numeric chart through an app-owned adapter.
5. Deliver CLI viewing and native interactions, then reusable Open/Reload/drop
   behavior with recoverable errors and visible document labels.
6. Deliver language-independent benchmark production and convert the existing
   profiler CSV after measurement. Keep raw source metrics and units intact.
7. Deliver Linux installation/file opening; validate both renderer backends and
   record acceptance evidence and documentation.

Detailed tasks, dependencies and verification are in `fchart-todo.md`. Implement
in dependency order; no parallel agents are required for this contract-dependent
feature. Checkpoints assess completed work without implying authorization to
publish, modify desktop defaults, or begin implementation during planning.

## Verification and completion

Focused codec tests cover valid round trips, malformed/unsupported input, finite
numbers on both read/write, ordering, empty data, resource limits and I/O failure.
Adapter tests assert series count/type, exact original keys/values, precision,
axis units and rejection without replacing the current document. Proportional
spacing must also pass the connected engine tests listed above. One standard
library Python-generated fixture must pass the Rust reader.

Native acceptance: open a real benchmark document, inspect sample values, pan,
zoom, Fit, resize, change scale factor, reload, drop a replacement and try a broken
file. Error messages must identify the document/field and preserve the last valid
chart. Exercise paths with spaces, no startup path, missing file and invalid JSON.
Test TinySkia and WGPU. Record checks not possible on available hardware rather
than treating synthetic/high-DPI capture as physical hardware acceptance.

At integration checkpoints, from the repository root:

```sh
cargo fmt --all --check
cargo test -p feather-chart-file
cargo test -p feather-charts-app
cargo check --workspace --all-targets
```

At final acceptance, additionally run `cargo test --workspace --all-targets`,
`cargo test --workspace --doc`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo build -p feather-charts-app --release`, and `git diff --check`. Do not weaken
existing tests/lints. Run affected engine/reference regressions for the new axis;
repeat expensive profiling only when projection-path changes justify it. Confirm the existing library package can still be packaged
with `cargo package -p feather-charts --list`; publishing is separate work.

Success requires a fresh producer -> .fchart -> installed viewer flow, plus a
Linux file-manager open test in an isolated user data prefix before testing the
user's actual associations. A source build and terminal launch do not establish
desktop integration. Do not install or change this machine's defaults as part of
planning.

## Risks and decisions to revisit

| Risk | Mitigation |
| --- | --- |
| Shared engine changes break existing horizontal domains | Opt-in continuous mode, complete consumer trace and unchanged-domain regression tests before viewer integration. |
| Geometry is numeric but labels/interaction remain index-based | Test numeric ticks, inverse projection, hit testing and zoom anchoring end to end, including x=1,2,100. |
| Histogram widths still use index spacing | Explicit numeric bar_width and edge projection; verify fit, clipping and picking with irregular x. |
| Numeric precision loses distinct keys | Use relative sample keys, validate strict ordering after parsing, test large/near-equal numbers. |
| Large valid input stalls construction or duplicates memory | Bounded read/point budgets, background parsing, measured 100k-point smoke check; reduce copies before adding infrastructure. |
| Feature changes profiling results | Serialize/convert only after collection; retain frames.csv and metadata.txt as authoritative evidence. |
| Workspace changes lose renderer patches/profile behavior | Keep patches/profiles at root and test both backends plus root library packaging. |
| Future files silently lose meaning | Strict version/type handling and committed v1 fixtures; no internal state serialization. |
| File association conflicts or destructive producer output | Scoped Linux MIME entry for `*.fchart`; no forced default changes; create-new producer examples. |

Product requirements are settled: proportional linear numeric spacing and the
`feather-charts` executable are required in v1. Proposed limits and wire field names
are defaults to freeze in Task 1. Numeric-axis design/caller analysis is an explicit
early engineering task, and makes this larger than a file-format/viewer-only change.
Windows/macOS packaging, additional series and multi-pane documents remain deferred.

## Implementation notes (2026-10-31)

The frozen wire contract is in `docs/fchart-format.md`; numeric API, consumer trace,
ownership and alternatives are in `docs/fchart-numeric-axis.md`. Implementation
uses an explicit opt-in viewport in TimeScale via `OptionsChart::new_numeric`.
Existing domains retain their defaults. `feather-chart-file` is headless; the
viewer owns document/chart replacement and uses the native executor for parsing.
Acceptance evidence and unavailable graphical-session checks are recorded in
`docs/fchart-acceptance.md`. No desktop defaults were installed or changed.

### User follow-up: presentation change

After successful native acceptance, the user requested a chart-only viewer and
a compact crosshair/OHLC readout. This supersedes permanent app controls and
surrounding document-label/legend panels. The format fields remain unchanged;
Ctrl+O/Enter, Ctrl+R, Ctrl+0, Escape and file drop preserve access to operations.
Readout is opt-in at the library widget boundary and enabled by the app and
built-in demos. Existing plain `element()` and engine screenshot output retain
their behavior. `.fchart` v1 remains line/histogram; existing library candle/bar
series supply OHLC values to the reusable readout.

# Screenshot/export contract

Implemented; physical mouse/export acceptance pending. Approved plan baseline `23476f1` (763 tests, 34 source
oracles, prior 96 exact compatibility comparisons). Earlier acceptance exceptions
remain independent. See `tasks/remaining-features-todo.md` for actual progress.

## Source and consumers

Local 5.2.1 ChartApi.takeScreenshot flushes pending chart drawing. ChartWidget
traverses complete pane/axis/separator/stub bitmap regions. Pane, price-axis and
time-axis widgets composite their base canvas and optionally their top canvas.
Graphics tests use the result as a full-chart regression image; pane coverage
uses multiple panes. Source graph takeScreenshot consumer edges were sparse;
actual methods, coverage cases and graphics helper were read directly. Series
metadata was stale and its actual view methods were read. Vendored renderer
sources are excluded from indexing and were read directly.

Real Chromium probe: `node scripts/check_screenshot_reference.cjs`; frozen source
hashes and observations are adjacent screenshot-source/states.json. Both flags
false and crosshair-only give identical images. Top-only includes the top
primitive. Top+crosshair includes the positioned crosshair. Crosshair mode is
restored by the source. Native filtering will not mutate live chart options.

Layer contract: normal series (including hovered-source raising), normal and
above-series markers, price lines, pane labels, bottom/normal primitives and
watermarks belong to base content. Top primitives, top markers and the price pulse
require top inclusion. Crosshair strokes/markers and price labels additionally
require crosshair inclusion. Time primitive labels require top inclusion; price
primitive labels are base. Source label pane views paint on both canvases.

## Design and ownership

An owned ChartScreenshot retains a clone of the last successfully displayed frame;
a fresh capture prepares the current viewport first. Failed display preparation
invalidates prepared capture. Snapshot-only preparation does not replace the
last displayed frame. Fresh capture preserves queued host events and follows yield
settlement. No temporary resize, event drain or live option patches.

Snapshot -> owned frame; reusable exporter -> Iced renderer/font resources;
image -> owned bytes. No backreference to Chart. Custom renderer snapshots retain
the existing frozen-view contract. Whole-window capture and per-call renderer
construction were rejected: unrelated controls/layer limitations and expensive
resource churn respectively.

API: ScreenshotLayers, ChartScreenshot, ScreenshotRasterOptions, ScreenshotImage,
ChartExporter, ScreenshotError; Chart::screenshot_of_prepared and
Chart::prepare_screenshot, with explicit yield forwarding. Renderer initialization
is a one-time Future; engine preparation and image readback remain synchronous.

Sizing: ceil(logical width/height * positive finite ratio), or explicit physical
canvas dimensions. Explicit canvas fits uniformly and centers the original layout;
ratio is still validated but does not override that canvas. Padding uses the
requested background (transparent by default). Limit: 8192 pixels per dimension
and 64 MiB RGBA output, with checked arithmetic before allocation/readback. This
fits Iced WGPU's requested default texture limit. No chart reflow or stretching.

Pixels: row-major top-to-bottom RGBA8 from the actual backend. Alpha semantics
will be verified on both backends before A4 acceptance; Iced tiny-skia's raw
readback is premultiplied, so normalized public straight-alpha output is required.
Iced readback does not expose a Result; GPU device-loss/map errors and allocation
failure can panic or abort. Validation prevents invalid user dimensions; the API
must document this underlying boundary rather than promise complete recovery.
Core has no file/codec responsibility. A host writes raw RGBA/PPM and previews via
Iced's image handle. No new async chart machinery or encoding dependencies.

## Validation

772 tests in 75 suites, both doctests, all-target check, strict all-target Clippy,
formatting and diff hygiene pass. All 34 unchanged source comparisons plus the new
screenshot oracle pass. Both explicit renderers pass 1158 export captures, including
six alpha/uniform-fit cases; previous event captures (288) and all 96 exact image
baselines pass unchanged. Normal 800×500 images from both renderers and the short
300×90/fractional-DPR WGPU image were inspected. Native/browser fonts and backend
antialiasing are not claimed to be pixel-identical.

Evidence is retained under `target/screenshot-export/reviewed`,
`validation-reviewed` and `oracles`. Source flags are frozen in the adjacent
screenshot-source/states fixtures. Physical mouse/preview/file acceptance is still
pending in all three modes on both renderers; no prior physical result is reused.

Frozen native matrix (before final replay): 3 existing horizontal modes × 3 chart
sizes (800×500, 300×300, 300×90) × 4 DPRs (1, 1.25, 1.5, 2) × 4 scenes (base,
positioned crosshair, transferred built-in/custom series, hidden horizontal axis)
× 4 layer selections × 2 explicitly asserted backends = 1152 feature images.
Six extra alpha/uniform-fit images cover each mode/backend. Every image is replayed
from the same snapshot; a retained full-layer snapshot is replayed after pane
mutation and owner drop. The previous 96 compatibility baselines stay unchanged.

## Implementation choices and migration

Filtering clones of prepared geometry was chosen over branching the ordinary draw
path. Axis preparation records overlay command ranges; glyph conversion carries
that classification alongside owned paths. Capture filters both without redoing
text layout. Normal on-screen command order, glyph preparation and drawing stay
unchanged. Low-level users constructing PreparedPriceAxis/PreparedTimeAxis literals
must initialize the new screenshot ranges (or use `..Default::default()`); ranges
remain attached to the commands they classify. ChartError gains a Screenshot variant.

Native capture intentionally replaces the source's temporary crosshair-option
mutation with immutable filtering. DOM/canvas identities, manual canvas ownership
and window-level file actions are not copied. No live chart references, new custom
renderer Clone bounds, async model work or thread-safe ownership were introduced.

RGBA normalization handles tiny-skia's premultiplied bytes and WGPU's premultiplied
linear color followed by sRGB readback. Transparent pixels are canonical zero RGBA;
partial-alpha RGB is unpremultiplied (with normal 8-bit quantization). Export padding
is drawn through the renderer over transparent clear so it shares that contract.
Default ratio sizing preserves the exact requested scale/origin; ceil-rounded extra
buffer pixels do not recenter the scene. Only explicit canvases apply centered fit.
A populated 300×90 chart at 1.25× failed the screen/export placement comparison
before this correction and passed after it.

The first matrix stopped on its image-count assertion: Path::with_extension treated
fractional-ratio identifiers as extensions and overwrote images. The example writer
now appends suffixes without altering identifiers; its file round-trip test uses a
`1.25` filename. The failed 300-image software run remains under
`target/screenshot-export/initial` and is not acceptance evidence.

Source labelPaneViews are replayed on the optional top canvas, above other top
content. Capture retains the layer policy for that final clipped replay. A
translucent pane-label test failed without the replay and passed with it. This is
export composition only; ordinary native frame drawing is unchanged.

## Changed files and behavioral tests

New Rust modules: `src/api/screenshot.rs`, `src/api/exporter.rs`,
`src/ui/screenshot_layers.rs`. Existing ownership/forwarding/export declarations:
`src/api/chart.rs`, `src/api/yield_curve.rs`, `src/api/mod.rs`. Axis classification:
`src/renderers/price_axis_renderer.rs`, `src/renderers/time_axis_renderer.rs`,
`src/ui/price_axis.rs`, `src/ui/time_axis.rs`, `src/ui/chart_frame.rs`, `src/ui/mod.rs`.
Ordinary plot/series drawing modules were not modified.

Public integration tests: `tests/screenshot_export.rs` covers valid/invalid display
lifecycle, all frontends, pending versus fresh geometry, queued events, snapshot-only
preparation, mutation/removal/drop, all four flags, transparent/colored padding,
file-independent pixel ownership and exact fractional-DPR placement. Module tests
cover bounded raster dimensions/precedence, normal/top annotations, price/time axis
labels and translucent label replay on both canvases.

Demo: `examples/pane_management.rs`, `examples/pane_management/host.rs`, new
`export.rs` and `screenshot_captures.rs` in the same directory. The host tests
capture/ratio/fit/preview, file round-trips with decimal filenames and reset in all
three horizontal modes. F8 captures while retaining plot hover; preview remains
frozen as the live chart changes. Hidden previews restore chart space. File saving
is an explicit host action, outside the library.

Reproduction: `scripts/check_screenshot_reference.cjs` and
`scripts/capture_screenshot_export.py`; the old source/capture checks are unchanged.
README and the feature-specific roadmap/checklist record usage and remaining work.

## Remaining dependencies and review

The public custom-horizontal factory and attribution milestones have not started.
Attribution must remain outside canvas export as specified in their approved plan.
PNG convenience, different logical-layout export, SVG/PDF and host file dialogs
remain outside this screenshot milestone. Existing timing, drawing-order mouse and
unavailable-device exceptions stay independent.

Review checked correctness (source placement/layer/lifecycle rules), readability
(small capture/export/filter modules), architecture (owned frames and reusable
renderer), bounded user inputs/file ownership, and runtime costs (no device or
renderer initialization on chart frame/input paths). Source consumers were reread
after implementation; the label-canvas replay issue found in that review has a
red/green regression. No dependency, engine synchronization or async policy changed.

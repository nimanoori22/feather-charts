# Native attribution logo

Milestone C, authorized 2026-10-18. Baseline `d9095de`. Implementation, automated gates and available-mouse software/GPU acceptance complete.

## Source contract and consumers

The complete local 5.2.1 `src/gui/attribution-logo-widget.ts` creates a DOM anchor
with two even-odd SVG paths (35×19), at left/bottom 10px of the last pane's plot
cell. PaneWidget.setState owns creation/removal as panes change; ChartWidget's
screenshot composes canvases and excludes this DOM child. The anchor owns browser
navigation and sits above chart input. Source-only Chromium probes freeze bounds,
last-pane transitions, theme, plot-click isolation and screenshot exclusion before
native changes. Graph class edges omit private calls; actual source bodies and
consumers were read. Coverage checks are best effort, not exhaustive proof.

Theme uses text-color grayscale >160, irrespective of background. Source Number
arithmetic classifies #a0a0a0 as dark because its weighted sum is slightly above
160. Preserve f64 arithmetic for this logo rule, leaving existing f32 chart contrast
behavior unchanged. Theme colors are #131722/#fff (light), #D1D4DC/#131722 (dark).

Native default stays off. Typed runtime visibility is an explicit native extension:
the source widget warns that dynamic option responses are unsupported. Source fixed
vector provenance/notices remain intact. Host destination is fixed to
https://www.tradingview.com/ with no browser hostname/path tracking. Engine code does
not open URLs. Enabled logo exports remain identical to disabled exports for every
layer policy. Clip drawing and hits to the last plot, including tiny empty panes;
this prevents overflow into adjacent panes/axes in native layouts.

## Design, alternatives, ownership and API

Use immutable prepared vector geometry on the chart frame, above chart content.
Chart -> frame -> prepared logo -> Rc<immutable paths/style>. Future frames share
fixed paths; retained frames own their bounds/theme. No owner callbacks, strong
backreferences, cycles or manual destruction. Disabling removes the decoration;
screenshot filtering always removes it. No runtime SVG parser or new dependency.

A watermark has incorrect export and ownership semantics. A separate widget-only
overlay omits attribution from ordinary frame drawing and complicates public
ChartProgram use. Prefer a prepared frame decoration and a focused Canvas input
recognizer ahead of chart gesture routing. Precompute SVG path commands offline;
use native cached paths with even-odd fills and the original clip box.

Add `LayoutOptionsPatch::attribution_logo: Option<bool>` and readback through
`LayoutAppearance::attribution_logo`. Existing construction flag remains supported.
Add `ChartMessage::AttributionActivated` and default-enabled user action
`ChartEvent::AttributionActivated { url: &'static str }`. The owner verifies current
visibility before emitting the fixed link. Hosts decide whether to open it; demos
only display the action. Update exhaustive message/event matches and document the
new enum arms. No event subscription identity or browser DOM sharing is reproduced.

Mouse/touch recognition uses clipped bounds and stable last-pane identity. Click/tap
wins over plot/primitive recognition. Movement beyond the existing 5px mouse / 8px
touch slop, release outside, Escape, focus loss, resize/reorder/removal or visibility/
theme change cancels activation. Ongoing plot/axis/divider/edit/touch gestures retain
ownership if they cross the logo. No plot click/double-click or edit begins from a
logo press. Touch mouse synthesis is suppressed; actual physical touch remains an
independent unavailable-device check. Widget state owns only finite capture state.

## Verification

Freeze real source behavior/hashes before implementation. Test configuration,
placement/theme precision, cache/snapshot lifetime and export exclusion. Replay
Canvas mouse/touch sequences for activation, cancellation and input priority.
Run unchanged old source/raster gates plus enabled/disabled/theme/pane/tiny captures
on actual software/GPU renderers across calendar/strike/maturity/custom domains.
Run heavy checks before physical acceptance. Retain red failures and prior artifacts.


## Implemented behavior and migration

`LayoutOptionsPatch.attribution_logo` and `LayoutAppearance.attribution_logo` expose
runtime changes/readback without adding layout slots. The current frame prepares
one immutable decoration after panes settle; future frames share paths and unchanged
style. The vector generator converts the exact source lines and circular arcs into
native commands (at most 45-degree cubic segments), checked against pinned source
hashes. No SVG parsing occurs in hot paths. Original LICENSE/NOTICE are retained in
`docs/porting/attribution-source/`; this adds no new licensing interpretation.

The source anchor is a sibling of the canvas to which MouseEventHandler is attached.
Native hover therefore shields the plot and clears its crosshair, while redraws
continue animation/expiry scheduling. Bounds, last-pane identity, visibility/theme,
focus/Escape/resize/DPR and move thresholds control click/tap lifetime. Synthetic
mouse events after a touch are suppressed for 500ms. Existing plot/axis/edit owners
win when their ongoing gesture crosses the overlay. Half-open hit bounds preserve
adjacent axis ownership even in tiny layouts.

Default user-action events carry a fixed static URL; they do not depend on opt-in
data/range interests. Public messages allow hosts to request the same action when
a logo is currently prepared/enabled. All exhaustive example matches were migrated.
Downstream exhaustive matches must add both new enum arms; complete layout/frame
literals must include their new attribution fields. Existing default-off behavior
is unchanged. The shared demo now also accepts the actual session/phase domain,
without a new engine domain or private access.

## Behavioral coverage and review

External tests cover construction/runtime readback, layout stability, default click
on empty charts, cancellation, touch redraw/multi-touch/synthetic mouse, exact theme
boundary, last-pane identity, tiny clipping, all export flags and owner drop. Native
pixel checks require visible screen changes to stay inside the logo. Overlay tests
cover overlapping editable primitives, ongoing plot/axis/edit ownership, cleared
hover and continued animation deadlines. A renderer unit test checks immutable
path/style sharing across frame changes and after chart drop. Shared controls run
in calendar, strike, maturity and session/phase modes.

The initial missing-configuration and missing-event compile failures are retained.
Removing screenshot exclusion was detected by the export regression test; the
mutation was restored before final validation. Review covers Result paths, private
prepared invariants, ownership/cycles, bounded gesture state, source correspondence,
clipping and no per-frame vector/style parsing. No dependencies, Tokio, channels or
thread-safe engine ownership were added.

## Changed files

Production: `src/api/{chart,interaction,widget,mod,attribution_input}.rs`,
`src/model/chart_configuration.rs`, `src/model/chart_model/configuration.rs`,
`src/renderers/{mod,attribution_logo_renderer,attribution_paths}.rs`,
`src/ui/{multi_pane_frame,screenshot_layers}.rs`.
Consumers: `examples/pane_management.rs`, its `host`, `configuration`, `attribution`
and `attribution_captures` modules; `examples/{public_chart,options_chart,yield_curve}.rs`,
`examples/support/chart_demo.rs`, `tests/series_markers.rs` (exhaustive-match migration).
Tests/provenance: `tests/attribution.rs`, `scripts/{check_attribution_reference.cjs,
generate_attribution_paths.py,capture_attribution.py}`, source/states fixtures and
preserved notices. README, roadmap and checklist document status and migration.

## Remaining dependencies and limits

The native logo has no unfinished engine dependency. Host URL opening is deliberately
host policy, and this demo only displays the request. Browser title/focus/navigation
infrastructure is not recreated by a Canvas overlay. Existing timing, drawing-order
and unavailable touchpad/touchscreen/high-DPI physical checks remain independent;
synthetic touch and fractional-DPR captures are not physical-device acceptance.


## Renderer-matrix findings

The first full matrix reproduced a populated-chart error that the empty-chart test
could not expose: a visibility-only patch requested Full invalidation. Axis allocation
then shrank from 56 to 46 pixels, moving chart content and changing exported pixels.
Before/after RGBA/PNG images and a failing populated-layout regression are retained.
A pure decoration patch now queues Light invalidation without touching autoscale or
resetting settled axis allocations. Existing geometry/scale configuration keeps its
original path. The regression passes with identical layout and exported bytes.

Input review also found that using the generic navigation-cancel message for overlay
hover stopped time-scale animations. Source mouseLeaveEvent only resets tracking/
hover/crosshair; native hover now sends a position-only clear with no navigation.
The failing animated-scroll test and passing repair are retained. Price-pulse and
time-scale animation scheduling both have focused coverage. These fixes do not
resolve or close the earlier independent timing exception.

The repaired pre-final input-review matrix passed 672 screens, 2688 exact visibility/
export comparisons and 2688 retained-export comparisons across both actual backends.
Its artifacts are preserved; final-source replay and compatibility validation follow.


Iced's exact Path/Fill/Frame APIs were checked for arc conversion, even-odd rules,
headless drawing and clip/transform behavior. Flowsurface's OverlayCanvas update and
cursor methods were inspected as a Canvas HUD example: controls are recognized before
panning and receive a pointer cursor. Its graph has no concrete trait callback edges;
actual snippets were read with clean best-effort path coverage. Existing native
primitive/touch routing remains the implementation foundation; no Flowsurface
architecture or code was copied.


## Final automated results

Final source at `e037a3b`: 796 tests across 78 suites and two doctests pass, along
with formatting, all-target check, strict Clippy, generated-vector verification and
all 37 local-source comparisons. Actual software/GPU replay passes 672 screen images,
2688 exact export-exclusion comparisons and 2688 retained-export comparisons over
four domains, seven scenes, three sizes and four DPR values. Both strict renderer
identities were asserted. Final screen images match the pre-final input-review set.
Representative light/dark/mismatched/tiny images were visually inspected.

The unchanged legacy gate passes 288 event captures and 96 exact baseline images.
All 240 accepted custom-domain images and all 1158 accepted screenshot images match
exactly on the final source, including the original layer/alpha-fit scene matrix.
No baseline, threshold or expected scene count was weakened. Failed runs and image
differences remain under `target/attribution`. Heavy validation finished before the
mouse sessions. See [attribution-evidence.json](attribution-evidence.json).


## Final physical acceptance and completion

On 2026-10-18 the user reported “All checks passed in all four modes;
previews/exports passed; windows closed” separately for tiny-skia and wgpu.
Each group covered calendar, strike, maturity and session/phase charts: logo
visibility/theme and host link requests, drag-off cancellation, pane lifecycle,
live-feed navigation/price-axis/box interaction at three window sizes, short-height
recovery and screenshot layer/ratio/canvas/preview controls. All eight processes
exited with status zero using the recorded final-binary SHA-256.

The GPU run retained a 640×480 straight-alpha sRGB RGBA8 save with the exact expected
byte length. The shared save path retains only the final image; no separate saved
artifact per mode is claimed. No software-run saved file was present for inspection;
that run's preview/export result is the user's report. Automated file round-trip
tests provide separate serialization coverage.

The integrated checks also supply A6's previously missing backend-specific
screenshot acceptance. All A/B/C feature milestones and combined final validation
are complete for the available mouse. Existing timing, dedicated drawing-order and
unavailable physical-device exceptions remain independent. Changes are local and
have not been pushed.

# Software pane clipping correction

Date: 2026-11-13. Reported from Sorena: vertically moved chart content can draw
into adjacent panes instead of disappearing at the pane edge.

## Source check and behavior

A fresh GitHub API/raw-source check inspected upstream Iced master commit
[`b2fbd3cb6289603f8cbbb2387a5242231f509d13`](https://github.com/iced-rs/iced/blob/b2fbd3cb6289603f8cbbb2387a5242231f509d13/tiny_skia/src/geometry.rs).
Its `Frame::paste` still extends child primitives/text/images without retaining
the child's clip rectangle. This check establishes the current default branch's
behavior, not the absence of every unmerged pull request.

The indexed Lightweight Charts reference creates separate pane canvases inside
an `overflow: hidden` wrapper (`src/gui/pane-widget.ts`, constructor), then paints
background, sources and crosshair into those pane-sized surfaces. The observable
contract is isolation: content outside a pane is clipped, including stroke width.
Separate DOM canvases and CSS stacking are incidental implementation choices.

Feather's `PlotSnapshot::draw_base_at` and `draw_crosshair_at` already request a
pane clip and translate plot-local geometry into that region. Sorena assigns
separate panes and sizes; it does not request overlap. The loss happens below that
boundary in the vendored Iced software renderer. Vendor files are excluded from
the codebase-memory graph and were read directly, including every primitive
constructor and the rendering, cache and damage consumers. The separate indexed
Iced source provides the draft/paste API contract.

## Design and ownership

Two approaches were considered:

- Retain nested drawing groups and rasterize them recursively with inherited
  clips. Correct in principle, but adds a new primitive/container and recursive
  drawing/damage paths.
- **Retain the clip on each recorded command**, intersect at merge, and apply it
  at rasterization. This reuses the existing flat command/cache ownership and
  image clip fields; it is the smaller change for the actual consumers.

The second approach was selected. A frame owns its vectors; paste moves commands
into the parent while intersecting their logical clips. Cached geometry owns the
same immutable commands through its existing Arc. No new callbacks, shared
mutable ownership, async work or public Feather API is introduced.

Fill/stroke primitive fields gain `clip_bounds: Rectangle`; image commands reuse
their existing clip. `Primitive::clip_to` intersects a command with its parent.
Canvas-generated cached text carries/intersects the frame clip too. Rasterization
intersects the transformed command clip with the existing render/damage clip and
refreshes masks when needed. Cached glyphs always use the effective mask: layout
bounds alone cannot prove all glyph ink is inside. Damage bounds include stroke
width and respect the retained clip. Coordinates, gradients, source data, series
order and WGPU paths are preserved.

## Regression evidence

All five new `tests/canvas_clipping.rs` tests failed on the original code and pass
with the patch. They rasterize through the actual tiny-skia backend and retained
canvas cache, without a native window:

- Fills and wide strokes at nonzero fractional pane origins and 1x/1.25x/2x scale.
- Nested child/parent intersections, including a child larger than its parent.
- Empty/disjoint nested clips drawing nothing.
- Cached text and raster images inside nested pane clips.
- Sibling clip masks, insertion order and subsequent unclipped drawing.

The first native minimal reproduction recorded 7,536 colored pixels outside the
requested pane clip with software rendering; the same reproduction on WGPU had
zero. After the patch the software reproduction also reports **zero** escaped
pixels. Screenshots were visually inspected. The fixed capture dimensions differed
because the tiling compositor chose a different window width; the pixel predicate
uses actual image dimensions and identical logical clip bounds.

Native probe source/captures remain under `/tmp/pane_clip_probe*` and
`/tmp/pane-clip-*.png` in this session; the durable automatic regression is the
repository test, not those temporary artifacts. No market data or credentials are
needed. This does not claim physical pointer, every GPU/OS, or every possible
font/transform validation.

The full library/integration suite passed: **789 tests**, zero failures or ignored
tests. All-target Clippy with warnings denied, workspace formatting and upstream
80-column vendor formatting passed. No unrelated dependency versions changed.

## Verification commands

```sh
cargo test -p feather-charts --test canvas_clipping --locked --offline
cargo test -p feather-charts --lib --tests --locked --offline
cargo clippy -p feather-charts --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
rustfmt --edition 2024 --config max_width=80 --check vendor/iced_tiny_skia-0.14.1/src/{geometry,primitive,engine}.rs
```

The vendor keeps upstream Iced's 80-column formatting. Tests use the repository's
existing renderer dependencies; no package was added. Sorena's Git dependency and
its two renderer patches must be pinned together when adopting the commit.

## Upstream follow-up

The downstream fix was merged in [Feather PR #1](https://github.com/nimanoori22/feather-charts/pull/1).
[Iced issue #3545](https://github.com/iced-rs/iced/issues/3545) reports the defect.
A focused patch adapted to current master is published on
[nimanoori22/iced:fix/tiny-skia-nested-canvas-clips](https://github.com/nimanoori22/iced/tree/fix/tiny-skia-nested-canvas-clips).
All seven final upstream regressions fail before the fix and pass afterward;
geometry-only tests and affected-package Clippy pass too.

PR submission waits for core-team feedback as required by Iced's contributing
guide. The [prepared PR text](evidence/software-pane-clipping/upstream-pr.md)
records the exact commands and limits so the contribution can resume later.

Native reproduction, before and after (the compositor negotiated different window
widths; both use identical logical clip boundaries):

[Before: content escapes the green rules](evidence/software-pane-clipping/before.png)
· [After: content stays between the rules](evidence/software-pane-clipping/after.png).

# Image watermark execution checklist

Plan: [image-watermark-plan.md](image-watermark-plan.md).
Starting checkpoint `f4f5f63`; implementation and software/GPU mouse acceptance completed on 2026-10-17.
Files are provisional; split any slice exceeding about six files. Checkpoints
are verification gates, not additional user-permission requirements.

## 0. Freeze contract and reproduce renderer ordering

Dependencies: none. Scope: 2–4 fixture/documentation files.
Likely files: source manifest, port report, renderer overlap fixture, baseline protocol.

- [x] Pin built-in TS sources/consumers and pane-widget exact bottom draw order;
  resolve max=0, upscaling, invalid values and nonpositive drawable-area policies.
- [x] Record failing image/vector ordering checks on TinySkia and WGPU, default-off
  baselines and current software damage/work at regular/narrow/problem sizes.
- [x] Compare both renderer designs, define owned RGBA API/error cases, public draw
  compatibility and the minimal feature/dependency changes before production edits.

Verification: source behavior has test cases; the renderer risk is reproduced,
not assumed solved by canvas call order. No timing threshold edits.

## 1. Prove the selected ordered rendering path

Final choice: the renderer-fix alternative. Only six vendored backend files changed;
Chart/widget/drawing APIs needed no adapter changes. The original likely-file list
below describes the rejected chart-local option, not additional work still due.

Dependencies: 0. Scope: 3–5 shared drawing/widget/test files per slice.
Likely files: `src/api/widget.rs`, `src/api/primitives.rs`,
`src/ui/chart_frame.rs`, plot drawing adapter, focused ordering fixture.

- [x] Submit image/vector phases in actual source order, preserving pane clipping,
  primitive attachment interleaving and existing public draw consumers.
- [x] Pass image/background/grid/filled-series/text/overlay occlusion checks on
  both backends, including multiple images and vector bottom primitives.
- [x] Keep image-absent path/output intact and compare software damage/redraw work
  using the unchanged baseline fixture; address proven regressions before continuing.

Verification: headless/native ordering and repeated-frame checks; existing widget,
primitive and software-damage regressions. No per-pixel vector conversion.

## Checkpoint A

- [x] Renderer feasibility, compatibility and damage gates pass.

## 2. Deliver one visible decoded image

Dependencies: 1. Scope: 3–5 API/native preparation/test files per slice.
Likely files: `src/api/image_watermark.rs`, API exports,
`src/ui/image_watermark.rs`, UI exports, `tests/image_watermark.rs`.

- [x] Add defaults and typed errors; validate positive dimensions, checked RGBA
  length, numeric options and finite drawing conversions before chart mutation.
- [x] Record a compiling public visible-output RED, then draw one centered image
  on the bottom layer with proper source-alpha multiplication and pane clipping.
- [x] Retain old prepared frames through replacement/removal; no hit target,
  autoscale, scheduler, IO or decoding in engine preparation/draw.

Verification: public visibility, alpha and frozen-frame tests on both backends.

## 3. Complete source geometry and option/image replacement

Dependencies: 2. Scope: 3–5 layout/API/oracle/test files.
Likely files: `src/renderers/image_watermark_renderer.rs`, layout tests/exports,
`scripts/check_image_watermark_reference.cjs`, API tests.

- [x] Match rounded centers, width/height aspect fit, allowed upscaling, padding,
  independent maxima and inactive zero maxima against actual TS renderer code.
- [x] Test square/wide/tall, odd sizes, transparency, limits and degenerate areas;
  freeze oracle fixture counts before execution and separate native filtering.
- [x] Add shared-resource with_options updates and image replacement; rejected
  options/buffers preserve the current attachment and stable ID/order.

Verification: actual-source layout/default/update/detach oracle; invalid input and
public image/option replacement tests.

## Checkpoint B

- [x] Visible behavior/source geometry and mutation safety pass.

## 4. Verify lifecycle and work bounds

Dependencies: 3. Scope: 2–4 tests/preparation files.
Likely files: public image tests, resource/layout work counters, mixed-scene fixture.

- [x] Cover alpha=0/restore, replacement/removal, old frames, multiple attachments,
  pane removal/index shifts and preserved empty panes; prove image lifetime release.
- [x] Check source order and no changes to hit winners, axes/autoscale, text
  watermarks, markers, pulse and editable box interactions.
- [x] Prove no pixel copy/resource recreation on options-only or unchanged updates,
  no history scanning on resize, and no watermark preparation on pulse-only ticks.

Verification: lifecycle, observable resource work and affected existing suites pass.

## 5. Provide the public demo

Dependencies: 4. Scope: 2–3 demo/documentation files.
Likely files: `examples/public_chart.rs`, `examples/public_chart/image_watermark.rs`, README.

- [x] Add --image-watermark with an owned deterministic RGBA logo fixture and
  opacity, padding/limits and image replacement controls using existing IDs.
- [x] Combine with text, live feed, pulse and editable primitives; default-off
  UI remains unchanged and narrow/short windows recover correctly.
- [x] Document decoded input and host loading/failure handling; make no promise
  of URL/encoded-format/SVG decoding in the core.

Verification: demo state/control tests and both native smoke runs.

## Checkpoint C

- [x] Lifecycle, work bounds and public demo pass.

## 6. Capture and physical mouse acceptance

Dependencies: 5. Scope: 2–4 capture/protocol files.
Likely files: example capture helper, capture runner,
`docs/porting/evidence/image-watermark/`.

- [x] Freeze the plan's 120-case feature matrix, 24 default-off comparisons,
  fixture/binary/source identity, repeated-frame and analytic alpha assertions.
- [x] Run both renderers at DPR 1/1.25/1.5/2; inspect representative transparent,
  filled-overlap, empty and short-height images. Retain failed runs and root fixes.
- [x] Record mouse wheel zoom, diagonal pan, price-axis wheel/drag, box editing,
  watermark controls and short-height recovery at regular/narrow/problem sizes on
  both renderers. Record unavailable touch/high-DPI hardware separately.

Verification: unchanged finite captures and recorded native results pass;
no old timing acceptance threshold or device status is silently changed.

## 7. Validate and document

Dependencies: 6. Scope: checks/docs, 3–4 files.
Likely files: README, image port report/source/evidence, this checklist.

- [x] Pass cargo fmt --check, cargo test --all-targets, cargo check --all-targets,
  cargo clippy --all-targets -- -D warnings and affected source oracles.
- [x] Review API compatibility, image resource ownership/release, input validation,
  paint order, default-off behavior, damage work and Cargo feature/lock changes.
- [x] Publish exact files, source/consumer contract, alternatives and remaining
  host-loader/format limits with native and automated results.

## Checkpoint D

- [x] Implemented decoded-RGBA image watermark gates pass.
- [x] Software/GPU mouse acceptance and unavailable devices are recorded.
- [x] Historical timing exception remains OPEN/deferred with unchanged thresholds.

## Recorded results

636 tests; check, strict Clippy, formatting; image/text/animation/conflation oracles;
120 captures and 24 unchanged default-off comparisons pass. Software damage stays
one region at all three acceptance sizes. Native direct/cached image order, alpha,
clipping and cached removal/restoration pass on both backends.

Software and GPU user responses: “All checks passed; window closed.”
Physical touchpad, touchscreen and high-DPI remain untested. Input is decoded RGBA;
codec/URL/SVG loading belongs to the host. See the report for patch maintenance
and original fixture failures. Historical timing acceptance is unchanged.

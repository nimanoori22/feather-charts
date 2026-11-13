# Prepared upstream PR

Status: tested patch published on the fork; PR submission awaits Iced core-team
feedback on [issue #3545](https://github.com/iced-rs/iced/issues/3545).
[Iced CONTRIBUTING.md](https://github.com/iced-rs/iced/blob/b2fbd3cb6289603f8cbbb2387a5242231f509d13/CONTRIBUTING.md)
says contributors “must wait until someone from the core team chimes in”.
No user authentication or repository-permission intervention is currently needed.

Branch: [nimanoori22/iced:fix/tiny-skia-nested-canvas-clips](https://github.com/nimanoori22/iced/tree/fix/tiny-skia-nested-canvas-clips).
Base: `b2fbd3cb6289603f8cbbb2387a5242231f509d13` (current master at preparation).

Proposed title: **Fix nested canvas clipping in the tiny-skia backend**.

## Proposed PR body

Fixes iced-rs/iced#3545.

Software `Frame::paste` flattens child commands into their parent and loses the
child's canvas clip. A line or fill can escape its pane, and a larger nested child
can escape a smaller parent. Cached text and nested image clips have the same
parent-boundary problem.

Retain logical clips on fill/stroke records, intersect parent clips while merging
commands, and use the transformed intersection with render/damage bounds during
rasterization. Preserve clips on cached text and image commands too. Cached text
requires a glyph mask because layout bounds are not measured ink bounds. Stroke
damage bounds include width. The flat cache/command ownership and Canvas Frame
API remain unchanged.

The patch targets current upstream Iced only. It contains none of Feather's
separate image-ordering patches, chart code or application dependencies.

### Validation

- All seven final pixel regressions fail against original upstream source and
  pass with the correction: fills/wide strokes at offset/fractional origins and
  scale factors, nested/empty/disjoint clips, cached text, raster/SVG images,
  sibling masks and drawing order. Tests use retained geometry and actual software
  screenshots without a window or GPU.
- Geometry-only configuration: five tests pass.
- Geometry + raster/SVG configuration: seven tests pass.
- Affected-package all-target Clippy with warnings denied passes.
- Formatting and diff checks pass.

```sh
cargo test -p iced_tiny_skia --features geometry --lib
cargo test -p iced_tiny_skia --features geometry,image,svg --lib
cargo clippy -p iced_tiny_skia --features geometry,image,svg --all-targets -- -D warnings
rustfmt --edition 2024 --check tiny_skia/src/geometry.rs tiny_skia/src/primitive.rs tiny_skia/src/engine.rs tiny_skia/src/geometry/tests.rs
```

The full upstream workspace, other operating systems and a separate dense-text
mask-cost benchmark were not run. Repeated mask updates have a known cost (see
iced-rs/iced#3368); this patch uses the existing mask implementation and focuses
on preserving clipping correctness.

A downstream backport additionally passes 789 chart-library tests, 58 application
tests and native checks; these are integration evidence, not upstream workspace
results. The native minimal reproduction changes from 7,536 escaped pixels to
zero; the WGPU control also has zero.

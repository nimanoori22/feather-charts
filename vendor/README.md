# Iced canvas image ordering patches

Exact registry versions: iced_tiny_skia 0.14.1 and iced_wgpu 0.14.0, MIT licensed.
Sources copied without Cargo cache markers; LICENSE included from Iced.

Reason: native Canvas batches images after all vector geometry, violating the
built-in image watermark bottom layer. Plain chart geometry splitting cannot fix
this because the render layers also batch by primitive type.

TinySkia: canvas images become ordered Primitive::Image entries inside the same
cached primitive group, retaining the single software chart damage boundary.
TinySkia raster placement also stays in the floating-point transform rather than
truncating the scaled image origin to an integer pixmap offset.
WGPU: image boundaries split geometry into ordered phases submitted in clipped
layers. Cached phases reuse prior mesh caches. Image-absent geometry is unchanged.
No chart engine async or new primitive registry; all existing Frame draw APIs work.
The phase adapter preserves existing overlay-text semantics (fill_text stays an
overlay); chart text is already represented as glyph paths.

Regression: cargo run --example image_ordering [--wgpu]. Both unpatched backends
failed with red pixels over a later green rectangle. Native capture and default-off
comparisons are recorded with the image watermark port report. Remove these forks
when an upstream release provides equivalent ordered raster/vector canvas drawing.

image-ordering.patch uses zero context to avoid whitespace-only context lines;
apply with git apply --unidiff-zero when reconstructing the patch.

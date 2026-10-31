# feather-chart-file

Headless reader, writer and validator for version-1 `.fchart` JSON chart documents.
This crate has no Iced or chart-engine dependency. Rust benchmarks and scripts can
write portable numeric chart results for the native Feather Charts viewer.

```sh
cargo add feather-chart-file@0.1.0
cargo install feather-charts-app --version 0.1.0 --locked
feather-charts results.fchart
```

Use `read_document(Read)`, `ChartDocument::validate()` and
`write_document(Write, &ChartDocument)`. Both codec entry points validate semantic
constraints; writers validate before emitting bytes. Callers own paths and
create-new/overwrite policy. Export benchmark results after the measured section.

Version 1 supports one pane, one shared y-unit, multiple line/histogram series,
linear numeric x/y coordinates, embedded finite f64 points, and bounded metadata.
Histogram columns require positive `bar_width` in x units. X keys must increase
strictly within each series. Empty series are legal. Readers reject unknown
fields, unsupported versions, invalid numbers and unrepresentable extents.

Input limits: 32 MiB, 32 series, 250,000 points, 1 KiB title/label/name fields,
and 64 metadata entries with 4 KiB keys/values. These bound input, not peak RAM.
The file format version is independent of Cargo package versions.

See the bundled `fixtures/` and `examples/write_benchmark.rs`, and the
[format specification](https://github.com/nimanoori22/feather-charts/blob/v0.2.0/docs/fchart-format.md).
Licensed under Apache-2.0.

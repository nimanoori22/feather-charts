# Feather Chart document format, version 1

`.fchart` is UTF-8 JSON containing a complete single-pane chart, with no external
references, executable content, URLs, renderer state or plugins. See the committed
[minimal](../examples/fchart/minimal.fchart) and [mixed benchmark](../examples/fchart/benchmark.fchart) fixtures.

Required root fields: `format` = `"feather-chart"`, `version` = `1`, `title` (plain
text), `x_axis`, `y_axis`, `series`. Optional `metadata` defaults to an empty
string-to-string map. Unknown structural fields, versions and enum values are
errors. Metadata is the only open-ended extension area.

`x_axis` requires `label` and `spacing: "linear"`; `y_axis` requires `label`.
Both accept optional `unit` (default absent) and integer `precision` (default 0,
range 0..12). Labels and units are plain text. Both axes measure proportional
linear numeric distances. Points x=1,2,100 have horizontal gap ratio 1:98,
including through pan, zoom, Fit and resize. Series share the same x coordinates
and y unit; dates, categorical values and logarithmic axes are outside v1.

Each series requires nonblank `name`, `kind` (`line` or `histogram`) and `points`.
Optional `color` is exactly `#RRGGBB`; omission chooses a viewer palette color.
Each point is exactly `[x,y]`, two finite f64 JSON numbers. X must increase strictly
within each series after conversion to f64. Duplicate/decreasing keys are errors;
readers must never sort or drop points. Different series can have disjoint keys.
Empty series are valid, but at least one named series is required. An all-empty
document displays an explicit empty state. The default viewer displays series names and values in an in-chart readout;
series colors are used for plotting.

Histogram means a value-per-x column, not automatic statistical binning. It
requires positive finite `bar_width` in x-axis units, centered on every x.
Line series forbid `bar_width`. Edges must be finite, distinct from the center
and representable as f64. Fit includes edges and the zero y baseline.

Ranges must have representable finite subtraction, reciprocal (when nonzero),
and padding. Empty charts have no sample projection. Single-x charts fit a finite
relative extent. Numbers use finite f64 semantics, not arbitrary-size integers:
normalize large timestamps relative to a nearby origin; nanosecond epoch integers
can lose distinctness. Precision controls display only, never sample geometry.

Budgets: 32 MiB input (enforced while reading), 32 series, 250,000 total points,
1 KiB each for title, axis labels/units and series names; 64 metadata entries,
4 KiB for each key and value. These bound input, not peak RAM. JSON recursion
limits remain enabled. Writers validate all semantic fields and the byte budget
before emitting any bytes. I/O failures are reported; stream failures may leave a
partial file. Producer examples create new files and remove only their own failed
partial output. They never overwrite existing results.

Public Rust API lives in `feather-chart-file`: public value structs, `validate`,
`read_document(Read)` and `write_document(Write, &ChartDocument)`. No engine or Iced
dependency. Serde serialization alone does not validate semantic constraints.
Accepted v1 fixtures remain readable throughout v1; breaking changes require a new
format version independently of Cargo package versions.

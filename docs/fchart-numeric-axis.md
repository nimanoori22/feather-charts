# Continuous numeric axis design

The existing `TimeScale` owns points, viewport and interaction state. Its projection
is consumed by line/area/baseline/bar/candle/histogram views, markers, primitives,
public coordinate queries and frame preparation. Visible ranges feed row searches
and vertical autoscale. Crosshair placement and its label are separate consumers.
Graph trace of `index_to_coordinate` found 44 callers through depth two; direct
source reads supplement graph method edges. Exact-path coverage reported no gaps
for the 13 initial evidence files (best effort, not proof of completeness).

Two viable designs: a dedicated numeric chart owner sharing renderers, or an
explicit numeric viewport in the shared scale. Choose the latter: it preserves
existing row storage, scene preparation, widget events and screenshot drawing.
A dedicated owner would duplicate lifecycle and host integration. Numeric mode
is opt-in; calendar, strike, maturity and custom defaults keep their old projection.

Ownership stays Chart -> model -> TimeScale -> owned points + numeric viewport.
There are no new callbacks, strong back-references, cycles, locks or async engine
operations. The viewer owns its document and chart; background parsing returns a
candidate document, and successful construction replaces both atomically.

Public API: `OptionsChart::new_numeric(precision: u8) -> Result<Self, ChartError>`;
`Chart::numeric_to_coordinate(f64) -> Option<f64>` and inverse;
`Chart::set_numeric_range(from: f64, to: f64) -> Result<(), ChartError>`;
`Chart::set_numeric_histogram_width(SeriesId, f64) -> Result<(), ChartError>`.
Row indices identify original samples. Numeric conversion is affine in the
original x space. Visible row bounds come from binary searches with neighboring
rows retained for segments and columns. No gap filling or index aggregation.

Fit pads the data extent by 5%; histogram edges expand that extent. Empty charts
have no projection; single points use a finite relative fallback extent. Reject
nonfinite, reversed, overflowing or unrepresentably narrow ranges. Zoom preserves
the cursor's numeric value, pan translates the extent, resize preserves the extent.
Numeric ticks use 1/2/5 steps independent of sample locations. Crosshair x and
label use the continuous cursor coordinate; selected row identity stays separate.
Nearest sample ties choose the lower x; per-series hit tests use prepared pixels.
Histogram columns project both numeric edges and include zero in y autoscale.

Regression gates: actual prepared line coordinates for 1,2,100 (1:98), shared
coordinates across disjoint series, pan/zoom/resize, inverse conversion, ticks,
crosshair, column rectangles and picking. Both renderer backends consume the same
prepared geometry. Existing domain regressions remain required.

Numeric mode disables index-based conflation and kinetic inertia. Direct drag,
wheel/pinch zoom and axis scale gestures use numeric ranges; horizontal reset
restores Fit. `set_visible_horizontal_range` uses exact numeric endpoints, while
logical range setters resolve row-index endpoints (fractional endpoints interpolate
between stored keys). Index spacing/right-offset options remain legacy controls;
use `set_numeric_range` for explicit numeric viewport control.

# Synchronized pane rendering

## Source investigation and contract

Read ChartWidget sizing/synchronization consumers, complete PaneSeparator,
CrosshairPaneView and CrosshairRenderer, Crosshair pane/axis view consumers,
PaneWidget pointer/scroll handlers and existing Rust pane/removal/axis code.
Graph traces connect chart synchronization -> sizing/separator construction,
and Crosshair -> per-pane view. Coverage checks reported no recorded gaps;
this remains best-effort and direct reads establish the behavior used here.
Local Iced Canvas Program exposes captured actions and read-only drawing;
Flowsurface dashboard uses explicit resize messages/state updates. Its PaneGrid
is for independent dashboard panels; the chart instead needs globally aligned
axes/time coordinates, so we keep the existing single Canvas approach.

Required behavior: shared maximum left/right widths, one bottom time axis,
weighted pane heights, source round-to-device-pixel for intermediate panes and
ceil remainder for the last, minimum 2px layout panes and 1px separators. Divider
handles extend 4px on either side; resizing preserves adjacent total stretch and
clamps both panes to 30px, refusing pairs already too small. Window resizing
retains stretch proportions. Vertical crosshair appears in every pane; horizontal
line/price labels belong to the origin pane; the time label is chart-wide.
Series removal may delete an empty pane and shift later indexes; callers rebuild
pane selections from model membership rather than retaining stable-ID assumptions.

The existing Rust navigation contract (wheel coalescing, 5px drag threshold,
focus/layout cancellation) stays intact. This milestone shares horizontal time
navigation; source vertical price dragging, touch and kinetic scroll are separate
unported capabilities, not silently introduced here.

## Alternatives and design

1. Extend ChartFrameOwner into a monolithic single/multi-pane assembler.
2. Add MultiPaneChartFrameOwner coordinating pane preparation through the existing
   borrowed dispatch and owned chart/plot snapshots, retaining one global cycle.

Choose 2 to preserve tested single-pane callers and performance. It is a chart
coordinator, not independent competing pane frame owners. It owns allocation and
retained-mask state; no pane preparer drains invalidation or advances animation.
The limited duplication of settlement orchestration avoids destabilizing the
existing path; geometry, axes, measurement, commands and drawing stay shared.

Public types: `SeriesPaneView` (owned Line/Candlesticks/Histogram variants),
`MultiPaneChartFrameInput` (one optional SeriesId per model pane, bounds/ratio/
background), `MultiPaneChartFrameSnapshot`, `IcedMultiPaneChartFrame`, and
`MultiPaneChartFrameOwner::prepare`. `allocate_multi_pane_layout` takes positive
stretch factors and returns shared chart regions plus per-pane rectangles.
The known one-selected-series-per-pane limitation matches the current single
frame API; generalized multi-series composition awaits the public widget facade.

```text
Application -> strong owned model/controller/coordinator/Vec<SeriesPaneView>
Coordinator borrows model + views -> owned vector of pane frame snapshots
Canvas borrows immutable snapshots -> typed pane/divider actions
Application updates model synchronously -> prepares next chart frame
```

No back-references, Rc cycles, async machinery or manual destruction. Views and
snapshots drop normally. Input owns an active plot gesture or divider gesture,
never both; pane membership/layout changes cancel obsolete gestures. Preparation
validates pane count/selections before changing dimensions or consuming commands.

## Verification and remaining dependencies

Pending: exact changed files, source-oracle/layout/input/frame/lifecycle tests,
both raster backends, performance measurements and physical input acceptance.
Public API facade and arbitrary pane reordering remain subsequent work.

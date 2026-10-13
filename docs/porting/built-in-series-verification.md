# Built-in series integrated verification

2026-10-13. The Area -> Bar -> Baseline roadmap is implemented. All six built-in
families now work through Chart, with immutable single/composed pane rendering,
live/historical data, options/visibility, price levels/labels and pointer events.
Fresh physical acceptance remains pending; automated checks do not close it.

## Source and behavioral checks

Reference: local Lightweight Charts revision
`8a7c94ff8ddffde68955db61ef8b58098f239de0`. Family contracts, ownership, alternatives,
files and limits are recorded in [Area](area-rendering.md), [Bar](bar-rendering.md)
and [Baseline](baseline-rendering.md). Source graph coverage was checked for
relied-on paths; changed metadata was supplemented with source reads.

Every `scripts/check_*_reference.cjs` passes: **29,235 cases** in total.
New cases: Area 288, Bar 7,776, Baseline 1,656. Existing line, candle, histogram,
axis/layout, price-line, navigation and multi-pane comparisons remain green.
These execute actual TypeScript methods, comparing geometry, style boundaries,
ordered gradient stops, dash phase, rectangles and source model behavior.

Pointer conversion follows `api/get-series-data-creator.ts`: _convertMouseParams
and SeriesApi data/dataByIndex/pop consume original times, exact rows and their
optional styles. The Rust shared click/crosshair converter now returns Area, Bar
and Baseline, including unit custom values, rather than omitting these types.
A mixed-series test checks both callback kinds, retained frame ownership and
clearing. A whitespace test confirms no nearest-row substitution in callbacks.
Magnet uses existing fulfilled plot values/price scales without series-kind gates.

## Visual and native checks

| Family | TinySkia | WGPU | Ratios |
| --- | ---: | ---: | --- |
| Area | 48 captures | 48 captures | 1, 1.25, 1.5, 2 |
| Bar | 44 captures | 44 captures | 1, 1.25, 1.5, 2 |
| Baseline | 60 captures | 60 captures | 1, 1.25, 1.5, 2 |

**304 repeatable captures**. Scenarios include independent row styles, curves,
steps, relative/inverted fills, thin/thick/compressed Bar stems, hidden open ticks,
Baseline base outside the pane, gradient markers, empty and single data, live
updates, crowded labels and left/right price scales. Representative Area, Bar,
Baseline, curves and markers were inspected. Each new native example ran ten
seconds without error and was intentionally terminated by timeout.
Headless public smoke passes for all six selectable example kinds and controls.
The compressed/thick Bar captures drain the initial fit-content request before
setting spacing. They assert spacing < 1 or a visibly thick stem respectively;
both backend runs were repeated after these fixture checks.
A ratio matrix of styled Area/Baseline and compressed Bar was also inspected.

Iced gradient interpolation/precision differs from browser Canvas (WGPU uses
smoothstep and half precision offsets). Command parity does not claim identical
raster pixels. Equal endpoints paint transparently using a finite backend gradient;
this also avoids upstream's NaN-offset failure when a degenerate relative gradient
coincides with its base. Hardware checks must still cover zoom/pan/axis scaling,
crosshair, live updates, style/base changes and crowded labels.

## Performance comparison

`examples/public_api_profile.rs` runs identical public API workloads against
pre-roadmap `706a695` and the implementation: 1,200 initial rows, 120 updates,
20 warm-up samples, 100 measured samples; 917 x 870.80005 logical size, ratio 1.25.
Timing includes model, owned frame/measurement and Iced glyph preparation, but
excludes GPU drawing and physical input latency. No dependency changes.

Two sequential alternating runs after an initial warm-up comparison:

| Workload | Before median ms | After median ms | Median change |
| --- | ---: | ---: | ---: |
| Line zoom | 6.693 | 6.641 | -0.8% |
| Line append | 11.052 | 10.976 | -0.7% |
| Candlestick zoom | 10.291 | 10.470 | +1.7% |
| Candlestick append | 14.456 | 14.787 | +2.3% |
| Histogram zoom | 7.960 | 7.999 | +0.5% |
| Histogram append | 12.136 | 12.298 | +1.3% |

P95 ranges across those runs: line zoom 6.937–6.951 -> 6.815–7.038 ms;
line append 11.770–11.886 -> 12.016–12.300 ms;
candle zoom 10.410–10.819 -> 11.045–11.569 ms;
candle append 15.125–15.361 -> 15.598–16.479 ms;
histogram zoom 8.459–8.602 -> 8.497–8.850 ms;
histogram append 12.616–13.390 -> 13.094–13.184 ms.
Medians are close; candle tail samples are somewhat higher. Review found no new
per-row geometry work on the existing candle path: added fields are empty Vecs,
and new-family preparation is dispatched only for its view. Local debug timing
and CPU contention limit interpretation; this is not a frame-rate guarantee.
The initial run had much larger before-version outliers and is excluded from this
table. Fresh physical acceptance is required before calling interaction finished.
Area now prepares fills without redundantly generating its outline a second time.
A source marker-rounding boundary check also caught Rust's different rounding of
negative half pixels; the shared Line/Area marker conversion now uses source JS
rounding, verified by a regression assertion.

## Final checks

Final checks pass: `cargo fmt --check`, `cargo test --all-targets` (**421 tests**:
336 library, 21 public API/family and 64 demo tests), `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --examples`, and
`git diff --check`. No skipped tests or suppressed
warnings; two legacy Bar fixtures now use valid default colors while retaining
all original autoscale assertions. Code review covered correctness, ownership,
validation, hot paths and source consumer fit. Changes saved in buildable local
commits; publishing is a separate user action.

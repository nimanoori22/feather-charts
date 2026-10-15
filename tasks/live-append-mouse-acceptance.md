# Live-append mouse acceptance

Available device confirmed by the user: **mouse only**. Touchpad and touchscreen
are unavailable for this session, not passed. The rows below require observed
hands-on results; scripted events and screenshots do not complete them.

Use the release build after automated timing work finishes. Start the GPU session:

```sh
ICED_BACKEND=wgpu cargo run --offline --release --example public_chart -- --histogram --primitives --live-feed
```

For software rendering, close that window and repeat with `ICED_BACKEND=tiny-skia`.
The explicit environment variable selects the native backend; the example's
`--wgpu` switch selects its capture backend and does not configure its native host.

For each backend, test at regular size, then resize the window to roughly half
width/height and repeat. Keep the live feed enabled throughout:

1. Wheel over the plot in both directions. Check zoom response and pointer anchor.
2. Hold the left button and drag diagonally. Check both axes and release behavior.
3. Wheel over the price axis, then drag the axis vertically. Check vertical scale
   response; double-click to reset.
4. Move and resize the box using its handles. Commit, cancel another edit, then
   start a fresh gesture. Check that updates do not interrupt dragging.
5. Move across the plot, axes and window edges. Check that interaction capture
   ends correctly and the chart bottom stays stable.

| Backend / size | Zoom | Pan | Price axis | Box editing | Layout/capture |
| --- | --- | --- | --- | --- | --- |
| wgpu / regular | Pass | Pass | Pass | Pass | Pass |
| wgpu / half size | Pass | Pass | Pass | Pass | Pass |
| tiny-skia / regular | Unresolved* | Unresolved* | Unresolved* | Unresolved* | Unresolved* |
| tiny-skia / half size | Unresolved* | Unresolved* | Unresolved* | Unresolved* | Unresolved* |

Record backend, approximate size, device/monitor scale, pass/fail and a concrete
reproduction for any failure. High-DPI physical coverage remains pending unless
an appropriate display/scale is actually tested. These checks assess feel and
behavior; the native presentation profiler separately measures latency. No
input-to-photon measurement is claimed without an external measurement setup.

GPU result: the user reported “All checks passed at both sizes” on 2026-10-15,
using the mouse in the live-feed Histogram/box scene.

Software failure: with the live feed running at approximately one-third of the
regular window size, dragging the plot or vertical price axis becomes slightly
laggy. The user reproduced this in the subsequent debugger session. Other
software gestures/sizes are not individually accepted. The measured fix and retest
are recorded below.
The 60-stack diagnostic contains 19 software primitive drawing samples, 35 event
waits, three text samples and three other samples. Debugger pauses perturb timing;
this is hotspot evidence, not acceptance latency.

*The failing software case is now identified above; no software cell is accepted.
High-DPI and input-to-photon acceptance remain untested.

## Software diagnostic retest after canvas damage fix

On 2026-10-15 the user repeated plot dragging and vertical price-axis dragging
with the live feed running at the same problem size and reported:
**“Smooth now; window closed.”** The trace records a 941×1030 physical window,
941×900.8 chart and scale 1. Render/present CPU p95 fell from 117.512 to
11.179 ms; all 1,049 after submissions resolved with zero discards.

This accepts the observed feel of these two diagnostic gestures at that size.
It does not fill the unresolved normal-build regular/half-size table above.
Full software zoom, box edit/cancel, capture/release and short-height recovery
checks still need individual results. See the [implementation report](../docs/porting/live-update-interaction-acceptance.md).

# Invalidation accumulator

Source: `lightweight-charts/src/model/invalidate-mask.ts`.

## Consumer contract

`ChartModel` produces masks for redraws, pane autoscaling, and deferred horizontal
scale operations. `ChartWidget._invalidateHandler` merges them before a frame.
`ChartWidget._drawImpl` can merge and replay commands after layout changes;
therefore reading commands must not consume them. Pane and axis widgets use
`None < Cursor < Light < Full` to choose the amount of rendering work.

Global and pane-specific redraw levels merge by maximum. Effective pane levels
include the global level. Autoscale requests merge by OR, are local to the pane,
and request momentary autoscaling without changing its persistent options.
Missing pane entries inherit global redraw priority without requesting autoscale.

Time-scale command setters do not raise redraw priority; their producers request
light/full invalidation separately. They store payloads without adding numeric
validation: validation remains the responsibility of the existing scale/API.

| Operation | Pending command normalization |
| --- | --- |
| Fit, range, reset | Replace the whole sequence with that operation |
| Spacing, offset | Remove animation; append stop, then the operation |
| Animation | Remove previous animation; append replacement |
| Direct stop | Remove animation; append stop marker |
| Stop replayed during merge | Remove animation without appending a marker |

Merge replays incoming commands in order using these rules, then combines redraw
and pane requests. It is directional and does not consume the incoming mask.
Stop markers are not time-scale mutations in the original frame consumer.
Their effect is to remove animation continuation from pending requests.

## Rust representation and API

`InvalidateMask` owns a `BTreeMap<usize, PaneInvalidation>` and a normalized
`Vec<TimeScaleInvalidation>`. Its collections are private. `merge(&Self)` borrows
an incoming mask; `time_scale_invalidations()` returns a borrowed slice.
`PaneInvalidation.auto_scale` is a boolean because consumers do not distinguish
false from absent. `LogicalRange` is reused from the existing model.

An alternative was a raw request log normalized at frame time. The owned,
normalized mask supports immediate pane queries and layout replay with less
consumer complexity. The sparse pane map allocates only for requested panes;
the command vector retains ordering. No async or thread-safe machinery is used.

## Animation ownership and handoff

The source has two real animation producers: linear scrolling in `TimeScale`
and kinetic scrolling in `PaneWidget`. A linear-only descriptor cannot represent
both. `TimeScaleAnimation` provides immutable, synchronous sampling with caller
supplied `Instant`; `AnimationRequest` owns it behind an `Rc`. Sampling returns
an offset and completion flag. Implementations must retain their time origin and
must not capture the chart owner. No callback-identity API is exposed.

```text
Chart/UI owner
  +-- owns pending mask -- strong --> immutable animation sampler
  +-- owns UI controller -- strong --> same sampler
```

The existing `ActiveTimeScaleAnimation` implements the sampler and reuses
`ScrollAnimation` to create linear animation state. The UI establishes its start
time once with `ActiveTimeScaleAnimation::from_effect(descriptor, now)`, wraps it
with `AnimationRequest::new`, and queues it with `mask.set_animation(request)`.

For layout or frame replay, pass the retained request to
`TimeScaleAnimationController::apply_animation(request.clone())`. Do not convert
it back into a new `StartScrollAnimation` effect: `apply_effects` intentionally
starts a new animation. The controller's existing frame/completion behavior is
preserved; a zero-duration descriptor completes immediately at its target.

An `Rc` clone does not restart animation. Dropping a mask or controller releases
its reference; the sampler drops when the last request drops. The library sampler
holds only offsets and timing, so it has no back-reference or cycle. Custom trait
implementations must follow the same ownership rule. No destruction order is
required.

Cancellation normalization in the mask and cancellation of an already installed
UI controller are distinct. The future frame owner must honor the final normalized
mask, cancel superseded controller state, and carry forward only surviving
unfinished animation requests. `controller.cancel()` is already available.

## Verification and remaining work

Tests cover redraw priority pairs, autoscale OR combinations, sparse pane fallback,
command replacement and ordering, directional merge, direct/merged stop behavior,
animation replacement, reusable incoming masks, layout replay, preserved animation
progress, reference lifetime, and UI cancellation/completion.

The chart-model core now maps model effects to masks and executes normalized scale
commands. Its UI animation frame driver handles continuation and cancellation;
see [chart-model.md](chart-model.md). Recalculate-all-panes effects remain model
work. The kinetic-animation algorithm, pane/crosshair rendering, and Iced frame
scheduling are separate subsequent ports; this sampler boundary can support
kinetic animation without changing the mask representation.

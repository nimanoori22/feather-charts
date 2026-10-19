# I — configured color policy and sRGB output

Baseline: `3eb01c7`. Reference: local Lightweight Charts 5.2.1,
`8a7c94ff8ddffde68955db61ef8b58098f239de0`.

## Source and consumer contract

The complete `src/model/colors.ts` and its unit tests were inspected. The model
constructs one ColorParser from layout.colorParsers. Successful original-string
lookups are cached; native/browser sRGB parsing precedes ordered custom fallback;
first Some wins. Failed colors are not cached. A further source probe reproduced
the same fallback for color(srgb/srgb-linear), lab/lch and oklab/oklch; these must
reach configured callbacks even when the native CSS library can convert them.
HSL, HWB and hex remain native-first. applyAlpha multiplies existing
alpha, with a transparent shortcut; contrast ignores alpha and uses grayscale
0.199R + 0.687G + 0.114B, choosing black only above 160. Gradients normalize
channels and alpha. Cache/ordering fixtures must pin actual callback invocations.

Real consumers: ChartModel background gradients; PriceScale shared contrast;
series/current/custom-line and crosshair axis labels; last-price pulse alpha.
Browser series/extension renderers also accept CSS colors directly. Native Iced
must resolve strings before drawing, so native layout, all six series and row
colors, custom-series labels, markers, lines, axes and attribution need the same
configured policy. Opaque application renderers already receive/produce native
Iced colors; they must explicitly use the shared resolver during preparation.
A chart cannot intercept arbitrary application Canvas drawing.

## Designs and ownership

Chosen: make existing ColorParser clones share Rc-owned callbacks and a cache.
Borrow the cache only for lookup/insertion, never while invoking user callbacks.
Keep the existing Box callback API. At chart construction, move callbacks into
shared parser storage and put callable forwarding boxes back into layout options;
readback remains callable without requiring Clone on application closures.
ChartModel and series/preparers own cheap parser clones, with no reference back to
the chart. Prepared frames own resolved colors/geometry, not parsers or charts.
The public Chart::color_parser() clone can outlive the chart; dropping its final
clone releases callbacks/cache. There is no owner cycle or async/threading.

Alternative: thread borrowed callback slices through every validator/preparer
and recreate contextual parsers. This avoids shared ownership but splits caches,
repeats callbacks and complicates DataLayer/custom/retained handle boundaries.
Canonicalizing stored option/data strings was also rejected: applications must
retain original color inputs in readbacks and metadata. Resolution happens at
validation/preparation, not by rewriting source data.

Public additions: cloneable ColorParser; Chart::color_parser() -> ColorParser;
TextWatermark::with_color_parser(options, ColorParser) -> Result<Self, ...>;
PrimitiveContext::color_parser(), DrawingPrimitive::validate_colors(), and
CustomSeriesPaneView::{validate_options,update_with_color_parser} for native custom renderers. Standalone constructors
continue to use ordinary CSS parsing. Generic primitive native ink remains
application-owned; primitive labels and chart-owned extensions use chart policy.
No runtime parser replacement is offered: callbacks/cache are fixed at creation.

## Output target and useful errors

Only sRGB native rendering/export is supported. Parsing a Display-P3 string
through a callback returns the application's chosen sRGB Rgba; it does not enable
Display-P3 output. Add a dedicated UnsupportedColorSpace(DisplayP3) model error.
Checked calendar, numeric, yield and custom-domain constructors reject P3 targets;
legacy infallible calendar construction remains available but prepare/prepare_snapshot
return that error before rendering. Runtime layout target patches reject P3
atomically, preserving options/gestures/old exports. Ordinary sRGB defaults remain.
Custom callback Rgba must have finite alpha in [0,1]; invalid values fail before
caching or committing input. CSS rendering retains existing float component
precision; custom Rgba uses its explicit 8-bit sRGB channel contract.

Unsupported native CSS still fails explicitly; browser invalid CSS may fall back
to inherited black. This existing safer native behavior is intentional, not a
claim of exact invalid-CSS browser parity. Browser DOM color probing and reference
identity/manual cleanup are incidental; the synchronous Rust owner is retained.

## Verification sequence

Freeze a real browser source oracle and native red parser-invocation regression.
Implement parser sharing/core validation first, then series/frame paths, then
lines/labels and extension boundaries, then target errors. Test alpha/contrast,
parser ordering/cache/reentrancy/drop, invalid input atomicity, original readbacks,
all six series and row colors, custom series and retained exports. Reuse current
public demo and capture helpers. Freeze four-domain, three-size (800x500,
300x300,300x90), four-DPR (1,1.25,1.5,2), both asserted-renderer captures before
implementing demo scenes. Run all common gates, all 46 source comparisons, affected
unchanged compatibility protocols, and normal software/GPU mouse acceptance.
No gamut hardware acceptance or earlier independent exception is inferred.


## Native extension boundary

Chart-owned raw strings (including primitive price/time labels) are resolved during
validation/preparation. Opaque custom series and primitive renderers draw native
Iced ink and cannot be intercepted. Custom views can validate their application
options before attachment/replacement and resolve ink in update_with_color_parser.
Primitives can validate declared color parameters before attached() or replacement
and use PrimitiveContext::color_parser while preparing owned geometry. Existing
implementations retain default hooks, so native-color consumers need no changes.
Context-dependent label output is validated at frame preparation; failures clear
displayed geometry rather than partially displaying invalid paint. This cannot
promise static validation of arbitrary context-dependent application code.

Low-level engine Series constructors preserve their existing validation contract;
public Chart series add/replace methods additionally validate the active palette.
Candlestick border_color/wick_color are aliases: final up/down colors are rendered
and validated. Inactive alias strings are not parsed as independent paint.
Row validation uses fixed stack arrays, adding no per-row palette allocation.
The native rendering adapter preserves CSS float precision; no parser runs in
immutable Canvas drawing or owned screenshot replay.


Migration: callers using color(), lab()/lch() or oklab()/oklch() should supply
layout.color_parsers returning their chosen sRGB Rgba. The source browser leaves
these functional forms unconverted, so bypassing callbacks with a native library
conversion was incorrect. Such callbacks do not configure an output gamut.
The previous independent csscolorparser acceptance of some of these forms is no
longer used as an implicit conversion behind configured callbacks.


Hosts converting raw snapshots can pass Chart::color_parser() to the public
from_snapshot_with_colors/from_commands_with_colors UI adapters. Their original
conversion methods retain default CSS behavior. Raw snapshots preserve authored
commands; converted native frames own resolved ink and retain no parser handle.
An external-consumer integration test covers this conversion boundary.


Checked native APIs validate inputs eagerly. Callback results, per-color fallback
ordering and successful caching follow the source; exact browser frame scheduling
of side effects is not a callback guarantee. Parser callbacks should be deterministic.
Library handles have no owner back-reference. Application closures can still create
an Rc cycle by strongly capturing their chart container; use Weak or separate state.
Different-key resolver re-entry is tested; recursively resolving the same missing
key remains the application's responsibility, as in the source.
The exact Iced `core/src/color.rs` documents its native Color as sRGB. The new
adapters pass resolved f32 components to existing Color/Canvas APIs and add no
window/device/thread setup. Source graph coverage was checked; stale source-color
metadata was supplemented with direct complete reads before implementation.

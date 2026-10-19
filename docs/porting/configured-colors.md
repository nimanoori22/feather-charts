# I — configured color policy and sRGB output

Baseline: `3eb01c7`. Reference: local Lightweight Charts 5.2.1,
`8a7c94ff8ddffde68955db61ef8b58098f239de0`.

## Source and consumer contract

The complete `src/model/colors.ts` and its unit tests were inspected. The model
constructs one ColorParser from layout.colorParsers. Successful original-string
lookups are cached; native/browser sRGB parsing precedes ordered custom fallback;
first Some wins. Failed colors are not cached. applyAlpha multiplies existing
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
preparation contexts/helpers for native custom renderers. Standalone constructors
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

# Public parity audit — local Lightweight Charts 5.2.1

Audited 2026-10-18 against native checkpoint `c08baeb` (runtime implementation
`e037a3b`) and local TypeScript HEAD `8a7c94ff8ddffde68955db61ef8b58098f239de0`.
The source worktree contains existing changes; exact consumed file hashes are
retained in [the evidence](public-parity-evidence.json). This is a comparison with
that local 5.2.1 tree, not a claim about a newer upstream release.

The major feature roadmap is implemented, but exact public behavior is incomplete.
This audit identifies **ten actionable finding groups**, six supported by executable
source/native comparisons and four by interface/ownership inspection. No engine,
rendering or interaction implementation was changed. The previous “95% complete”
statement remains an informal feature estimate; these counts are not a completion
percentage or an assertion that every remaining defect has been found.

Implementation follow-up: the baseline findings and diagnostic evidence below
remain a record of the audited revisions. Formatting core/runtime fixes are now
tracked in [formatting-parity.md](formatting-parity.md), with separate red/green
regressions and a 72-case source comparison. The generated inventory now reflects
those API mappings; its original baseline is retained at `55ba7d7` and pinned by
[the implementation manifest](public-parity-implementation-baseline.json). Formatting
automated renderer checks pass; mouse acceptance is pending. Other finding groups remain open.

Scale-assignment follow-up: [scale-assignment.md](scale-assignment.md) records the
F03 implementation and the fresh series-to-scale query. Its eight behavioral tests
and 20-case source comparison pass; renderer/mouse acceptance remains milestone B3.
Baseline findings/evidence below continue to describe the original audited revision.

## Inventory and evidence standard

[public-parity-inventory.json](public-parity-inventory.json) inventories **55
interfaces and 374 declared members**, including **all 99 declared methods** on
IChartApiBase, IPaneApi, IPriceLine, IPriceScaleApi, ISeriesApi, ITimeScaleApi and
IYieldCurveChartApi. It also covers the chart/scale/layout/grid/crosshair/input/
localization/series/price-format/plugin option families, built-in data structures
and pointer payloads. Base interfaces are recorded separately; inherited members
are not expanded or counted twice. This is a bounded public-contract inventory,
not every exported alias, internal interface or possible option combination.

Each row contains the original signature, current source path/line, native mapping
and classification. `mapped` means an implementation entry point was found;
`represented` means the corresponding real Rust field exists. Neither means all
behavior is proven. `native` records a Rust ownership/API adaptation; `platform`
records a DOM/host distinction. `partial` and `gap` identify the findings below.
A differently spelled Rust method is not by itself missing functionality.

Graph discovery and bidirectional traces located the contracts and consumers.
Coverage was checked for 44 source and 58 native paths and the relevant scopes.
All native paths had matching metadata; several source paths had changed metadata.
Their current definitions were read directly with the TypeScript AST scanner, and
relevant implementation bodies were reread/executed. Some graph method edges were
absent or resolver candidates were inaccurate; current source and executable probes
are authoritative. Clean graph coverage is not exhaustive proof.

The probe reports **12 observations: 11 differences across six groups, plus one
matching German month-label control**. They were deliberately chosen to investigate
suspected gaps; **11/12 is not the failure rate of the whole port**. Both actual
source mouse gestures and native Iced Canvas events were driven synthetically.
No new physical-device or rendering-performance acceptance is claimed.

## Findings, in recommended implementation order

### F01 — Chart-wide price localization is not connected to native scales (P1)

**Reproduced.** Supply `LocalizationOptions.base.price_formatter`,
`tickmarks_price_formatter` and `percentage_formatter` through
`Chart::with_custom_behavior(HorzScaleBehaviorTime::default, ...)`, create a Line
series and prepare it. Source global labels are `GLOBAL`, `GLOBAL_TICKS` and
`PERCENT`; native labels remain `20.00`, `20.00` and `100.00%`.

Source `src/model/price-scale.ts` `_formatPrice`, `_formatTickmarks`,
`_formatPercentage`, `_formatPercentageTickmarks` consult chart localization.
Native [price_scale.rs](../../src/model/price_scale.rs) `format_price`,
`format_logical_tickmarks`, `format_absolute` use default/source formatters without
chart localization. Native `LocalizationOptionsBase` declares all four callbacks,
but the generic chart does not route them into scale formatting. The YieldCurve
wrapper has its own price-formatter handling; that does not complete the generic
contract or the batch/percentage cases.

There is also no general localization patch in `ChartOptionsPatch` or
`ChartModelOptionsPatch`. Engine `TimeScale::set_localization_options` needs mutable
access that `Chart::model()` does not expose. Behavior-specific configuration and
maturity/strike format setters cover selected cases; they are not a full chart-wide
runtime localization operation.

**Next change:** connect owned localization to price scales and all label/tick paths;
add an atomic runtime localization update, invalidate dependent formatting caches,
and preserve native no-op/gesture/event semantics. Test normal/log/percentage/indexed
scales, multiple panes, custom series and formatter fallback/clearing/precedence.

### F02 — Custom batch tick-mark formatters are ignored (P1)

**Reproduced.** Set `PriceFormat::Custom` with a single-value formatter returning
`SINGLE` and a batch formatter returning `BATCH`. Source
`series.priceFormatter().formatTickmarks([20])` returns `["BATCH"]` and invokes the
batch callback. Native returns `["SINGLE"]` without invoking it.

Source `Series._recreateFormatter` preserves both callbacks. Native
[series.rs](../../src/model/series.rs) `SeriesValueFormatter::Custom` stores only
the single-value callback, and `formatter_for` discards `tickmarks_formatter`.
Native scale `format_logical_tickmarks` separately maps single-value formatting;
[PaneSeriesPriceSource](../../src/model/pane.rs) also has only a scalar formatting
bridge. Fixing the public formatter alone would leave axis ticks incomplete.

**Next change:** carry the batch operation through series, projection/source and
scale formatting with source fallback/precedence. Include custom-series formatters,
not just the built-in Series implementation. Test batches where labels depend on
neighboring values and verify actual axis snapshots.

### F03 — Changing a series price-scale ID does not relocate it (P1)

**Reproduced.** Start a populated Line on the right scale; replace its options with
`common.price_scale_id = Some("left")`. Source `series.applyOptions({priceScaleId:
'left'})` moves it left. Native still populates the right scale and leaves the left
scale empty, although the stored option says left.

Source `Series.applyOptions` calls `model.moveSeriesToScale` before merging options.
Native [ChartModel::replace_series_options](../../src/model/chart_model.rs) only
replaces the series options and refreshes formatter sources. The lower-level
[Pane](../../src/model/pane.rs) has move-to-scale operations, but the reusable
Chart owner does not expose a corresponding operation or route this option.
Built-in and custom option replacement need the same ownership rule. Construction
has an explicit Rust `PriceScalePosition` argument; using that argument is a valid
native API choice, but silently storing contradictory runtime state is not parity.

**Next change:** implement a single validated owner operation for moving a series
between left/right/named overlay scales; route supported option changes through it.
Preserve data, series ID, attached annotations, source order, scale cleanup/defaults
and active-gesture cancellation. Test destination creation, empty-scale cleanup,
custom series, pane transfers and failed operations without partial mutation.

### F04 — Horizontal-axis gestures and independent axis controls are missing (P2)

**Reproduced.** With 20 fitted Line bars, drag the horizontal axis 100px horizontally.
The real JS widget changes bar spacing; the Rust widget does not. Double-clicking
the axis resets source spacing to its configured default; native spacing stays at
the fitted value. Source `gui/time-axis-widget.ts` implements mouse/touch scale
gestures and reset, guarded by `handleScale.axisPressedMouseMove.time` and
`axisDoubleClickReset.time`.

Native [ChartProgram](../../src/api/widget.rs) routes plot input and
[PriceAxisInput](../../src/ui/price_axis_input.rs), but no horizontal-axis input
controller. `MouseNavigationOptions` covers plot wheel zoom/scroll and dragging;
`TouchNavigationOptions` covers touch drag/pinch/tracking. There are no independent
price/time axis drag/reset settings equivalent to the source nested options.
Existing price-axis gestures work, including the native wheel extension, but an
application cannot independently disable axis drag or double-click reset.

**Next change:** add synchronous horizontal-axis gesture ownership and typed axis
interaction settings. Match source anchor/default/reset behavior. Verify priority,
empty/hidden axes, release outside, focus loss, pane changes and touch double-tap.

### F05 — Configured color parsers and wide-gamut rendering are incomplete (P2)

**Reproduced parser gap.** Configure `layout.color_parsers` for valid
`color(display-p3 1 0 0)` and use it as text color. The source actually invokes the
custom parser and returns the expected grayscale. The validated Rust constructor
rejects the chart despite the configured parser.

Native [LayoutOptions](../../src/model/layout_options.rs) stores parsers, and
[ColorParser](../../src/model/colors.rs) supports custom parsers on its own, but
chart validators/preparers construct default parsers without those callbacks.
Routing must cover layout, series/row colors, lines, primitives, watermarks and
contrast/alpha transformations. A hand-built ColorParser working in isolation is
not a working chart-level option.

`ColorSpace::DisplayP3` is also stored and copied into axis snapshots. The inspected
native draw/export paths use sRGB/iced Color; this audit found no wide-gamut target
configuration or conversion. **No hardware gamut test was run.** Treat the rendering
contract as incomplete, not as proven Display-P3 support merely because the enum
exists. Screenshot output deliberately documents sRGB.

**Next change:** route one configured parser policy consistently. Decide and document
whether wide gamut is supported end-to-end or explicitly rejected at the public
boundary; retaining a successful inert option is insufficient. Keep export color
space explicit and test custom parser fallback/error/caching behavior.

### F06 — Reusable scale/query APIs are incomplete (P2, static contract finding)

The inventory separates missing owner methods from available read-only helpers.
Applications can query many engine fields through `Chart::model()`, perform item
conversion and compose logical-range mutations. Those are valid native adaptations.
The remaining limitations are:

- No reusable owner forwarding for horizontal item `setVisibleRange`,
  `resetTimeScale` or `scrollToRealTime`; engine algorithms exist.
- `visible_logical_range` and `visible_time_range` require `&mut TimeScale`;
  `Chart::model()` only returns `&ChartModel`. Range events can be observed, but
  they are not direct current-range query methods.
- No reusable enumeration of a series' price-line IDs. Their storage/projection is
  private. Applications currently must retain IDs themselves.
- No public query for the actual price-scale membership of a series. Pane lookup
  is public; source-to-scale membership/projection is private. This also complicates
  robust series price/coordinate conversion, especially after scale reassignment.
- Built-in options/formatter access exists through `model().series(id)`, but this
  method deliberately returns None for custom series. Typed custom handles expose
  data queries, not equivalent option/formatter/current-value readbacks.
- Crosshair positioning accepts pane/pixel coordinates. Source accepts
  price/horizontal-item/series selectors. Conversion can be performed by the host
  in limited cases; an owner helper should use actual current series scale state.

**Next change:** add narrow checked methods on the existing owner/typed handles;
avoid exposing mutable engine internals to callers. Reuse existing algorithms and
preserve event, invalidation and gesture contracts. Test external-consumer usage,
empty data, absent points, numeric/custom domains and stale selectors.

### F07 — Programmatic pane pixel height is absent (P2, static contract finding)

Source `PaneApi.setHeight` calls `ChartModel.changePanesHeight`, converting desired
pixel height into the layout's pane stretch allocation. Native exposes stretch
factors and divider gestures, but no equivalent reusable pixel-height operation.
Engine `set_pane_height` is not a substitute: layout preparation owns and reapplies
actual pane heights. Do not add a forwarding setter that the next layout overwrites.

**Next change:** implement source-compatible allocation through the chart layout
owner. Test neighboring panes, minimum heights, one-pane/no-op behavior, small
windows and resizing after a programmatic change.

### F08 — Built-in customValues cannot carry application data (P2, static finding)

Native [data_consumer.rs](../../src/model/data_consumer.rs) correctly parameterizes
metadata on rows, but `Chart<B>`, its built-in data methods and `PointerEvent`
instantiate the default metadata type `()`. Applications cannot supply a string or
application record as built-in `custom_values` and retrieve it through the reusable
component, as source `customValues` permits. This is visible in public signatures;
it is not inferred from an empty graph search. Custom series can retain arbitrary
fields in their typed `V::Data`, so that mechanism is not wholly missing.

**Next change:** choose a consistent typed or owned type-erased metadata policy for
built-ins and carry it through updates, queries, pop, snapshots and event payloads.
Avoid silently discarding metadata or requiring a separate application map merely
to emulate the source data contract.

### F09 — Default time tick localization differs from Intl (P2)

**Reproduced.** For the same timestamp, source `ar-EG` day/time labels are `٢` and
`١٢:٠٠`; native labels are `2` and `12:00`. German January matches (`Jan`), providing
a positive control. Source `defaultTickMarkFormatter` uses `Intl` for each tick type.
Native [default_tick_mark_formatter.rs](../../src/model/horz_scale_behavior_time/default_tick_mark_formatter.rs)
localizes month names but formats years, days and times with ASCII numeric output.
Locale resolution in [format_date.rs](../../src/formatters/format_date.rs) also
falls back to English for unrecognized Chrono locale names; browser Intl fallback
and Unicode locale extensions have a different contract.

**H implementation:** the source/native matrix now matches 560 ticks and 192
month/date labels across 16 requested-locale cases, including Arabic numbering,
French April, aliases and selected Unicode extensions. Runtime invalidation,
ASCII date-token/crosshair behavior and callback priority are covered. The
[H contract](time-locale.md) distinguishes verified Gregorian profiles, best-effort
Chrono locales and English fallback. Non-Gregorian calendars and full Intl
compatibility remain open. Renderer and physical acceptance are recorded separately.

### F10 — Pointer notifications omit source-input metadata (P3, static finding)

Source `MouseEventParams.sourceEvent` exposes translated event data, including
modifier keys and local/client/page/screen positions. Native
[PointerEvent](../../src/api/interaction.rs) carries typed chart data/hits but no
modifiers or source-input metadata. DOM event objects and CSS coordinate systems
need not be copied into Rust. The useful native equivalents still need a deliberate
contract if consumers rely on Ctrl/Shift-click behavior or input source identity.

**Next change:** add owned, optional native pointer metadata at the widget/host
boundary and document coordinates. Preserve programmatic-event None semantics.
This is less urgent than incorrect formatting or scale assignment.

## Features and adaptations that should not be reimplemented

The six series definitions, custom-series renderer contracts, screenshot/export,
watermark plugins, manual/update markers, non-time factories, pane movement,
series ordering and data mutation/query milestones remain present. Source row
style options have corresponding native fields. Marker price positions carry
prices in Rust enum variants; they are not missing `price` fields.

The source index reexports enums/factories/series/plugin creators. Native modules,
constructors, enum variants and Default implementations provide their counterparts.
`customSeriesDefaultOptions` maps to typed defaults. JS `version()` and the Rust
crate's own version need not falsely identify the Rust crate as the JS release.
`defaultHorzScaleBehavior` maps to the default time behavior. Export aliases are
listed in the inventory; not all public types are explicitly reexported by the
source index, so that list alone is not the type/API inventory.

Source primitive labels, fixed coordinates, tick visibility, pane/axis geometry,
background/top layers, hit testing and autoscale contributions have native
`PreparedPrimitive`, `PrimitiveAxisLabel`, `PrimitiveView` and `DrawingPrimitive`
counterparts. Source renderer/update/lifecycle methods map to prepared immutable
output, owned renderer traits and attachment Drop/detached behavior. The custom
horizontal trait retains conversion/key/cache/weight/formatter/reset hooks; custom
series retain renderer/update/price-values/whitespace/default/conflation hooks.
Their prior dedicated behavioral suites remain the evidence for those contracts;
this audit did not exhaust every plugin callback combination.

Intentional distinctions include Drop instead of chart.remove/destroy; typed IDs
instead of retained JS wrappers; owned event delivery instead of callback identity;
Iced layout instead of DOM/ResizeObserver; owned RGBA/sRGB snapshots instead of an
HTMLCanvasElement; typed replacement/patch semantics instead of JS object merging;
finite/order/size validation stronger than permissive source boundaries; and the
previously documented default-off native attribution plus runtime-toggle extension.
These require documentation, not mechanical JS-shaped APIs. Host-controlled link
activation is already supported. Existing physical/timing exceptions stay separate.

## Reproduction, validation and next milestone

```sh
node scripts/audit_public_surface.cjs --check
cargo run --offline --example parity_audit
node scripts/probe_public_parity.cjs
```

The last command needs the existing local source TypeScript/Rollup/Puppeteer
packages and Chromium. It drives a headless browser, asserts source preconditions,
runs the Rust diagnostic and saves source/native comparisons under
`target/parity-audit`. It reports mismatches as findings; **a successful audit
command does not mean parity passed**. The native example opens no window and
prints observations, without baking current bugs into normal passing regression
tests. Source hashes, comparisons, validation and coverage summary are retained in
[public-parity-evidence.json](public-parity-evidence.json).

Recommended first implementation milestone: **formatting parity (F01/F02)**, followed
by **series scale reassignment (F03)**, then **horizontal-axis interaction/settings
(F04)**. Each needs source-derived failing regressions before fixes, actual axis/
render/event verification and unchanged existing compatibility gates. Other findings
can follow without reopening the completed feature roadmap or changing its scope.

This audit adds only the inventory generator, diagnostic example, source/native
probe and reports. No new dependency, engine async machinery or acceptance threshold
change. No physical-device, GPU-gamut, performance or exhaustive screenshot matrix
was rerun. Their earlier results are historical evidence, not new audit results.


Validation of this audit change: inventory drift check, JavaScript syntax checks,
formatting, all-target cargo check and strict Clippy passed. The six focused
integration suites (series data, price-scale configuration, public events, pane
management, custom horizontal behavior and attribution) passed all 52 tests.
The final source/native probe completed with zero browser errors and saved all
12 observations. These validate the diagnostics and preserve relevant existing
behavior; the reproduced differences remain unresolved.


Milestone C follow-up: horizontal-axis dragging/reset and independent price/time
axis settings are implemented with source-backed Canvas regressions, including
configured default/pixel-offset reset and release-time price reset. All-disabled
edge-label alignment now follows the existing engine policy through public settings.
See [axis-interaction.md](axis-interaction.md) for final validation and acceptance;
this does not resolve construction-only default-visible-scale configuration or
any older device/timing exception.


Milestone D follow-up: F06 now has checked owner range/navigation helpers,
creation-order price-line enumeration, typed/scoped option and formatter reads,
owned last values, current membership, price/domain conversions and silent
synthetic crosshair placement. Scoped options deliberately preserve Rust callback
ownership rather than reproducing JavaScript cloning. See
[public-helpers.md](public-helpers.md) for individual dispositions and validation;
this does not resolve other findings or older acceptance exceptions.

### Milestone E follow-up — F07

`Chart::set_pane_height` and the yield-curve forward now convert requested logical
pixels into persistent stretch allocation, matching source sequential neighbor
redistribution, clamp and resize behavior. Current pixel readbacks remain available
through pane/frame snapshots. Source/native assertions cover settled allocation,
resize, move and removal. Unprepared/collapsed layouts preserve valid factors
instead of accepting source zero-division/negative-factor states. This is an
explicit native safety adaptation. See [pane-sizing.md](pane-sizing.md) for the
contract, consumer, regression and acceptance evidence; physical acceptance is
tracked there rather than inferred from API presence.


### Milestone F follow-up — F08

The reusable owner now exposes the existing engine metadata parameter as
`Chart<B, M = ()>`, including canonical data/query/pop, pointer events, primitive
contexts, numeric/yield/custom-domain factories and typed custom adapters. Existing
unit constructors remain inference-compatible. Payloads use their own Clone
policy; Rc preserves identity without mandatory deep copies or Debug/PartialEq/
Send/Sync constraints. Source assertions cover all six built-in families. See
[application-metadata.md](application-metadata.md) for exact ownership, snapshots,
consumer migration and acceptance; separate device/timing records remain open.

### Milestone G follow-up — F10

`PointerEvent.source_event` now owns optional native device/contact, event-kind,
changed-button, chart/window logical positions and observed modifier data. The
Canvas samples before dispatch; delayed notifications keep the snapshot. Unknown
modifiers stay None across focus transitions, and ordinary programmatic/redraw
input stays unsourced. Explicit host-supplied metadata is supported. DOM page,
screen, target/view references and preventDefault are deliberately replaced by
the native coordinate/capture contract. Mouse identifies Iced's event family,
not physical hardware. See [native-pointer.md](native-pointer.md) for source,
consumer, migration and acceptance evidence; separate device/timing checks remain
independent.

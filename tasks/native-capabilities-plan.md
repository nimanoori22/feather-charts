# Native capability completion plan

Requested scope: plan all three capabilities; implement only C in this change.
Baseline: `fb096eb`. Source: local Lightweight Charts 5.2.1.
Checklist: [native-capabilities-todo.md](native-capabilities-todo.md).
Earlier public-parity completion and independent exceptions remain historical evidence.

| Capability | Current status | Authorization |
|---|---|---|
| A — Display-P3 drawing/export | **NOT IMPLEMENTED**. Existing sRGB drawing and configured color parsers remain implemented. | Planning only |
| B — Broader Intl-equivalent chart formatting | **NOT IMPLEMENTED**. Existing verified Gregorian locale profiles remain implemented. | Planning only |
| C — Runtime default price-axis selection | **IMPLEMENTED AND ACCEPTED** on software and GPU | Implement and verify now |

## A — Display-P3 (planning only; not implemented)

A0 (M): freeze the source color-space contract and native renderer feasibility.
Inspect source chart creation, canvas color-space selection and screenshot behavior,
then the exact Iced/wgpu/tiny-skia and compositor boundaries. Record which paths can
preserve P3 and which must reject it. Choose profile/transfer-function handling,
color precision and capability reporting before changing the renderer. Dependency:
none. Deliverable: capability matrix and design with explicit unsupported cases.

A1 (M): define color-managed preparation without changing existing sRGB output.
Separate the authored color's space from the destination space, define conversion,
compositing and gamut-mapping behavior, and preserve parser callback/cache rules.
Use reference color vectors for colors inside/outside sRGB and transparency.
Dependency: A0. Likely files: model/colors, layout target policy and color tests.

A2 (M per backend): implement an end-to-end screen path for each capable backend.
Propagate the chosen destination through prepared frames, GPU surface/attachments
and desktop presentation; implement a software destination or explicitly reject it.
Preserve the old sRGB path and expose the actual supported target. Dependency: A1.
Likely files: native renderer adapters and targeted Iced/vendor integration.
Verification: backend assertions, reference-vector readback, unchanged sRGB images.

A3 (M): add color-space-aware owned export and retained-snapshot behavior.
Specify raw pixel interpretation and any file profile metadata; do not label sRGB
bytes as P3. Match source screenshot choices where applicable. Dependency: A2.
Likely files: screenshot/export types, exporter and export tests.

A4 (M): demonstrate and accept the capability. Test all four horizontal domains,
regular/narrow/short layouts, DPRs 1/1.25/1.5/2, both supported renderer paths and
screen/export consistency. Validate on a known wide-gamut display through the real
OS presentation path; record monitor/profile/backend identity. Dependency: A3.
No acceptance claim from an enum, successful parser, or an sRGB capture alone.

## B — Broader Intl parity (planning only; not implemented)

B0 (M): freeze the chart-used formatting surface and supported locale/calendar
matrix against actual source Intl calls, including fa-IR/Persian-calendar behavior,
numbering extensions, month abbreviations, fallback, UTC and date-token distinctions.
Do not port unrelated Intl facilities. Dependency: none. Deliverable: source fixtures
and a support contract extending, not weakening, the existing H profiles.

B1 (M): evaluate the smallest sufficient formatter/data implementation. Compare
existing Chrono behavior with an ICU4X-backed provider using exact compatible
versions, CLDR data size, deterministic data versions and platform needs. Record
licensing and dependency impact. Install/add nothing during this planning task.
Dependency: B0. Deliverable: dependency decision and representative red tests.

B2 (M per formatter): implement locale/calendar/digit resolution and the required
tick and item formatters separately. Preserve custom callbacks, source ASCII token
rules, runtime cache invalidation and explicit supported/best-effort/fallback reads.
Dependency: B1. Likely files: formatters/time_locale, format_date, time behavior and
focused locale tests. Existing verified profiles must remain unchanged.

B3 (M): verify source parity and native rendering/export. Include timestamps at
rollovers, negative/submillisecond values, invalid tags, unsupported extensions,
month sets, callback overrides and retained snapshots. Exercise all four domains;
numeric/session domains retain their own behavior. Dependency: B2. Add normal
software/GPU acceptance and record remaining unsupported combinations explicitly.

## C — Runtime default price axis (implement now)

C0 (S): freeze source semantics and add failing public tests. Source chart options
own one preference; pane selection chooses preferred populated visible scale,
then the other, then source/visible/empty fallbacks. Existing series stay assigned;
new panes observe the current preference. Native add-series calls keep their
explicit typed scale argument. Dependencies: none. Files: contract report, tests
and a focused source oracle. Verify invalid/equal patches and immutable readbacks.

C1 (S): extend existing `ChartModelOptionsPatch` with
`default_visible_price_scale: Option<DefaultVisiblePriceScale>`. Reexport the
existing Left/Right enum through the public API. Preview the change atomically,
copy it to existing panes/defaults, classify it as an interaction-target change and
reuse the existing invalidation/crosshair refresh. Read through
`runtime_options().model.pane.default_visible_price_scale`. Dependencies: C0.
Likely files: model/chart_configuration, chart_model/configuration, api/configuration.
No separate setter, new shared ownership, strings, async runtime or renderer fork.

C2 (M): verify actual effects and provide desktop controls. Cover grid marks,
normal/magnet crosshair conversion, overlay label side, scale visibility/emptiness,
custom series, existing/new/moved/removed panes, preserved IDs/data/manual ranges,
no-op/invalid patches and cancellation of active viewport gestures. Add controls
to the existing configuration demo, with a two-axis comparison setup, using its
existing Host forwarding in calendar/strike/maturity/session modes. Dependencies: C1.

C3 (M): validate and record. Run formatting, all-target tests/check/build, doctests,
strict Clippy and unchanged source comparisons plus the new oracle. Freeze focused
software/GPU captures at regular/narrow/short sizes and supported DPRs, and rerun
affected unchanged protocols. Complete normal software/GPU mouse checks without
heavy work running concurrently. Update inventory/score provenance and public
examples only from current evidence; retain Z0 history and partial P3/Intl rows.
Save local checkpoints; do not push. Dependencies: C2.

## Design and ownership

Alternative 1 (chosen): add an optional typed field to the existing atomic model
patch. It shares existing validation, readback, no-op detection, pane propagation
and owner invalidation with other chart options; no extra allocations beyond the
existing patch path. Alternative 2: a standalone per-pane setter. This duplicates
configuration routing and introduces per-pane policy diverging from source global
options; it is unnecessary for the requested capability.

Chart owns model/defaults and panes; each pane keeps a copied enum. Existing frames
own prepared output. No new references, callbacks, Rc cycles or drop ordering are
introduced. A changed preference cancels viewport gestures through the existing
geometry-change policy so a drag cannot continue against an obsolete scale target;
this is an explicit native ownership adaptation. Equal and invalid patches preserve
current gestures/events. Box/series ownership and unrelated scale ranges remain.

Display-P3 and broader Intl implementation tasks stay unchecked in this delivery.
Existing timing, drawing-order and unavailable-device acceptance stay independent.

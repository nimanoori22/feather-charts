# H — supported calendar locale formatting

Baseline: `3494b04`. Source: local Lightweight Charts 5.2.1, revision
`8a7c94ff8ddffde68955db61ef8b58098f239de0`. Frozen implementation hashes and
browser observations are `time-locale-source.json` and `time-locale-states.json`.
The oracle runs the actual source default tick and date formatters in Chromium,
with English host locale and UTC timezone. Preserve its failed native run at
`target/time-locale/red-labels.log`.

## Source contract and consumers

`default-tick-mark-formatter.ts` formats year, short month, day and 24-hour time
using Intl. `format-date.ts` localizes month names only: numeric date tokens are
ASCII. `DateTimeFormatter` also retains ASCII time fields. The time behavior
uses the first for axis ticks and the latter for crosshair items. Custom tick
callbacks receive the original requested locale and may decline with null;
custom item callbacks replace default item formatting. The time scale caches
formatted marks; the existing localization patch invalidates those caches and
rebuilds the item formatter. No change to data keys or navigation is needed.

Source observations include all twelve months, year rollover, midnight, negative
and submillisecond timestamps, aliases, Unicode extensions, unknown and invalid
locales. The current source runtime uses newer CLDR data than the initial audit:
German short September is `Sep`, already matching Chrono. The actual month gaps
are French April (`avr.`) and Arabic short month names (full names in Intl).
Arabic `ar` uses Latin digits in this source runtime; `ar-EG` uses Arabic digits.

## Chosen support boundary

Verified Gregorian profiles: en-US, fr-FR, de-DE, ar-EG and language aliases en,
fr, de, ar, case-insensitive. `default` is deterministic English in Rust; the
browser fixture has an English host. Recognized Unicode options are ca-gregory,
nu-latn/arab/arabext and hc-h23, in either keyword order. Tick digits follow the
numbering option. Date tokens and crosshair times remain ASCII, matching source.
UTC dates use JS millisecond truncation before extracting fields, including
negative submillisecond values and second-boundary rounding.

Other Chrono locales retain best-effort Gregorian month formatting. Unknown or
invalid locales fall back to English without panicking. Unsupported extension
values are best effort, even when their result happens to match Intl. A public
`time_locale_support(&str) -> TimeLocaleSupport` query distinguishes Supported,
BestEffort and EnglishFallback. It reports the built-in calendar formatter only,
not an application's custom formatter or the unrelated numeric horizontal axes.

This is partial Intl compatibility. Non-Gregorian calendars, locale-specific
suffixes outside the verified profiles, arbitrary numbering systems, exhaustive
BCP47 canonicalization/validation, host-dependent defaults, ancient calendar eras
and browser DST-constructor quirks are outside the verified contract. Numeric,
yield and application-defined session axes retain their domain formatting.
Custom callbacks can supply broader application-specific behavior.

## Designs and ownership

1. Reuse Chrono locale data with a small verified-profile resolver, targeted month
   corrections and numeric tick digit substitution. No new dependency or global
   state. Existing chart options own requested strings and Rc callbacks; prepared
   snapshots own label strings. The resolver is temporary owned data. There are
   no additional handles, listeners, cycles or async work. Chosen because it fixes
   the demonstrated gaps while retaining existing best-effort locales.
2. Add ICU4X locale/calendar/number formatting with compiled CLDR data. This offers
   broader locale parsing and calendar coverage but requires a larger formatting
   and data-version contract; it does not by itself guarantee ECMA-402 output or
   preserve the source's ASCII date-token distinction. Defer until broader locale
   requirements justify the dependency and data footprint. See the official
   [ICU4X quickstart](https://icu4x.unicode.org/2_0/tutorials/quickstart/) and
   [date formatter API](https://docs.rs/icu/latest/icu/datetime/struct.DateTimeFormatter.html).

The public query is additive. Existing formatter signatures, callback priority,
requested locale readback and patch APIs remain intact. Source's local-Date UTC
field reconstruction is incidental to JS; native formatters use UTC directly.

## Verification and delivery

Native/source matrix, fallback classification, runtime label invalidation,
callback priority and key/snapshot stability precede demo work. Freeze a native
capture matrix of four domains, three sizes (800x500, 300x300, 300x90), four pixel
ratios (1, 1.25, 1.5, 2) and both asserted renderers. Scenes cover supported
profiles, numbering overrides, fallback, custom callback and short-height recovery.
Retained exports must survive chart changes/drop. Reuse existing capture helpers.
Rerun unchanged affected formatting compatibility captures and all common gates.
Physical mouse checks use normal software then GPU windows. Unavailable device,
earlier timing, formatting GPU and drawing-order exceptions remain independent.

## Implementation and automated results

Formatter checkpoint: `b083cec`; consumer checkpoint: `4daa116`. No dependency
or lockfile change. The implementation adds
`src/formatters/time_locale.rs`, updates `src/formatters/format_date.rs`,
`src/formatters/mod.rs` and
`src/model/horz_scale_behavior_time/default_tick_mark_formatter.rs`.
Existing localization patch, cache invalidation and callback ownership are reused.
Tests are `tests/time_locale.rs`, `tests/localization_runtime.rs` and the locale
controls test in `examples/pane_management/locale.rs`.

The demo uses `examples/pane_management/{locale,locale_captures}.rs`, its existing
host bridge and main entry. Launch with:

```sh
cargo run --example pane_management -- --locale --screenshot-export --live-feed
```

Add `--options`, `--yield-curve` or `--custom-domain`. Calendar demo data spans
January–April so changing months is visible. Other domains retain their original
keys and label behavior. The latest update reuses its actual last key after the
demo's calendar data replacement. Locale changes themselves never replace data.
`Item override` toggles the existing custom item callback; `Date tokens` changes
only calendar item formatting. Unsupported/invalid input keeps its requested
readback while showing the public support status. No browser locale, DOM Date,
Intl object identity or global process locale is introduced.

Common gates pass: 872 tests in 89 suites, two compile-fail doctests, offline
all-target check/build, Clippy with warnings denied, formatting and public-surface
inventory. The locale source/native comparison matches 560 ticks and 192 date/month
labels. Both asserted native renderers pass 960 total screen replays and 960
retained exports over four domains, three sizes and four DPRs. Arabic changes
actual calendar pixels; invalid-locale fallback reproduces English pixels. Exports
retain their original labels after locale changes, Reset and owner drop.
The unchanged formatting protocol matches all 576 previously accepted images.
Software fractional-DPR and GPU narrow Arabic captures were visually inspected,
including connected month names and Arabic/extended Arabic digits.

All 44 earlier source-comparison scripts are unchanged. All 45 source comparisons pass. The user passed normal software and GPU mouse
acceptance in all four domains, including previews/exports and regular, narrow,
earlier problem-size and short-height recovery checks. All eight sessions exited
successfully with the validated binary. The final record is
[time-locale-evidence.json](time-locale-evidence.json). Milestone H is accepted. Larger combined compatibility captures
remain Z0; this milestone reruns the directly affected formatting protocol.
Full Intl, earlier timing, unavailable devices and other independent acceptance
exceptions remain open. Milestone I (configured color parsing) is the next
feature dependency; H adds no provider infrastructure that I must adopt.

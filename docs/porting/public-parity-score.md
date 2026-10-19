# Measured parity scorecard — local Lightweight Charts 5.2.1

The reproducible **native API-surface coverage score is 98.65%: 365/370**
applicable declared members in the audited inventory. This replaces the earlier
informal 97% estimate with a scoped measurement. It does not establish an overall
behavioral parity percentage for the complete library.

| Measurement | Result | What it measures |
|---|---:|---|
| Native API/option/data surface coverage | **365/370 = 98.65%** | Reviewed mapping, native adaptation or represented Rust field; four browser-only contracts excluded |
| Public method surface coverage | **94/96 = 97.92%** | Methods under the same classification rule; three browser-only methods excluded |
| Property surface coverage | **271/274 = 98.91%** | Option/data/payload fields under the same rule; one browser-only property excluded |
| Surface coverage including browser-only differences | **365/374 = 97.59%** | Same numerator; platform distinctions remain in the denominator |
| Recorded source-conformance suites | **46/46 = 100%** | All existing suites pass their selected source-backed cases |
| Original source/native audit observations | **12/12 = 100%** | All original diagnostic observations match; these overlap the source suites |
| Native image regression comparisons | **9,510/9,510 = 100%** | Final renders match accepted native images; this is not a browser/native pixel-parity measurement |
| Whole-library behavioral parity | **Unmeasured** | Requires a larger contract inventory and member-to-source-assertion traceability |

These metrics are separate. Adding tests, observations and images into one
percentage would double-count overlapping behavior and weight features by how
many fixtures happen to exist.

## Scoring rule and denominator

Use the existing bounded inventory: 55 declarations, 374 declared members,
including 99 methods. Base interfaces are counted once; inherited members are not
expanded again. Every member has equal weight for this surface metric.

- `native` (81), `mapped` (80) and `represented` (204) each receive one surface
  coverage point. They indicate a reviewed Rust adaptation, an implementation
  entry point, or a real corresponding field respectively.
- `partial` (5) receives zero full-coverage points. No arbitrary half-credit is
  assigned to partially supported behavior.
- `platform` (4) is excluded from the native desktop denominator and shown
  explicitly. An alternative measurement retains those rows in its denominator.
- A future `gap` or `review` classification receives zero points.

Thus the native numerator is `81 + 80 + 204 = 365`; the denominator is
`374 - 4 = 370`. In particular, the 204 represented fields are not 204 proven
behavioral matches. Their inclusion makes this a surface coverage measure.

The five partial rows are `IChartApiBase.applyOptions`, `IChartApiBase.options`,
`ChartOptionsBase.defaultVisiblePriceScaleId`, `LayoutOptions.colorSpace` and
`LocalizationOptionsBase.locale`. These overlap three capability limits:
construction-only default visible-scale selection, deferred Display-P3 output,
and partial Intl support. Five rows do not mean five equally sized missing
features.

The four exclusions are `IChartApiBase.autoSizeActive`,
`IChartApiBase.chartElement`, `IPaneApi.getHTMLElement` and
`ChartOptionsBase.autoSize`. They describe DOM/ResizeObserver integration replaced
by the native Iced host; they are not silently counted as full JS compatibility.

## Evidence and reproducibility

The calculation consumes [the inventory](public-parity-inventory.json) and
[Z0 completion evidence](public-parity-completion-evidence.json). It verifies
pinned evidence/log hashes, source-check script hashes and final implementation/
demo source identities before producing [the machine-readable score](public-parity-score.json).
The source results are recorded Z0 results; this task reuses that validated
snapshot and does not claim a fresh run of all tests or captures. The internal
Rust results remain 884 passing tests in 90 suites plus two compile-fail doctests.

```sh
node scripts/audit_public_surface.cjs --check
python scripts/measure_public_parity.py --self-test
python scripts/measure_public_parity.py
python scripts/measure_public_parity.py --check
```

The inventory drift check validates current source declarations/mappings. The
calculator rejects duplicate members, unknown classifications and changed pinned
inputs. Four self-tests cover exclusion/partial-credit arithmetic, rounding, duplicate/
classification rejection and an empty native denominator. An empty denominator
produces an unmeasured result, not a fabricated zero or 100%. A temporary mutation
that removed duplicate-member protection was rejected by the self-tests; the
production calculator was left intact.

## What is needed for a whole-port behavioral score

The inventory does not map every member to an explicit source-conformance test.
It also excludes some exported aliases, plugin contracts and combinations of
options/lifecycle operations. Consequently, an API being present or a test suite
passing does not establish what fraction of all upstream behavior is verified.
A measured behavioral score needs a frozen contract/case ledger covering that
larger scope, each row linked to a source-derived assertion and marked pass,
fail, untested or an explicit native exception. Untested rows must stay in that
behavioral denominator. Its score can then be calculated independently of the
surface metric above.

Existing timing, drawing-order and physical touchpad/touchscreen/high-DPI
acceptance remain open. Full Intl and Display-P3 output remain partial/deferred.
This measurement does not close those gates or estimate their engineering effort.

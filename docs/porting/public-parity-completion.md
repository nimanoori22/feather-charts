# Public API and desktop parity — combined completion

Status: Z0 complete within the plan’s scope; independent exceptions remain open. Validated implementation checkpoint: `bcf44e8`.
Plan: [public-parity-plan.md](../../tasks/public-parity-plan.md).
The scope is the ten finding groups in the local Lightweight Charts 5.2.1 audit,
not an exhaustive compatibility claim or a measured port-completion percentage.
No chart-engine, input or renderer code is changed by this completion step.

## Addressed contracts

| Audit group | Implemented behavior and detailed evidence |
|---|---|
| F01/F02 | [Global price/percentage localization and custom batch formatting](formatting-parity.md) |
| F03 | [Checked built-in/custom series scale reassignment](scale-assignment.md) |
| F04 | [Horizontal-axis drag/reset and independent axis controls](axis-interaction.md) |
| F05 | [Chart-owned configured color parsing and explicit unsupported-target errors](configured-colors.md) |
| F06 | [Public range, navigation, series, coordinate and crosshair helpers](public-helpers.md) |
| F07 | [Persistent pane pixel sizing and layout restoration](pane-sizing.md) |
| F08 | [Typed application metadata through data, queries and events](application-metadata.md) |
| F09 | [Supported Gregorian locale/numbering behavior with explicit fallbacks](time-locale.md) |
| F10 | [Owned native pointer source and observed modifier metadata](native-pointer.md) |

Each report retains its source-component analysis, real consumers, contract,
Rust ownership/API decisions, alternatives, migration, files and behavioral tests.
The implementation keeps synchronous engine/preparation paths and immutable draw/
export frames. It adds no async runtime, thread-safe owner or dependency for Z0.

## Final checks

The common gates pass: 884 tests in 90 suites, two compile-fail doctests,
all-target check, strict Clippy, example build, formatting and inventory drift.
The unchanged original diagnostic now matches all 12 source/native observations;
its historical output remains intact. Focused acceptance assertions are also
maintained in the source-comparison suites; diagnostic success alone is not used
as acceptance. All 46 source comparisons pass. All 13 combined capture families
pass with 9,510 images exactly matching their accepted results on the final source. The
screenshot protocol also passes its nested 912-image compatibility run, including
the original 96 baseline checks; these nested images are not added to 9,510.

Artifacts are retained under `target/public-parity-final/`. Common commands:

```sh
cargo build --offline --examples
cargo test --offline --all-targets
cargo test --offline --doc
cargo check --offline --all-targets
cargo clippy --offline --all-targets -- -D warnings
cargo fmt -- --check
node scripts/audit_public_surface.cjs --check
git diff --check
```

Run each `scripts/check_*reference.cjs` with Node. The combined capture manifest
freezes 13 unchanged protocols and their accepted result hashes before execution.
For each family, run `python scripts/capture_<family>.py <new-output-directory>`.
The exact names, commands, image counts and result hashes are pinned in
[the completion evidence](public-parity-completion-evidence.json). Every image key
and digest must match the accepted family;
counts, backend assertions and the old 96-image compatibility gate are retained.
No image threshold or expected scene was relaxed.

## Unchanged combined capture families

| Script | Images matching accepted results |
|---|---:|
| `capture_public_events.py` | 912 |
| `capture_custom_horizontal.py` | 240 |
| `capture_screenshot_export.py` | 1158 |
| `capture_attribution.py` | 672 |
| `capture_formatting.py` | 576 |
| `capture_scale_assignment.py` | 768 |
| `capture_axis_interaction.py` | 864 |
| `capture_public_helpers.py` | 576 |
| `capture_pane_sizing.py` | 672 |
| `capture_application_metadata.py` | 864 |
| `capture_native_pointer.py` | 576 |
| `capture_time_locale.py` | 960 |
| `capture_configured_colors.py` | 672 |

Both actual backends pass every family. The helper protocol retains its original
three DPRs (1/1.25/2); the other new milestone matrices retain 1/1.25/1.5/2. No
protocol is expanded or reduced merely to normalize the table. Screen replays,
retained exports and family-specific lifecycle assertions run unchanged.

## Physical scope

The user passed all four final color-demo modes on tiny-skia and wgpu, including
live-feed navigation, pane/scale moves, axes, box editing/cancellation/release
outside, recovery, parser idle counts and retained-color previews/exports.
All eight normal sessions exited with code zero. The executable and source hashes
match this final implementation; this record reuses those direct physical results
without claiming a new mouse run. Sizes are the user's regular, narrow and earlier
problem window sizes, plus shrinking/enlarging for recovery; exact physical pixel
dimensions were not supplied. Automated capture dimensions are recorded separately.
Earlier per-feature physical records remain linked in their milestone reports.

Z0 also adds a new, explicit four-mode wgpu formatting check. The user passed
all formatting controls, precedence/clearing, scale modes, live-feed interactions,
three window-size scopes, recovery and retained preview/export labels. All four
normal sessions exited successfully with the same validated binary. This resolves
A4’s separate GPU record alongside the earlier general user acceptance; it does
not invent a new itemized software formatting run.

## Limits retained

- Display-P3 output and hardware gamut acceptance remain unimplemented. Parsing a
  custom color into sRGB does not establish a wide-gamut renderer.
- Full Intl behavior outside H's supported Gregorian/numbering profiles remains
  partial; best-effort and English fallbacks are observable.
- `defaultVisiblePriceScaleId` remains a construction setting through
  `PaneOptions.default_visible_price_scale`; runtime replacement is outside the
  completed axis-input scope. Aggregate options/applyOptions inventory rows remain
  partial for this and the deferred output target.
- The earlier timing exception, dedicated series drawing-order mouse acceptance,
  physical touchpad, touchscreen and high-DPI acceptance remain open. Synthetic DPR captures do not close them.

The bounded inventory retains 55 declarations, 374 members and 99 methods. Its
five partial rows remain partial. Representation and mapped API counts are not a
behavioral parity score. Source worktree modifications already present before this
plan remain preserved and are identified by the source fixtures' consumed hashes.

## Files and review

Z0 updates this report, its generated evidence, the plan/checklist, README, the
historical audit and formatting-acceptance follow-ups, and two stale inventory
descriptions plus the generated inventory. It changes no Rust API or runtime behavior. Review checks result counts,
source/binary identities, exact old-image comparisons, physical evidence provenance
and preservation of every partial/deferred classification. The completion
checkpoint is the local commit containing this report and evidence; its parent
implementation checkpoint is `bcf44e8`. The worktree is checked after that commit.
No push is authorized.

The original sandbox-denied Chromium run is retained separately. Its browser
startup errors were resolved by rerunning with browser access; all 46 unchanged
source scripts then passed. Final diff, JavaScript syntax and inventory checks pass.

The [measured scorecard](public-parity-score.md) separates audited surface
coverage from recorded source-conformance pass rates and unmeasured whole-library
behavioral parity. Its calculator pins this completion evidence without treating
the completed roadmap as a 100% port score.

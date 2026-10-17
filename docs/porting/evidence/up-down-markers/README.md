# Up/down marker evidence

Date: 2026-10-17. Native raw captures: `target/up-down-markers/final`.

`protocol.json` pins binary, Rust/vendor/fixture and font identities for the final
120-feature/24-default-off matrix; `results.json` records hashes and successful
actual software/GPU backends. `baseline-protocol.json` records verified retained
pre-feature provenance. Capture failures from the initial wrong runner flag remain
in `target/up-down-markers/after`; corrected earlier views in `corrected`.

All-target logs record 654 passing tests; check, Clippy and formatting pass. The
final change after the full suite only zoomed capture scenes and was executed in
the final matrix; check/Clippy include it. Source logs are actual-source comparisons.
Behavioral RED logs retain the failures that drove rendering, updates, expiration,
widget scheduling and stale-controller cleanup.

Physical software/GPU acceptance is recorded in the port report after user testing.
Other device checks and historical timing acceptance remain separate.

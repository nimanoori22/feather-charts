# Yield-curve chart port

Implementation in progress. Baseline `24bd3ba`; plan commit `1112050`.
Source working-tree hashes: [yield-curve-source.json](yield-curve-source.json).

## Contract checkpoint

Source files, consumers, alternatives and provisional API are documented in
[tasks/yield-curve-plan.md](../../tasks/yield-curve-plan.md). The source oracle
executes the original behavior, whitespace helpers and DataLayer with explicit
microtasks. Twenty default/custom labels and weights, eight scaffold states and
nine real DataLayer lifecycle states are frozen before production edits. It writes
`target/yield-curve/source-contract.json` for native comparison. Extent remains
420 months after shorter replacement, longest-series removal and clearing public
data; native settlement preserves this source scaffold feedback behavior.

Finite negative/fractional maturities are accepted; signed zero is normalized.
The scaffold clamps start/minimum to zero and resolution to at least one. Its
ordered union with real maturities defines coordinates; off-grid points are not
reprojected onto an invented continuously linear axis. Default formatting reads
localization.timeFormatter, not the declared but unused yieldCurve.formatTime.

Native validation rejects nonfinite options/maturities, invalid row/value/order
and scaffold counts above 100,000 points before mutation. This explicit resource
limit is a native correction, not source parity. The budget allows over 8,000 years
at monthly resolution while bounding support memory/work. Explicit option
replacement will settle the scaffold coherently rather than depending on a later
browser microtask/data mutation. Public iteration/events hide the private support
series. Numeric custom series and arbitrary chart-family plugins are deferred.

## Chosen design and ownership

A shared behavior-generic chart core with calendar defaults is preferred over a
separate numeric frontend: it reuses mutation, preparation, event and widget code
without duplicating an engine or navigation loop. A typed yield frontend limits
public built-in additions to Line/Area and owns private scaffold state. A Time-or-
Number enum was rejected: it would admit mixed invalid data and branch hot paths.

Chart owns model/data plus attachments and marker controllers. Numeric scaffold
state is owned by the yield chart; behaviors never reference their owners. Shared
primitive state has no chart back-reference. Preparation freezes coordinates,
colors and labels; old frames retain no live scaffold/controller state. Formatter
closures use existing Rc callbacks; hosts must avoid strong owner captures.

The calendar API and third-party calendar DrawingPrimitive implementations must
remain source-compatible. Numeric event payloads and primitive contexts carry
numeric items. Time-specific custom-series methods remain specialized. No async
engine, locks, new dependency or renderer patch is planned.

Validation, exact files, API, capture and native acceptance results will be added
as each gate completes. Historical timing acceptance remains OPEN/deferred;
touchpad, touchscreen and physical high-DPI checks remain separate.

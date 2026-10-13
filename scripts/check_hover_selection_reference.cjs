const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const ts = require(require.resolve('typescript', { paths: [root] }));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root, 'src', file), 'utf8'), {
        compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020 },
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm, '').replace(/\bexport /g, '');
}
const priority = { Range: 0, Line: 1, Point: 2 };
const lineType = { Simple: 0, WithSteps: 1, Curved: 2 };
const algorithms = new Function(`${source('helpers/algorithms.ts')}\nreturn {lowerBound, upperBound};`)();
const common = new Function('lowerBound', 'upperBound', `${source('renderers/hit-test-common.ts')}\nreturn {distanceToSegment, hoveredSeriesHitTestResult, isWithinHorizontalSweep, lowerBoundByX, upperBoundByX, rangePair};`)(algorithms.lowerBound, algorithms.upperBound);
const controls = new Function(`${source('renderers/walk-line.ts')}\nreturn getControlPoints;`)();
const names = ['HitTestPriority', 'LineType', 'getControlPoints', ...Object.keys(common)];
const values = [priority, lineType, controls, ...Object.values(common)];
const line = new Function(...names, `${source('renderers/line-hit-test.ts')}\nreturn hitTestLineSeries;`)(...values);
const range = new Function(...names, `${source('renderers/range-hit-test.ts')}\nreturn hitTestSeriesRange;`)(...values);
const output = spawnSync(path.resolve('target/debug/examples/hover_selection_reference'), { encoding: 'utf8', maxBuffer: 100 * 1024 * 1024 });
assert.equal(output.status, 0, output.stderr);
const cases = JSON.parse(output.stdout);
for (const c of cases) {
    const result = c.kind === 'line'
        ? line(c.items, { from: c.from, to: c.to }, c.x, c.y, c.lineType, c.width, c.radius ?? undefined, c.spacing, c.tolerance)
        : range(c.items, { from: 0, to: c.items.length }, c.x, c.y, c.spacing, c.tolerance, (p, pair) => { pair[0] = p.top; pair[1] = p.bottom; });
    const expected = c.expected;
    assert.equal(result === null, expected === null, JSON.stringify(c));
    if (result !== null) {
        const distance = c.kind === 'line' ? expected[0] : expected;
        assert.ok(Math.abs(result.distance - distance) < 1e-9, JSON.stringify(c));
        assert.equal(result.priority, c.kind === 'line' ? expected[1] : priority.Range);
    }
}
console.log(`${cases.length} Line/point/curve/time-slot/range cases match actual TypeScript.`);

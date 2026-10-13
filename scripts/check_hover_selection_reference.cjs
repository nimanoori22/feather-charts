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
const better = new Function('HitTestPriority', `${source('model/internal-hit-test.ts')}\nreturn isBetterHit;`)(priority);
const paneHit = new Function('HitTestPriority', 'isBetterHit', `${source('model/pane-hit-test.ts')}\nreturn hitTestPane;`)(priority, better);
const horizontal = new Function('BitmapCoordinatesPaneRenderer', 'HitTestPriority', `${source('renderers/horizontal-line-renderer.ts')}\nreturn HorizontalLineRenderer;`)(class {}, priority);
const order = new Function(`${source('model/hovered-source-order.ts')}\nreturn hoveredSourceOnTopOrder;`)();
const fillComposite = new Function(`${source('model/series/fill-series-hovered-pane-view.ts')}\nreturn FillSeriesCompositeRenderer;`)();
for (const c of cases) {
    if (c.kind === 'order') {
        if (!c.filled) {
            assert.deepEqual(c.passes, order([0,1,2], c.hovered, c.enabled).map(i => [i,0]));
        } else {
            const drawCalls=[];
            const normal=[];
            for (const i of [0,1,2]) {
                if(i !== c.hovered) {normal.push([i,0]); continue;}
                new fillComposite({draw:()=>drawCalls.push('fill')}, {draw:()=>drawCalls.push('line')},()=>true,()=>c.enabled).draw(null,true);
                normal.push([i,drawCalls.includes('line') ? 0 : 1]);
            }
            if(c.enabled) normal.push([c.hovered,2]);
            assert.deepEqual(c.passes,normal);
            assert.equal(drawCalls.filter(x=>x==='fill').length,1);
        }
        continue;
    }
    if (c.kind === 'pane') {
        const sources = c.kinds.map((kind, i) => ({
            index: i,
            paneViews: () => [{hitTest: () => ({distance: c.distances[i], priority: kind >= 2 ? priority.Point : kind === 1 ? priority.Line : priority.Range, itemType: kind === 3 ? 'price-line' : 'series-line'})}],
            primitiveHitTest: () => c.marker === i ? [{zOrder: 'normal', itemType: 'marker', isBackground: false}] : [],
        }));
        const result = paneHit({orderedSources: () => sources}, 50, 50);
        assert.equal(result.source.index, c.series, JSON.stringify(c));
        assert.equal(result.itemType === 'marker', c.isMarker, JSON.stringify(c));
        continue;
    }
    if (c.kind === 'horizontal') {
        const renderer = new horizontal();
        renderer.setData({y: 50, lineWidth: c.width, visible: true});
        assert.equal(renderer.hitTest(50, 50 + c.offset) !== null, c.hit, JSON.stringify(c));
        renderer.setData({y: 50, lineWidth: c.width, visible: false});
        assert.equal(renderer.hitTest(50, 50), null);
        continue;
    }
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
console.log(`${cases.length} geometry/price-line/pane arbitration cases match actual TypeScript.`);

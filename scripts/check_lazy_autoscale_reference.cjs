// Exercise the original PriceScale invalidation methods without a DOM or renderer.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const ts = require(require.resolve('typescript', { paths: [root] }));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root, 'src', file), 'utf8'), {
        compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020 },
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm, '').replace(/\bexport /g, '');
}
const PriceRange = new Function(`${source('model/price-range-impl.ts')}\nreturn PriceRangeImpl;`)();
const Autoscale = new Function('PriceRangeImpl', `${source('model/autoscale-info-impl.ts')}\nreturn AutoscaleInfoImpl;`)(PriceRange);
const Scale = new Function('PercentageFormatter', 'PriceFormatter', 'PriceRangeImpl', 'ensureNotNull',
    `${source('model/price-scale.ts')}\nreturn PriceScale;`)(class {}, class {}, PriceRange, v => {assert.notEqual(v, null);return v;});
const scale = Object.create(Scale.prototype);
Object.assign(scale, {
    _options: { autoScale: true, mode: 0, ensureEdgeTickMarksVisible: false },
    _isCustomPriceRange: false, _invalidatedForRange: { isValid: true, visibleBars: null },
    _priceRange: new PriceRange(0, 10), _marginAbove: 0, _marginBelow: 0, _marksCache: null,
});
let calls = 0, high = 30, nested = false;
scale._dataSources = [{ visible: () => true, firstValue: () => ({value: 1, timePoint: 0}),
    autoscaleInfo: () => {
        calls++;
        if (nested) assert.equal(scale.priceRange().maxValue(), 80);
        return new Autoscale(new PriceRange(0, high));
    },
}];
const interval = {left: () => 5, right: () => 10};
scale.recalculatePriceRange(interval);
scale.recalculatePriceRange(interval);
assert.equal(calls, 0);
high = 80;
assert.equal(scale.priceRange().maxValue(), 80);
assert.equal(scale.priceRange().maxValue(), 80);
assert.equal(calls, 1);
scale.recalculatePriceRange(interval);
nested = true;high = 90;
assert.equal(scale.priceRange().maxValue(), 90);
assert.equal(calls, 2);
scale.recalculatePriceRange(interval);
let otherCalls = 0;
scale._dataSources.push({visible: () => true, firstValue: () => ({value: 1, timePoint: 0}),
    autoscaleInfo: () => {otherCalls++;return new Autoscale(new PriceRange(0, 100));},
});
nested = false;
assert.equal(scale.priceRange().maxValue(), 100);
assert.equal(calls, 3);
assert.equal(otherCalls, 1);
console.log('Original PriceScale coalesces invalidations, queries current provider state and bounds recursive reads.');

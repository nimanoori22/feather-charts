const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const ts = require(require.resolve('typescript', {paths: [root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root, file), 'utf8'), {compilerOptions: {target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020}}).outputText
        .replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm, '').replace(/\bexport /g, '');
}
const toolkit = ['candles', 'crosshair-width', 'positions'].map(name => source(`packages/lwc-toolkit/src/dimensions/${name}.ts`)).join('\n');
const Renderer = new Function(`${toolkit}\n${source('plugin-examples/src/plugins/box-whisker-series/renderer.ts')}\nreturn WhiskerBoxSeriesRenderer;`)();
const View = new Function('WhiskerBoxSeriesRenderer', 'defaultOptions', `${source('plugin-examples/src/plugins/box-whisker-series/box-whisker-series.ts')}\nreturn WhiskerBoxSeries;`)(Renderer, {});
const view = new View();
assert.equal(view.isWhitespace({time: 1}), true);
const output = spawnSync(path.resolve('target/debug/examples/custom_series_reference'), {encoding: 'utf8', maxBuffer: 64*1024*1024});
if (output.status !== 0) throw new Error(output.stderr || 'Build custom_series_reference first');
const cases = JSON.parse(output.stdout);
for (const [index, fixture] of cases.entries()) {
    assert.deepEqual(view.priceValueBuilder({quartiles:fixture.q, outliers:[-10,50.5]}), fixture.prices, `case ${index} price contract`);
    const result = [];
    const context = {fillStyle: 0, save() {}, restore() {}, beginPath() {}, closePath() {}, fill() {},
        fillRect(x,y,w,h) { result.push([this.fillStyle, false, x,y,w,h]); },
        arc(x,y,r) { result.push([this.fillStyle, true, x,y,r,0]); }};
    const renderer = new Renderer();
    renderer.update({bars: [{x: fixture.x, originalData: {quartiles: fixture.q, outliers: [-10,50.5]}}], barSpacing: fixture.spacing, visibleRange: {from:0,to:1}},
        {whiskerColor:0, lowerQuartileFill:1, upperQuartileFill:2, outlierColor:3});
    renderer._drawImpl({context, horizontalPixelRatio:fixture.ratio, verticalPixelRatio:fixture.ratio}, price=>price);
    assert.equal(result.length, fixture.result.length, `case ${index} shape count`);
    result.forEach((shape, i)=>shape.forEach((value,j)=>assert.ok(typeof value==='number' ? Math.abs(value-fixture.result[i][j])<1e-8 : value===fixture.result[i][j], `case ${index} shape ${i} field ${j}: ${value} / ${fixture.result[i][j]}`)));
}
console.log(`${cases.length} box-whisker drawing cases match the original TypeScript plugin.`);

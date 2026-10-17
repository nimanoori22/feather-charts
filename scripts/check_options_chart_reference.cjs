// Execute the pinned upstream price behavior; native tests consume the same frozen cases.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {createHash} = require('node:crypto'), {spawnSync} = require('node:child_process');
const root = path.resolve('../lightweight-charts');
for (const [file, hash] of Object.entries(JSON.parse(fs.readFileSync('docs/porting/options-chart-source.json')).files))
    assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root, file))).digest('hex'), hash, file);
const ts = require(require.resolve('typescript', {paths: [root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root, 'src', file), 'utf8'), {
        compilerOptions: {target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020},
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm, '').replace(/\bexport /g, '');
}
const Behavior = new Function(source('model/horz-scale-behavior-price/horz-scale-behaviour-price.ts') + ';return HorzScaleBehaviorPrice;')();
const b = new Behavior(); b.setOptions({localization: {}});
assert.equal(b.formatHorzItem(1.5), '2');
b.updateFormatter({precision: 2, timeFormatter: () => 'ignored by tick behavior'});
assert.equal(b.formatTickmark({time: 1.5}, {}), '1.50');
for (const precision of [-1, 101]) { b.updateFormatter({precision}); assert.throws(() => b.formatHorzItem(1), RangeError); }
b.updateFormatter({precision: 2.9}); assert.equal(b.formatHorzItem(1), '1.00');
// Execute the real factory with an observed createChartEx boundary, not a fake DOM claim.
const factory = new Function('HorzScaleBehaviorPrice', 'createChartEx', source('api/create-options-chart.ts') + ';return createOptionsChart;')(Behavior, (container, behavior, options) => ({container, behavior, options}));
const options = {localization: {precision: 3}}, chart = factory('container', options);
assert.ok(chart.behavior instanceof Behavior); assert.equal(chart.options, options);
// Read real TimeScale methods without constructing a DOM/widget or substituting them.
const SourceScale = new Function('isInteger', source('model/time-scale.ts') + ';return TimeScale;')(Number.isInteger);
const isInteger = Number.isInteger;
const view = {_localizationOptions: {}, _horzScaleBehavior: b};
b.setOptions({localization: {precision: 2}});
assert.equal(SourceScale.prototype.formatDateTime.call(view, {time: 1.5, originalTime: 1.5}), '1.50');
view._localizationOptions.timeFormatter = v => 'Strike ' + v;
assert.equal(SourceScale.prototype.formatDateTime.call(view, {time: 1.5, originalTime: 1.5}), 'Strike 1.5');
view._width = 800; view._rightOffset = 0; view._barSpacing = 10;
view.isEmpty = () => false; view.baseIndex = () => 2;
const sourceCoordinates = [0, 1, 2].map(i => SourceScale.prototype.indexToCoordinate.call(view, i));
assert.equal(sourceCoordinates[1] - sourceCoordinates[0], sourceCoordinates[2] - sourceCoordinates[1]);
const ensureDefined = v => { assert.notEqual(v, undefined); return v; };
const lowerBound = new Function(source('helpers/algorithms.ts') + ';return lowerBound;')();
const creators = new Function('ensureDefined', source('model/data-consumer.ts') + source('model/get-series-plot-row-creator.ts') + ';return {getSeriesPlotRowCreator,isSeriesPlotRow};')(ensureDefined);
const DataLayer = new Function('lowerBound','ensureDefined','getSeriesPlotRowCreator','isSeriesPlotRow', source('model/data-layer.ts') + ';return DataLayer;')(lowerBound, ensureDefined, creators.getSeriesPlotRowCreator, creators.isSeriesPlotRow);
const layerBehavior = new Behavior(); layerBehavior.setOptions({localization: {precision: 0}});
const layer = new DataLayer(layerBehavior);
const owners = [0, 1].map(id => ({id, seriesType: () => 'Line', isConflationEnabled: () => false, customSeriesPlotValuesBuilder: () => undefined, customSeriesWhitespaceCheck: () => undefined}));
const states = []; let current = [];
function accept(name, update) {
    if (update.timeScale.points) current = update.timeScale.points;
    states.push(`${name}\t${current.map(p => p.time).join(',')}\t${current.map(p => p.timeWeight).join(',')}`);
}
accept('empty', layer.setSeriesData(owners[0], []));
accept('line', layer.setSeriesData(owners[0], [{time:90,value:1},{time:100,value:2},{time:150,value:3}]));
accept('union', layer.setSeriesData(owners[1], [{time:95,value:3},{time:100},{time:175,value:2}]));
accept('historical', layer.updateSeriesData(owners[0], {time:100,value:4}, true));
accept('append', layer.updateSeriesData(owners[0], {time:200,value:4}, false));
accept('whitespace', layer.updateSeriesData(owners[0], {time:200}, false));
accept('shorter', layer.setSeriesData(owners[0], [{time:90,value:1},{time:100,value:2}]));
accept('remove', layer.removeSeries(owners[1]));
accept('clear', layer.setSeriesData(owners[0], []));
const stateFixture = states.join('\n') + '\n', stateFile = 'docs/porting/options-chart-states.tsv';
if (process.argv.includes('--freeze')) fs.writeFileSync(stateFile, stateFixture);
else assert.equal(fs.readFileSync(stateFile, 'utf8'), stateFixture);
const cases = [];
function record(value, precision) {
    if (!Number.isFinite(value)) return;
    b.setOptions({localization: {precision}});
    const points = [{time: value, timeWeight: -1}]; b.fillWeightsForPoints(points, 0);
    const bytes = Buffer.alloc(8); bytes.writeDoubleBE(value === 0 ? 0 : value);
    const label = b.formatHorzItem(value);
    assert.equal(label, b.formatTickmark({time: value}, {}));
    assert.equal(b.key(b.convertHorzItemToInternal(value)), value);
    assert.equal(b.createConverterToInternalObj([])(value), value);
    cases.push(`${bytes.toString('hex')}\t${precision}\t${points[0].timeWeight}\t${label}`);
}
const samples = [-100, -50, -25, -10, -5, -2.5, -0.5, -0.001, -0, 0, 0.1, 0.25, 0.5, 1, 1.005, 1.25, 1.5, 2.5, 2.675, 5, 10, 25, 50, 100, 125, 1e-100, Number.MIN_VALUE, 1e20, 1e21, 1.2345678901234568e21, Number.MAX_VALUE];
for (let precision = 0; precision <= 100; precision++) for (const value of samples) record(value, precision);
// Include exact halfway values with both neighboring parities and decimal carry.
for (let n = -80; n <= 80; n++) for (let precision = 0; precision <= 8; precision++) record(n / 16, precision);
let bits = 0x123456789abcdef0n;
for (let n = 0; n < 2000; n++) {
    bits = BigInt.asUintN(64, bits * 6364136223846793005n + 1442695040888963407n);
    const bytes = Buffer.alloc(8); bytes.writeBigUInt64BE(bits);
    record(bytes.readDoubleBE(), n % 101);
}
const fixture = cases.join('\n') + '\n', file = 'docs/porting/options-chart-cases.tsv';
if (process.argv.includes('--freeze')) fs.writeFileSync(file, fixture);
else assert.equal(fs.readFileSync(file, 'utf8'), fixture, 'Source contract changed; investigate before re-freezing.');
if (!process.argv.includes('--source-only')) {
    const run = spawnSync('cargo', ['test', '--offline', '--lib', 'options_chart_source_reference'], {encoding: 'utf8'});
    assert.equal(run.status, 0, run.stderr + run.stdout); assert.ok(run.stdout.includes('1 passed'));
    const integration = spawnSync('cargo', ['test', '--offline', '--test', 'options_chart', 'price_data_layer_states_match_actual_source'], {encoding: 'utf8'});
    assert.equal(integration.status, 0, integration.stderr + integration.stdout); assert.ok(integration.stdout.includes('1 passed'));
}
console.log(`${cases.length} actual-source item/tick/weight cases; precision boundaries and factory delegation and 9 real DataLayer states pass.`);

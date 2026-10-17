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
}
console.log(`${cases.length} actual-source item/tick/weight cases; precision boundaries and factory delegation pass.`);

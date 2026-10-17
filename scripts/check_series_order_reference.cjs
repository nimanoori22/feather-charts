// Execute the local TypeScript source in an isolated browser, including GUI cleanup.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const root = path.resolve('../lightweight-charts');
const dependency = name => require(require.resolve(name, { paths: [root] }));
const hash = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
async function main() {
 const ts = dependency('typescript');
 const bundle = await dependency('rollup').rollup({
  input: path.join(root, 'src/index.ts'),
  plugins: [{ name: 'local-typescript',
   resolveId(id, importer) {
    if (importer && id.startsWith('.')) {
     const candidate = path.resolve(path.dirname(importer), id);
     if (fs.existsSync(candidate + '.ts')) return candidate + '.ts';
    }
   },
   transform(code, id) {
    if (!id.endsWith('.ts')) return null;
    return ts.transpileModule(code, { compilerOptions: {
     target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext,
    }}).outputText.replaceAll('process.env.NODE_ENV', '"development"')
      .replaceAll('process.env.BUILD_VERSION', '"5.2.1-local"');
   },
  }, dependency('@rollup/plugin-node-resolve').nodeResolve()],
 });
 const files = Object.fromEntries(bundle.watchFiles.filter(p => p.startsWith(root + '/src/')).sort()
  .map(p => [path.relative(root, p), hash(p)]));
 const manifest = 'docs/porting/series-order-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const { output } = await bundle.generate({ format: 'iife', name: 'LightweightCharts' });
 await bundle.close();
 fs.mkdirSync('target/series-order', { recursive: true });
 const browser = await dependency('puppeteer').launch({ executablePath: '/usr/bin/chromium', headless: true });
 try {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart" style="width:800px;height:500px"></div>');
  await page.addScriptTag({ content: output[0].code });
  const result = await page.evaluate(async () => {
   const L = LightweightCharts;
   const settle = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
   const rows = [{ time: 1, value: 2 }, { time: 2, value: 3 }];
   const make = () => L.createChart(document.getElementById('chart'), { width: 800, height: 500, layout: { attributionLogo: false } });
   const c = make();
   const series = [c.addSeries(L.AreaSeries), c.addSeries(L.LineSeries), c.addCustomSeries({
    defaultOptions: () => ({}), renderer: () => ({ draw() {} }), update() {},
    priceValueBuilder: d => [d.value], isWhitespace: d => d.value === undefined,
   })];
   series.forEach(s => s.setData(rows)); await settle();
   const states = [], alive = new Set(series);
   const snapshot = name => states.push({ name,
    order: c.panes().map(p => p.getSeries().map(s => series.indexOf(s))),
    indices: series.map(s => alive.has(s) ? s.seriesOrder() : null),
   });
   snapshot('initial'); series[2].setSeriesOrder(0); snapshot('custom_back');
   series[0].setSeriesOrder(99); snapshot('area_front');
   series[1].setSeriesOrder(1); snapshot('middle');
   series[0].setSeriesOrder(99); snapshot('same');
   c.addPane(); series[1].moveToPane(1); await settle(); snapshot('transfer');
   series[2].moveToPane(1); await settle(); snapshot('custom_transfer');
   series[2].setSeriesOrder(0); snapshot('target_back');
   c.removeSeries(series[1]); alive.delete(series[1]); snapshot('remove');
   const detachedOrder = series[1].seriesOrder(); series[1].setSeriesOrder(0);
   series.push(c.addSeries(L.LineSeries, {}, 1)); alive.add(series[3]); snapshot('append');
   c.remove();
   const y = L.createYieldCurveChart(document.getElementById('chart'), { width:800,height:500,layout:{attributionLogo:false} });
   const ya = y.addSeries(L.LineSeries), yb = y.addSeries(L.AreaSeries);
   const yieldOrders = [[ya.seriesOrder(),yb.seriesOrder()]];
   yb.setSeriesOrder(0); yieldOrders.push([ya.seriesOrder(),yb.seriesOrder()]);
   ya.setSeriesOrder(99); yieldOrders.push([ya.seriesOrder(),yb.seriesOrder()]);
   ya.moveToPane(1); await settle(); yieldOrders.push([ya.seriesOrder(),yb.seriesOrder()]);
   y.remove();
   const formats = {};
   for (const side of ['left','right','overlay']) {
    const c = make();
    const a = c.addSeries(L.LineSeries,{priceScaleId:side,priceFormat:{type:'custom',minMove:0.01,formatter:v=>'A'+v.toFixed(1)}});
    const b = c.addSeries(L.LineSeries,{priceScaleId:side,priceFormat:{type:'custom',minMove:0.01,formatter:v=>'B'+v.toFixed(2)}});
    a.setData(rows); b.setData(rows); await settle();
    const scale = a._series.priceScale();
    formats[side] = [scale.formatPriceAbsolute(12.5)];
    b.setSeriesOrder(0); await settle(); formats[side].push(scale.formatPriceAbsolute(12.5));
    a.applyOptions({visible:false}); b.applyOptions({visible:false});
    formats[side].push(scale.formatPriceAbsolute(12.5));
    c.remove();
   }
   return { states, yieldOrders, detachedOrder, formats };
  });
  fs.writeFileSync('target/series-order/source-results.json', JSON.stringify({ result, errors }, null, 2) + '\n');
  assert.deepEqual(errors, [], 'source browser errors');
  const fixture = 'docs/porting/series-order-states.json';
  if (process.argv.includes('--freeze')) fs.writeFileSync(fixture, JSON.stringify(result, null, 2) + '\n');
  else assert.deepEqual(result, JSON.parse(fs.readFileSync(fixture)), 'source behavior changed');
  if (!process.argv.includes('--source-only')) {
   const native = spawnSync('cargo', ['test','--offline','--test','series_order','source_reference_sequence','--','--nocapture'], { encoding:'utf8' });
   assert.equal(native.status,0,native.stdout+native.stderr);
   const rows = native.stdout.split('\n').filter(l=>l.startsWith('ORDER_REFERENCE ')).map(l=>JSON.parse(l.slice(16)));
   assert.deepEqual(rows, result.states);
  }
  console.log(JSON.stringify(result, null, 2));
 } finally { await browser.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; });

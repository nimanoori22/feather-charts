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
 const manifest = 'docs/porting/pane-management-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const { output } = await bundle.generate({ format: 'iife', name: 'LightweightCharts' });
 await bundle.close();
 fs.mkdirSync('target/pane-management', { recursive: true });
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
   const c = make(), series = [0, 1, 2].map(p => c.addSeries(L.LineSeries, {}, p));
   series.forEach(s => s.setData(rows)); await settle();
   const identity = c.panes(), states = [];
   const snapshot = name => states.push({ name,
    panes: c.panes().map(p => identity.indexOf(p)),
    series: series.map(s => c.panes().indexOf(s.getPane())),
    order: c.panes().map(p => p.getSeries().map(s => series.indexOf(s))),
   });
   snapshot('initial'); c.panes()[0].moveTo(2); await settle(); snapshot('move');
   c.swapPanes(0, 2); await settle(); snapshot('swap');
   series[1].moveToPane(0); await settle(); snapshot('transfer_cleanup');
   series[0].moveToPane(99); await settle(); snapshot('append_clamped');
   series[0].moveToPane(c.panes().length - 1); snapshot('same');
   await settle(); c.remove();
   // Capture the real reconciliation behavior rather than assuming splice destroys
   // the requested pane's owners. Keep observations separate from native policy.
   const removal = [];
   for (const index of [0, 1, 2]) {
    const c = make(), destroyed = [0, 0, 0], detached = [0, 0, 0];
    const series = [0, 1, 2].map(i => c.addCustomSeries({
     defaultOptions: () => ({}), renderer: () => ({ draw() {} }), update() {},
     priceValueBuilder: d => [d.value], isWhitespace: d => d.value === undefined,
     destroy() { destroyed[i]++; },
    }, {}, i));
    series.forEach((s, i) => { s.setData(rows); c.panes()[i].attachPrimitive({ detached() { detached[i]++; } }); });
    await settle(); c.removePane(index); await settle();
    const safe = f => { try { return f(); } catch (e) { return 'error:' + e.message; } };
    removal.push({ index, destroyed: destroyed.slice(), detached: detached.slice(),
     panes: c.panes().map(p => p.getSeries().map(s => series.indexOf(s))),
     data: series.map(s => safe(() => s.data().length)),
     membership: series.map(s => safe(() => c.panes().indexOf(s.getPane()))),
    });
    c.remove();
   }
   const last = make(); last.removePane(99); const finalPane = last.panes().length; last.remove();
   return { states, removal, finalPane };
  });
  fs.writeFileSync('target/pane-management/source-results.json', JSON.stringify({ result, errors }, null, 2) + '\n');
  assert.deepEqual(errors, [], 'source browser errors');
  const fixture = 'docs/porting/pane-management-states.json';
  if (process.argv.includes('--freeze')) fs.writeFileSync(fixture, JSON.stringify(result, null, 2) + '\n');
  else assert.deepEqual(result, JSON.parse(fs.readFileSync(fixture)), 'source behavior changed');
  if (!process.argv.includes('--source-only')) {
   const native = spawnSync('cargo', ['test','--offline','--test','pane_management','pane_source_reference_sequence','--','--nocapture'], { encoding:'utf8' });
   assert.equal(native.status,0,native.stdout+native.stderr);
   const rows = native.stdout.split('\n').filter(l=>l.startsWith('PANE_REFERENCE ')).map(l=>JSON.parse(l.slice(15)));
   assert.deepEqual(rows, result.states.map(({name,series,order})=>({name,series,order})));
  }
  console.log(JSON.stringify(result, null, 2));
 } finally { await browser.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; });

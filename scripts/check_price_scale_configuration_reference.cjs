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
 const manifest = 'docs/porting/price-scale-configuration-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const { output } = await bundle.generate({ format: 'iife', name: 'LightweightCharts' });
 await bundle.close();
 fs.mkdirSync('target/price-scale-configuration', { recursive: true });
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
   const make = () => L.createChart(document.getElementById('chart'), {width:800,height:500,layout:{attributionLogo:false}});
   const states=[];
   for (const mode of [0,1,2,3]) {
    const c=make(),s=c.addSeries(L.LineSeries);
    s.setData([{time:1,value:10},{time:2,value:20}]);c.timeScale().fitContent();await settle();
    const scale=c.priceScale('right');scale.applyOptions({mode});await settle();
    scale.setVisibleRange({from: mode===1?4:1,to:mode===1?5:9});await settle();
    const snap=name=>states.push({mode,name,auto:scale.options().autoScale,range:scale.getVisibleRange()});
    snap('manual');s.update({time:3,value:30});await settle();snap('update');
    scale.setAutoScale(true);await settle();
    states.push({mode,name:'restored',auto:scale.options().autoScale,finite:Object.values(scale.getVisibleRange()).every(Number.isFinite)});
    c.remove();
   }
   const c=make(); c.addPane();
   c.applyOptions({rightPriceScale:{invertScale:true},layout:{fontSize:17},grid:{vertLines:{visible:false}}});
   const opts=()=>({font:c.options().layout.fontSize,vertical:c.options().grid.vertLines.visible,invert:c.options().rightPriceScale.invertScale});
   const defaults=[opts()]; c.priceScale('right',1).applyOptions({invertScale:false});defaults.push(opts());c.addPane();await settle();c.remove();
   const t=make(),ts=t.addSeries(L.LineSeries);ts.setData([{time:1,value:10},{time:2,value:20}]);t.timeScale().fitContent();await settle();
   const ps=t.priceScale('right');ps.setVisibleRange({from:4,to:5});const transitions=[];
   for (const mode of [1,0,2,3]) {ps.applyOptions({mode});await settle();transitions.push({mode,auto:ps.options().autoScale,range:ps.getVisibleRange(),defaultAuto:t.options().rightPriceScale.autoScale});}
   t.remove();
   return {states,defaults,transitions};
  });
  fs.writeFileSync('target/price-scale-configuration/source-results.json', JSON.stringify({ result, errors }, null, 2) + '\n');
  assert.deepEqual(errors, [], 'source browser errors');
  const fixture = 'docs/porting/price-scale-configuration-states.json';
  if (process.argv.includes('--freeze')) fs.writeFileSync(fixture, JSON.stringify(result, null, 2) + '\n');
  else assert.deepEqual(result, JSON.parse(fs.readFileSync(fixture)), 'source behavior changed');

  if (!process.argv.includes('--source-only')) {
   const native=spawnSync('cargo',['test','--offline','--test','price_scale_configuration','source_manual_range_sequence','--','--nocapture'],{encoding:'utf8'});
   assert.equal(native.status,0,native.stdout+native.stderr);
   const rows=native.stdout.split('\n').filter(l=>l.startsWith('CONFIG_REFERENCE ')).map(l=>JSON.parse(l.slice(17)));
   assert.deepEqual(rows,result.states,'native/source range sequence');
   const transitions=spawnSync('cargo',['test','--offline','--test','price_scale_configuration','option_mode_transitions','--','--nocapture'],{encoding:'utf8'});
   assert.equal(transitions.status,0,transitions.stdout+transitions.stderr);
   const changes=transitions.stdout.split('\n').filter(l=>l.startsWith('TRANSITION_REFERENCE ')).map(l=>JSON.parse(l.slice(21)));
   assert.deepEqual(changes,result.transitions,'native/source manual mode transitions');
  }
  console.log(JSON.stringify(result, null, 2));
 } finally { await browser.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; });

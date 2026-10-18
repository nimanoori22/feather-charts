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
      .replaceAll('process.env.BUILD_VERSION', '"5.2.1-local"')
      .replaceAll('MouseEventButton.Left', '0').replaceAll('MouseEventButton.Middle', '1');
   },
  }, dependency('@rollup/plugin-node-resolve').nodeResolve()],
 });
 // transpileModule does not inline ambient const enums; include their source identity.
 bundle.watchFiles.push(path.join(root, 'src/typings/_global-types/index.d.ts'));
 const files = Object.fromEntries(bundle.watchFiles.filter(p => p.startsWith(root + '/src/')).sort()
  .map(p => [path.relative(root, p), hash(p)]));
 const manifest = 'docs/porting/public-events-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const { output } = await bundle.generate({ format: 'iife', name: 'LightweightCharts' });
 await bundle.close();
 fs.mkdirSync('target/public-events', { recursive: true });
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
   const c=make(), s=c.addSeries(L.LineSeries), events=[];
   s.subscribeDataChanged(scope=>events.push(scope));
   const rows=[{time:1,value:10},{time:2,value:20},{time:3,value:30}];
   s.setData(rows);s.setData(rows);s.update(rows[2]);s.update({time:1,value:12},true);
   s.pop(0);s.pop(1);s.setData([]);s.setData([]);s.pop(1);
   let rejected=false;try{s.setData([{time:2,value:1},{time:1,value:2}]);}catch{rejected=true;}
   s.setData(rows);c.timeScale().fitContent();await settle();
   const ranges=[];c.timeScale().subscribeVisibleTimeRangeChange(range=>ranges.push(['horizontal',range]));
   c.timeScale().subscribeVisibleLogicalRangeChange(range=>ranges.push(['logical',range]));
   for(const range of [{from:0,to:2},{from:0.1,to:2.1},{from:0.2,to:2.2},{from:1,to:3}]){
    c.timeScale().setVisibleLogicalRange(range);await settle();
   }
   const sizes=[];c.timeScale().subscribeSizeChange((w,h)=>sizes.push([w,h]));
   c.applyOptions({timeScale:{visible:false}});await settle();
   const hidden=[c.timeScale().width(),c.timeScale().height()];
   c.applyOptions({timeScale:{visible:true}});await settle();
   window.eventChart=c;window.pointerEvents=[];
   c.subscribeClick(()=>window.pointerEvents.push('click'));
   c.subscribeDblClick(()=>window.pointerEvents.push('double'));
   return {events,rejected,ranges,hidden,sizeCount:sizes.length,restored:sizes.at(-1).every(v=>v>0)};
  });
  await page.mouse.click(200,150);
  await page.mouse.click(200,150);
  result.clicks=await page.evaluate(()=>window.pointerEvents);
  await page.evaluate(()=>window.eventChart.remove());
  fs.writeFileSync('target/public-events/source-results.json', JSON.stringify({ result, errors }, null, 2) + '\n');
  assert.deepEqual(errors, [], 'source browser errors');
  const fixture = 'docs/porting/public-events-states.json';
  if (process.argv.includes('--freeze')) fs.writeFileSync(fixture, JSON.stringify(result, null, 2) + '\n');
  else assert.deepEqual(result, JSON.parse(fs.readFileSync(fixture)), 'source behavior changed');

  if (!process.argv.includes('--source-only')) {
   const native=spawnSync('cargo',['test','--offline','--test','public_events','--','--nocapture'],{encoding:'utf8'});
   assert.equal(native.status,0,native.stdout+native.stderr);
   const row=native.stdout.split('\n').find(l=>l.startsWith('EVENT_REFERENCE '));
   assert.deepEqual(JSON.parse(row.slice(16)),result.events,'native/source data sequence');
   const ranges=native.stdout.split('\n').find(l=>l.startsWith('RANGES_REFERENCE '));
   const normalize=value=>JSON.parse(JSON.stringify(value,(_,v)=>typeof v==='number'?Math.round(v*1e9)/1e9:v));
   assert.deepEqual(normalize(JSON.parse(ranges.slice(17))),normalize(result.ranges),'native/source range causes, order and original endpoints');
   assert.deepEqual(result.clicks,['click','double']);
   assert.deepEqual(result.hidden,[0,0]);assert.equal(result.sizeCount,2);assert.equal(result.restored,true);
  }
  console.log(JSON.stringify(result, null, 2));
 } finally { await browser.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; });

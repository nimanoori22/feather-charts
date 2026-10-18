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
 const manifest = 'docs/porting/series-data-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const { output } = await bundle.generate({ format: 'iife', name: 'LightweightCharts' });
 await bundle.close();
 fs.mkdirSync('target/series-data', { recursive: true });
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
   const c=make(), a=c.addSeries(L.LineSeries), b=c.addSeries(L.LineSeries);
   a.setData([{time:1,value:10},{time:2},{time:3,value:30},{time:4}]);
   b.setData([{time:3,value:99},{time:5,value:50}]);
   const pair=d=>[d.time,d.value];
   const ranges=[null,[-1,6],[0.25,1.25],[1,1],[3,3],[20,30]];
   const states=[];
   const snapshot=(name,popped=[])=>states.push({name,data:a.data().map(pair),peer:b.data().map(pair),popped:popped.map(pair),
    queries:[-1,0,0.5,1,2,3,4,5].map(i=>[i,[0,-1,1].map(dir=>a.dataByIndex(i,dir)?.value??null)]),
    ranges:ranges.map(r=>{const v=a.barsInLogicalRange(r&&{from:r[0],to:r[1]});return v?{before:v.barsBefore,after:v.barsAfter,range:v.from===undefined?null:[v.from,v.to]}:null;})
   });
   snapshot('initial');snapshot('zero',a.pop(0));snapshot('shared',a.pop(1));
   snapshot('unique',a.pop(99));snapshot('empty',a.pop(1));
   a.update({time:3,value:33});snapshot('append');
   await settle();c.remove();
   const kinds={};
   const values={Line:{value:12,color:'red'},Histogram:{value:12,color:'red'},Area:{value:12,lineColor:'red',topColor:'blue',bottomColor:'green'},Baseline:{value:12,topLineColor:'red',bottomLineColor:'blue',topFillColor1:'green',topFillColor2:'pink',bottomFillColor1:'cyan',bottomFillColor2:'black'},Bar:{open:10,high:15,low:9,close:12,color:'red'},Candlestick:{open:10,high:15,low:9,close:12,color:'red',borderColor:'blue',wickColor:'green'}};
   for (const [kind,fields] of Object.entries(values)) {
    const c=make(),s=c.addSeries(L[kind+'Series']);
    s.setData([{time:'2024-01-01',...fields,customValues:{tag:kind}},{time:'2024-01-02'}]);
    kinds[kind]={data:s.data(),popped:s.pop(5),empty:s.data()};await settle();c.remove();
   }
   const custom=make(),s=custom.addCustomSeries({defaultOptions:()=>({}),renderer:()=>({draw(){}}),update(){},priceValueBuilder:d=>[d.value],isWhitespace:d=>d.value===undefined});
   s.setData([{time:1,value:5,label:'a'},{time:2},{time:3,value:7,label:'b'}]);
   const customResult={data:s.data(),popped:s.pop(1),remaining:s.data()};await settle();custom.remove();
   const y=L.createYieldCurveChart(document.getElementById('chart'),{width:800,height:500});
   const ys=y.addSeries(L.LineSeries);ys.setData([{time:12,value:3},{time:120,value:4}]);
   const yieldResult={popped:ys.pop(99),data:ys.data()};ys.update({time:60,value:5});yieldResult.append=ys.data();await settle();y.remove();
   return {states,kinds,custom:customResult,yield:yieldResult};
  });
  fs.writeFileSync('target/series-data/source-results.json', JSON.stringify({ result, errors }, null, 2) + '\n');
  assert.deepEqual(errors, [], 'source browser errors');
  const fixture = 'docs/porting/series-data-states.json';
  if (process.argv.includes('--freeze')) fs.writeFileSync(fixture, JSON.stringify(result, null, 2) + '\n');
  else assert.deepEqual(result, JSON.parse(fs.readFileSync(fixture)), 'source behavior changed');
  if (!process.argv.includes('--source-only')) {
   const native = spawnSync('cargo', ['test','--offline','--test','series_data','source_reference_sequence','--','--nocapture'], { encoding:'utf8' });
   assert.equal(native.status,0,native.stdout+native.stderr);
   const rows = native.stdout.split('\n').filter(l=>l.startsWith('DATA_REFERENCE ')).map(l=>JSON.parse(l.slice(15)));
   assert.deepEqual(rows, result.states);
  }
  console.log(JSON.stringify(result, null, 2));
 } finally { await browser.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; });

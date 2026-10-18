// Read-only source/native observations. Differences are findings, never accepted baselines.
const fs = require('node:fs'), path = require('node:path');
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const root = path.resolve('../lightweight-charts');
const dep = name => require(require.resolve(name, { paths: [root] }));
async function main() {
 const ts = dep('typescript');
 const bundle = await dep('rollup').rollup({input: 'audit-entry', plugins: [{
  name: 'audit-source',
  resolveId(id, importer) {
   if (id === 'audit-entry') return '\0audit';
   if (importer && id.startsWith('.')) {
    const file = path.resolve(path.dirname(importer), id) + '.ts';
    if (fs.existsSync(file)) return file;
   }
  },
  load(id) {
   if (id === '\0audit') return `export * from ${JSON.stringify(root+'/src/index.ts')}; export {defaultTickMarkFormatter} from ${JSON.stringify(root+'/src/model/horz-scale-behavior-time/default-tick-mark-formatter.ts')};`;
  },
  transform(code, id) {
   if (!id.endsWith('.ts')) return null;
   return ts.transpileModule(code, {compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ESNext}}).outputText
    .replaceAll('process.env.NODE_ENV','"development"').replaceAll('process.env.BUILD_VERSION','"5.2.1-local"')
    .replaceAll('MouseEventButton.Left','0').replaceAll('MouseEventButton.Middle','1');
  }
 }, dep('@rollup/plugin-node-resolve').nodeResolve()]});
 const hashes = Object.fromEntries(bundle.watchFiles.filter(p=>p.startsWith(root+'/src/')).sort()
  .map(p=>[path.relative(root,p),createHash('sha256').update(fs.readFileSync(p)).digest('hex')]));
 const {output} = await bundle.generate({format:'iife',name:'L'}); await bundle.close();


 const dir='target/public-helpers';fs.mkdirSync(dir,{recursive:true});
 const browser=await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let rows,errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  rows=await page.evaluate(async()=>{
   const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const c=L.createChart(document.getElementById('chart'),{width:800,height:500,timeScale:{barSpacing:9,rightOffset:3},crosshair:{mode:0}});
   const s=c.addSeries(L.LineSeries);s.setData(Array.from({length:100},(_,i)=>({time:i+1,value:101+i})));
   c.timeScale().setVisibleRange({from:20,to:40});await settle();
   const range=c.timeScale().getVisibleRange(),logical=c.timeScale().getVisibleLogicalRange();
   const global=s.lastValueData(true).price,visible=s.lastValueData(false).price;
   s.createPriceLine({price:120});s.createPriceLine({price:150});
   c.timeScale().fitContent();await settle();const roundtrips=[];
   for(const mode of [0,1,2,3]) {s.priceScale().applyOptions({mode});await settle();roundtrips.push(Math.round(s.coordinateToPrice(s.priceToCoordinate(150))));}
   let events=0;c.subscribeCrosshairMove(()=>events++);
   c.setCrosshairPosition(150,50,s);const index=c._chartWidget.model().crosshairSource().appliedIndex();c.clearCrosshairPosition();
   c.timeScale().setVisibleRange({from:20,to:40});await settle();c.setCrosshairPosition(150,99,s);
   const outside=[c._chartWidget.model().crosshairSource().appliedIndex()];
   c.timeScale().setVisibleRange({from:50,to:70});await settle();outside.push(c._chartWidget.model().crosshairSource().appliedIndex());
   const result={outside,range:[range.from,range.to],logical:[Math.round(logical.from),Math.round(logical.to)],global,visible,roundtrips,index,events,lines:s.priceLines().length,nearest:[c.timeScale().timeToIndex(0,true),c.timeScale().timeToIndex(101,true)],missing:c.timeScale().timeToIndex(0,false)};
   c.remove();return result;
  });
 } finally {await browser.close();}
 assert.deepEqual(errors,[]);
 const fixture='docs/porting/public-helpers-states.json',manifest='docs/porting/public-helpers-source.json';
 if(process.argv.includes('--freeze')){fs.writeFileSync(fixture,JSON.stringify(rows,null,2)+'\n');fs.writeFileSync(manifest,JSON.stringify({files:hashes},null,2)+'\n');}
 else {assert.deepEqual(rows,JSON.parse(fs.readFileSync(fixture)));assert.deepEqual(hashes,JSON.parse(fs.readFileSync(manifest)).files);}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({rows,errors},null,2)+'\n');
 if(!process.argv.includes('--source-only')){
  const native=spawnSync('cargo',['test','--offline','--test','public_helpers','source_public_helper_states','--','--nocapture'],{encoding:'utf8'});
  fs.writeFileSync(dir+'/native.log',native.stdout+native.stderr);assert.equal(native.status,0,native.stdout+native.stderr);
  const line=native.stdout.split('\n').find(l=>l.startsWith('HELPER_REFERENCE '));assert.ok(line,'Missing native observations');
  assert.deepEqual(JSON.parse(line.slice(17)),rows,'native/source public helper states differ');
 }
 console.log('Public ranges, nearest indices, last values, conversions, annotation enumeration and silent synthetic crosshair match source.');
}
main().catch(e=>{console.error(e);process.exitCode=1;});

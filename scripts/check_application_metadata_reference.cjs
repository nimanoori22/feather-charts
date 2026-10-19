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


 const dir='target/application-metadata';fs.mkdirSync(dir,{recursive:true});
 const browser=await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let rows,errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  rows=await page.evaluate(async()=>{
   const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const rows=[];
   for(const [family,definition] of ['Line','Histogram','Area','Baseline','Bar','Candlestick'].map(n=>[n,L[n+'Series']])) {
    const c=L.createChart(document.getElementById('chart'),{width:800,height:500});
    const s=c.addSeries(definition),peer=c.addSeries(L.LineSeries);
    const a={id:2,count:0},b={id:4,count:0};
    const item=(time,customValues)=>({time,value:10,open:9,high:11,low:8,close:10,...(customValues===undefined?{}:{customValues})});
    s.setData([item(2,a),item(4,b)]);peer.setData([item(1),item(3)]);await settle();
    const retained=s.data(), ids=()=>s.data().map(r=>r.customValues?.id??0);
    const initial=ids(),same=s.dataByIndex(1).customValues===a;
    const modelSeries=c._seriesMap.get(s),plot=modelSeries.bars().rows()[0];
    const pointer=c._convertMouseParams({seriesData:new Map([[modelSeries,plot]]),originalTime:2,index:1,point:{x:100,y:100},paneIndex:0});
    a.count=3;const shared=pointer.seriesData.get(s).customValues.count===3;
    let rejected=false;try{s.update(item(1.5,{id:99}),true)}catch{rejected=true}
    s.update(item(2,{id:20}),true);const history=ids();
    s.update(item(4,{id:40}));const latest=ids();
    s.update(item(4));const cleared=ids();
    s.update({time:4,customValues:{id:88}});const whitespace=ids();
    s.update(item(4));const restored=ids();
    s.update(item(4,{id:44}));const popped=s.pop(2).map(r=>r.customValues?.id??0);
    rows.push({family,initial,same,shared,rejected,history,latest,cleared,whitespace,restored,popped,retained:retained.map(r=>r.customValues.id),empty:s.data().length===0});c.remove();
   }
   return rows;
  });
 } finally {await browser.close();}
 assert.deepEqual(errors,[]);
 const fixture='docs/porting/application-metadata-states.json',manifest='docs/porting/application-metadata-source.json';
 if(process.argv.includes('--freeze')){fs.writeFileSync(fixture,JSON.stringify(rows,null,2)+'\n');fs.writeFileSync(manifest,JSON.stringify({files:hashes},null,2)+'\n');}
 else {assert.deepEqual(rows,JSON.parse(fs.readFileSync(fixture)));assert.deepEqual(hashes,JSON.parse(fs.readFileSync(manifest)).files);}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({rows,errors},null,2)+'\n');
 if(!process.argv.includes('--source-only')){
  const native=spawnSync('cargo',['test','--offline','--test','application_metadata','source_application_metadata_states','--','--nocapture'],{encoding:'utf8'});
  fs.writeFileSync(dir+'/native.log',native.stdout+native.stderr);assert.equal(native.status,0,native.stdout+native.stderr);
  const line=native.stdout.split('\n').find(l=>l.startsWith('METADATA_REFERENCE '));assert.ok(line,'Missing native observations');
  assert.deepEqual(JSON.parse(line.slice(19)),rows,'native/source application metadata states differ');
 }
 console.log('Application metadata identity, updates, clearing, whitespace, pop and pointer conversion match source.');
}
main().catch(e=>{console.error(e);process.exitCode=1;});

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

 const dir='target/scale-assignment';fs.mkdirSync(dir,{recursive:true});
 const browser=await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let rows,errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  rows=await page.evaluate(async()=>{
   const rows=[];const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const data=base=>[1,2,3].map(time=>({time,value:base+time*10,low:base+time*10-1,high:base+time*10+1}));
   for(const custom of [false,true]){
    const c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false}});c.addPane();
    const view={renderer:()=>({draw(){}}),update(){},priceValueBuilder:d=>[d.low,d.high,d.value],isWhitespace:d=>d.value===undefined,defaultOptions:()=>({color:'blue'})};
    const s=custom?c.addCustomSeries(view,{priceScaleId:'right'}):c.addSeries(L.LineSeries,{priceScaleId:'right'});
    const peer=c.addSeries(L.LineSeries,{priceScaleId:'right'});peer.setData(data(40));
    const shared=c.addSeries(L.LineSeries,{priceScaleId:'shared'});shared.setData(data(90));
    s.setData(data(0));s.setSeriesOrder(1);c.timeScale().fitContent();await settle();
    shared.priceScale().setVisibleRange({from:0,to:200});shared.priceScale().setAutoScale(false);await settle();
    for(const [stage,target] of [['initial',null],['left','left'],['same-left','left'],['shared','shared'],['other','other'],['empty',''],['right','right'],['omitted',null],['new-pane',null],['left-new-pane','left']]){
     const before=s._series.priceScale().id();
     if(stage==='new-pane')s.moveToPane(1);else if(stage!=='initial')s.applyOptions({priceScaleId:target===null?undefined:target});
     await settle();const pane=s._series.model().paneForSource(s._series);
     rows.push({custom,stage,position:s._series.priceScale().id(),option:s.options().priceScaleId,order:s.seriesOrder(),pane:s.getPane().paneIndex(),points:s.data().length,oldOverlayAlive:['left','right'].includes(before)?null:pane.priceScaleById(before)!==null,sharedAuto:pane.priceScaleById('shared')?.isAutoScale()??null});
    }
    c.remove();
   }
   return rows;
  });
 } finally {await browser.close();}
 assert.deepEqual(errors,[]);assert.equal(rows.length,20);
 const fixture='docs/porting/scale-assignment-states.json',manifest='docs/porting/scale-assignment-source.json';
 if(process.argv.includes('--freeze')){fs.writeFileSync(fixture,JSON.stringify(rows,null,2)+'\n');fs.writeFileSync(manifest,JSON.stringify({files:hashes},null,2)+'\n');}
 else {assert.deepEqual(rows,JSON.parse(fs.readFileSync(fixture)));assert.deepEqual(hashes,JSON.parse(fs.readFileSync(manifest)).files);}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({rows,errors},null,2)+'\n');
 if(!process.argv.includes('--source-only')){
  const native=spawnSync('cargo',['test','--offline','--test','scale_assignment','source_scale_assignment_matrix','--','--nocapture'],{encoding:'utf8'});
  fs.writeFileSync(dir+'/native.log',native.stdout+native.stderr);assert.equal(native.status,0,native.stdout+native.stderr);
  const actual=native.stdout.split('\n').filter(l=>l.startsWith('SCALE_REFERENCE ')).map(l=>JSON.parse(l.slice(16)));
  assert.deepEqual(actual,rows,'native/source scale assignment differs');
 }
 console.log('20 source scale assignment states verified.');
}
main().catch(e=>{console.error(e);process.exitCode=1;});

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
 const dir='target/public-parity/formatting';fs.mkdirSync(dir,{recursive:true});
 const browser=await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let result,errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  result=await page.evaluate(async()=>{
   const rows=[];const masks=[0,1,2,4,6,8,16,24,31];
   const batch=prefix=>values=>values.map((_,i)=>`${prefix}${i}/${values.length}`);
   const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   for(const custom of [false,true]) for(const mode of [0,1,2,3]) for(const mask of masks){
    const localization={};
    if(mask&2)localization.priceFormatter=()=> 'global-price';
    if(mask&4)localization.tickmarksPriceFormatter=batch('global-batch');
    if(mask&8)localization.percentageFormatter=()=> 'global-percent';
    if(mask&16)localization.tickmarksPercentageFormatter=batch('percent-batch');
    const c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false},localization});
    const options={priceFormat:{type:'custom',formatter:()=> 'series',minMove:0.01}};
    if(mask&1)options.priceFormat.tickmarksFormatter=batch('series-batch');
    const view={renderer:()=>({draw(){}}),update(){},priceValueBuilder:d=>[d.low,d.high,d.value],isWhitespace:d=>d.value===undefined,defaultOptions:()=>({color:'blue'})};
    const s=custom?c.addCustomSeries(view,options):c.addSeries(L.LineSeries,options);
    s.setData([{time:1,value:10,low:9,high:11},{time:2,value:20,low:19,high:21},{time:3,value:30,low:29,high:31}]);
    s.priceScale().applyOptions({mode});c.timeScale().fitContent();await settle();
    const scale=s._series.priceScale();
    rows.push({custom,mode,mask,scalar:scale.formatLogical(20),batch:scale.formatLogicalTickmarks([10,20,30]),absolute:scale.formatPriceAbsolute(20)});
    c.remove();
   }
   const c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false},localization:{priceFormatter:()=> 'retained'}});
   c.applyOptions({localization:{priceFormatter:undefined}});
   const s=c.addSeries(L.LineSeries);s.setData([{time:1,value:1}]);c.timeScale().fitContent();await settle();
   const undefinedClears=s._series.priceScale().formatPriceAbsolute(1)!=='retained';c.remove();
   return {rows,undefinedClears};
  });
 } finally {await browser.close();}
 assert.deepEqual(errors,[]);assert.equal(result.rows.length,72);assert.equal(result.undefinedClears,false);
 const fixture='docs/porting/formatting-states.json',manifest='docs/porting/formatting-source.json';
 if(process.argv.includes('--freeze')){fs.writeFileSync(fixture,JSON.stringify(result,null,2)+'\n');fs.writeFileSync(manifest,JSON.stringify({files:hashes},null,2)+'\n');}
 else {assert.deepEqual(result,JSON.parse(fs.readFileSync(fixture)));assert.deepEqual(hashes,JSON.parse(fs.readFileSync(manifest)).files);}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({result,errors},null,2)+'\n');
 if(!process.argv.includes('--source-only')){
  const native=spawnSync('cargo',['test','--offline','--test','formatting_parity','source_formatting_matrix','--','--nocapture'],{encoding:'utf8'});
  fs.writeFileSync(dir+'/native.log',native.stdout+native.stderr);assert.equal(native.status,0,native.stdout+native.stderr);
  const actual=native.stdout.split('\n').filter(l=>l.startsWith('FORMATTING_REFERENCE ')).map(l=>JSON.parse(l.slice(21)));
  assert.deepEqual(actual,result.rows,'native/source formatting precedence differs');
 }
 console.log('72 source formatting cases verified; JS undefined does not clear an existing formatter.');
}
main().catch(e=>{console.error(e);process.exitCode=1;});

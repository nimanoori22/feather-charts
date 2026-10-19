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



 const dir='target/primary-axis';fs.mkdirSync(dir,{recursive:true});
 const browser=await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let rows,errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  rows=await page.evaluate(()=>{
   const c=L.createChart(document.getElementById('chart'),{width:800,height:500,leftPriceScale:{visible:true},layout:{attributionLogo:false}});
   const left=c.addSeries(L.LineSeries,{priceScaleId:'left'}),right=c.addSeries(L.LineSeries,{priceScaleId:'right'});
   left.setData([{time:1,value:10},{time:2,value:20}]);right.setData([{time:1,value:100},{time:2,value:200}]);
   const model=c._chartWidget.model(),rows=[];
   const emit=(name,index)=>{const pane=model.panes()[index];rows.push({name,preference:c.options().defaultVisiblePriceScaleId,primary:pane.defaultPriceScale().id(),visible:pane.defaultVisiblePriceScale()?.id()??'none'});};
   emit('initial',0);c.applyOptions({defaultVisiblePriceScaleId:'left'});emit('left',0);
   c.addPane();emit('new-empty',1);c.swapPanes(0,1);emit('moved',1);
   c.applyOptions({defaultVisiblePriceScaleId:'right'});emit('right',1);emit('right-empty',0);
   c.priceScale('right',1).applyOptions({visible:false});emit('right-hidden',1);
   c.priceScale('left',1).applyOptions({visible:false});emit('both-hidden',1);
   c.removeSeries(left);emit('only-right-hidden',1);c.removeSeries(right);emit('empty-fallback',0);
   c.remove();return rows;
  });
 } finally {await browser.close();}
 assert.deepEqual(errors,[]);
 const fixture='docs/porting/runtime-primary-axis-states.json',manifest='docs/porting/runtime-primary-axis-source.json';
 if(process.argv.includes('--freeze')){fs.writeFileSync(fixture,JSON.stringify(rows,null,2)+'\n');fs.writeFileSync(manifest,JSON.stringify({files:hashes},null,2)+'\n');}
 else {assert.deepEqual(rows,JSON.parse(fs.readFileSync(fixture)));assert.deepEqual(hashes,JSON.parse(fs.readFileSync(manifest)).files);}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({rows,errors},null,2)+'\n');
 if(!process.argv.includes('--source-only')){
  const native=spawnSync('cargo',['test','--offline','--test','runtime_primary_axis','source_primary_axis_states','--','--nocapture'],{encoding:'utf8'});
  fs.writeFileSync(dir+'/native.log',native.stdout+native.stderr);assert.equal(native.status,0,native.stdout+native.stderr);
  const line=native.stdout.split('\n').find(l=>l.startsWith('PRIMARY_REFERENCE '));assert.ok(line,'Missing native observations');
  assert.deepEqual(JSON.parse(line.slice('PRIMARY_REFERENCE '.length)),rows,'native/source primary-axis states differ');
 }
 console.log('Runtime primary-axis preference and pane fallbacks match source.');
}
main().catch(e=>{console.error(e);process.exitCode=1;});

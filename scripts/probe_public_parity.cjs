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
 const dir = 'target/parity-audit'; fs.mkdirSync(dir,{recursive:true});
 const browser = await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let source, errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>'); await page.addScriptTag({content:output[0].code});
  source = await page.evaluate(async()=>{
   const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const make=extra=>L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false},...extra});
   const c=make({localization:{priceFormatter:()=> 'GLOBAL',tickmarksPriceFormatter:vs=>vs.map(()=> 'GLOBAL_TICKS'),percentageFormatter:()=> 'PERCENT'}});
   const s=c.addSeries(L.LineSeries);s.setData([{time:1,value:10},{time:2,value:20}]);c.timeScale().fitContent();await settle();
   // Inspect actual internal scale formatters behind the public configuration.
   const scale=s._series.priceScale();
   const result={global_price:scale.formatPriceAbsolute(20),global_ticks:scale.formatLogicalTickmarks([20])};
   s.priceScale().applyOptions({mode:2});await settle();result.global_percentage=scale.formatPrice(20,10);
   let calls=0;s.applyOptions({priceScaleId:'left',priceFormat:{type:'custom',formatter:()=> 'SINGLE',tickmarksFormatter:vs=>{calls++;return vs.map(()=> 'BATCH');},minMove:0.01}});
   result.custom_ticks=s.priceFormatter().formatTickmarks([20]);result.custom_batch_calls=calls;result.scale_after_option=s._series.priceScale().id();c.remove();
   const point={timestamp:1704196800};
   result.arabic_day=L.defaultTickMarkFormatter(point,L.TickMarkType.DayOfMonth,'ar-EG');
   result.arabic_time=L.defaultTickMarkFormatter(point,L.TickMarkType.Time,'ar-EG');
   result.german_month=L.defaultTickMarkFormatter(point,L.TickMarkType.Month,'de-DE');
   let parserCalls=0;const token='color(display-p3 1 0 0)';
   const p=make({layout:{attributionLogo:false,textColor:token,colorParsers:[v=>{if(v!==token)return null;parserCalls++;return [255,0,0,1];}]}});
   const gray=p._chartWidget.model().colorParser().colorStringToGrayscale(token);
   await settle();p.remove();result.configured_color_parser=parserCalls>0&&Math.abs(gray-50.745)<1e-8;
   return result;
  });
  const position=await page.evaluate(async()=>{
   const c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false}});
   const s=c.addSeries(L.LineSeries);s.setData(Array.from({length:20},(_,i)=>({time:i+1,value:i+1})));c.timeScale().fitContent();
   await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   window.__auditChart=c;window.__auditSpacing=c.timeScale().options().barSpacing;
   const rect=document.getElementById('chart').getBoundingClientRect();
   return {x:rect.left+c.timeScale().width()*0.6,y:rect.top+c.paneSize().height+c.timeScale().height()*0.5};
  });
  await page.mouse.move(position.x,position.y);await page.mouse.down();await page.mouse.move(position.x-100,position.y,{steps:8});await page.mouse.up();
  source.time_axis_drag=await page.evaluate(()=>window.__auditChart.timeScale().options().barSpacing!==window.__auditSpacing);
  await page.mouse.click(position.x,position.y);
  await page.mouse.click(position.x,position.y);
  await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  source.time_axis_double_reset=await page.evaluate(()=>{const c=window.__auditChart;const reset=c.timeScale().options().barSpacing===6;c.remove();return reset;});
 } finally {await browser.close();}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({source,errors,hashes},null,2)+'\n');
 assert.deepEqual(errors,[]);
 assert.equal(source.global_price,'GLOBAL'); assert.deepEqual(source.global_ticks,['GLOBAL_TICKS']);
 assert.equal(source.global_percentage,'PERCENT'); assert.deepEqual(source.custom_ticks,['BATCH']);
 assert.equal(source.custom_batch_calls,1);assert.equal(source.scale_after_option,'left');assert.equal(source.configured_color_parser,true);
 assert.equal(source.time_axis_drag,true);assert.equal(source.time_axis_double_reset,true);
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({source,errors,hashes},null,2)+'\n');
 const run=spawnSync('cargo',['run','--offline','--example','parity_audit'],{encoding:'utf8'});
 fs.writeFileSync(dir+'/native.log',run.stdout+run.stderr);assert.equal(run.status,0,run.stderr);
 const native=Object.fromEntries(run.stdout.split('\n').filter(l=>l.startsWith('AUDIT ')).map(l=>{
  const split=l.indexOf(' ',6);return [l.slice(6,split),JSON.parse(l.slice(split+1))];
 }));
 assert.deepEqual(Object.keys(native).sort(),Object.keys(source).sort());
 const comparisons=Object.entries(source).map(([name,value])=>({name,source:value,native:native[name],equal:JSON.stringify(value)===JSON.stringify(native[name])}));
 fs.writeFileSync(dir+'/comparisons.json',JSON.stringify(comparisons,null,2)+'\n');
 console.log(JSON.stringify(comparisons,null,2));
}
main().catch(e=>{console.error(e);process.exitCode=1;});

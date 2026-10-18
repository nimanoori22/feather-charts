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

 const dir='target/axis-interaction';fs.mkdirSync(dir,{recursive:true});
 const browser=await dep('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 let rows,errors=[];
 try {
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  rows=await page.evaluate(async()=>{
   const rows=[],settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   // The source ignores compatibility mouse input in the first 500ms after a touch timestamp (initially zero).
   if(performance.now()<600)await new Promise(r=>setTimeout(r,600-performance.now()));
   for(let mask=0;mask<16;mask++){
    const flags={timeDrag:!!(mask&1),timeReset:!!(mask&2),priceDrag:!!(mask&4),priceReset:!!(mask&8)};
    const c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false},timeScale:{barSpacing:9,rightOffset:3},handleScale:{axisPressedMouseMove:{time:flags.timeDrag,price:flags.priceDrag},axisDoubleClickReset:{time:flags.timeReset,price:flags.priceReset}}});
    const s=c.addSeries(L.LineSeries);s.setData(Array.from({length:250},(_,i)=>({time:i+1,value:100+(i%10)})));await settle();
    const model=s._series.model(),widget=c._chartWidget._timeAxisWidget;
    const time=widget._topCanvasBinding.canvasElement;
    const price=c._chartWidget._paneWidgets[0]._rightPriceAxisWidget._topCanvasBinding.canvasElement;
    function send(target,type,x,y){target.dispatchEvent(new MouseEvent(type,{bubbles:true,button:0,buttons:type==='mouseup'?0:1,clientX:x,clientY:y,view:window}));}
    function drag(target,horizontal){const r=target.getBoundingClientRect();const x=r.x+r.width*(horizontal?0.6:0.5),y=r.y+r.height*0.5;send(target,'mousedown',x,y);send(document.documentElement,'mousemove',x-(horizontal?r.width*0.2:0),y+(horizontal?0:r.height*0.1));send(document.documentElement,'mouseup',x-(horizontal?r.width*0.2:0),y+(horizontal?0:r.height*0.1));}
    function twice(target){const r=target.getBoundingClientRect(),x=r.x+r.width*0.6,y=r.y+r.height*0.5;for(let i=0;i<2;i++){send(target,'mousedown',x,y);send(document.documentElement,'mouseup',x,y);}}
    drag(time,true);await settle();const scaled=model.timeScale().barSpacing()!==9;if(scaled!==flags.timeDrag)throw new Error(`Unexpected source drag for mask ${mask}`);
    // Change the viewport without changing configured defaults.
    model.timeScale().setBarSpacing(18);model.timeScale().setRightOffset(8);model.lightUpdate();await settle();twice(time);await settle();
    const spacing=model.timeScale().barSpacing(),offset=model.timeScale().rightOffset();
    const scale=s.priceScale();scale.setAutoScale(true);await settle();drag(price,false);await settle();const manual=!scale.options().autoScale;
    scale.setVisibleRange({from:100,to:110});scale.setAutoScale(false);await settle();twice(price);await settle();
    rows.push({mask,scaled,spacing,offset,manual,autoAfterReset:scale.options().autoScale});c.remove();
   }return rows;
  });
 } finally {await browser.close();}
 assert.deepEqual(errors,[]);assert.equal(rows.length,16);
 const fixture='docs/porting/axis-interaction-states.json',manifest='docs/porting/axis-interaction-source.json';
 if(process.argv.includes('--freeze')){fs.writeFileSync(fixture,JSON.stringify(rows,null,2)+'\n');fs.writeFileSync(manifest,JSON.stringify({files:hashes},null,2)+'\n');}
 else {assert.deepEqual(rows,JSON.parse(fs.readFileSync(fixture)));assert.deepEqual(hashes,JSON.parse(fs.readFileSync(manifest)).files);}
 fs.writeFileSync(dir+'/source-results.json',JSON.stringify({rows,errors},null,2)+'\n');
 if(!process.argv.includes('--source-only')){
  const native=spawnSync('cargo',['test','--offline','--test','axis_interaction','source_axis_flag_matrix','--','--nocapture'],{encoding:'utf8'});
  fs.writeFileSync(dir+'/native.log',native.stdout+native.stderr);assert.equal(native.status,0,native.stdout+native.stderr);
  const actual=native.stdout.split('\n').filter(l=>l.startsWith('AXIS_REFERENCE ')).map(l=>JSON.parse(l.slice(15)));
  assert.deepEqual(actual,rows,'native/source axis gestures differ');
 }
 console.log('16 actual DOM axis gesture/flag states verified.');
}
main().catch(e=>{console.error(e);process.exitCode=1;});

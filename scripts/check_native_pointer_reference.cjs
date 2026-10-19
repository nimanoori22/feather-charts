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
 for (const file of ['src/model/touch-mouse-event-data.ts','src/api/ichart-api.ts']) bundle.watchFiles.push(path.join(root,file));
 const files = Object.fromEntries(bundle.watchFiles.filter(p => p.startsWith(root + '/src/')).sort()
  .map(p => [path.relative(root, p), hash(p)]));
 const manifest = 'docs/porting/native-pointer-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const { output } = await bundle.generate({ format: 'iife', name: 'LightweightCharts' });
 await bundle.close();
 fs.mkdirSync('target/native-pointer', { recursive: true });
 const browser = await dependency('puppeteer').launch({ executablePath: '/usr/bin/chromium', headless: true });
 try {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.setContent('<!doctype html><div id="chart" style="width:800px;height:500px"></div>');
  await page.addScriptTag({ content: output[0].code });
  await page.evaluate(async () => {
   const c=LightweightCharts.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false}});
   const s=c.addSeries(LightweightCharts.LineSeries);s.setData([{time:1,value:1},{time:2,value:2}]);c.timeScale().fitContent();
   await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   window.pointerChart=c;window.pointerSeries=s;window.captured=[];
   const save=kind=>p=>window.captured.push({kind,source:p.sourceEvent&&{ctrl:p.sourceEvent.ctrlKey,alt:p.sourceEvent.altKey,shift:p.sourceEvent.shiftKey,meta:p.sourceEvent.metaKey,local:[p.sourceEvent.localX,p.sourceEvent.localY],client:[p.sourceEvent.clientX,p.sourceEvent.clientY]},point:p.point});
   c.subscribeClick(save('click'));c.subscribeDblClick(save('double'));c.subscribeCrosshairMove(save('move'));
  });
  await page.mouse.move(200,150);
  await page.keyboard.down('Control');await page.keyboard.down('Shift');
  await page.mouse.click(200,150);
  await page.keyboard.up('Control');await page.keyboard.up('Shift');
  await page.keyboard.down('Alt');await page.keyboard.down('Meta');
  await page.mouse.click(200,150);
  await page.keyboard.up('Alt');await page.keyboard.up('Meta');
  await page.evaluate(()=>{
   const target=document.elementFromPoint(200,150);
   const touch=new Touch({identifier:7,target,clientX:200,clientY:150,pageX:200,pageY:150,screenX:200,screenY:150});
   target.dispatchEvent(new TouchEvent('touchstart',{touches:[touch],changedTouches:[touch],targetTouches:[touch],bubbles:true,shiftKey:true}));
   target.dispatchEvent(new TouchEvent('touchend',{touches:[],changedTouches:[touch],targetTouches:[],bubbles:true,shiftKey:true}));
  });
  const result=await page.evaluate(()=>{
   const allClicks=window.captured.filter(p=>p.kind!=='move');
   const clicks=allClicks.slice(0,2),tap=allClicks[2];
   const hover=window.captured.find(p=>p.kind==='move'&&p.source);
   const before=window.captured.length;
   window.pointerChart.setCrosshairPosition(1,1,window.pointerSeries);
   window.pointerChart.clearCrosshairPosition();
   const synthetic=window.captured.slice(before);
   const states={clickKinds:clicks.map(p=>p.kind),modifiers:clicks.map(p=>[p.source.ctrl,p.source.alt,p.source.shift,p.source.meta]),localEqualsPoint:clicks.every(p=>p.source.local[0]===p.point.x&&p.source.local[1]===p.point.y),hoverSource:!!hover,touchClick:tap.kind,touchModifiers:[tap.source.ctrl,tap.source.alt,tap.source.shift,tap.source.meta],touchLocalEqualsPoint:tap.source.local[0]===tap.point.x&&tap.source.local[1]===tap.point.y,syntheticSources:synthetic.map(p=>p.source!==undefined)};
   window.pointerChart.remove();return states;
  });
  assert.deepEqual(errors,[]);
  assert.deepEqual(result,{clickKinds:['click','double'],modifiers:[[true,false,true,false],[false,true,false,true]],localEqualsPoint:true,hoverSource:true,touchClick:'click',touchModifiers:[false,false,true,false],touchLocalEqualsPoint:true,syntheticSources:[]});
  const fixture='docs/porting/native-pointer-states.json';
  if(process.argv.includes('--freeze'))fs.writeFileSync(fixture,JSON.stringify(result,null,2)+'\n');
  else assert.deepEqual(result,JSON.parse(fs.readFileSync(fixture)),'source pointer behavior changed');
  fs.writeFileSync('target/native-pointer/source-results.json',JSON.stringify({result,errors},null,2)+'\n');
  if(!process.argv.includes('--source-only')){
   const native=spawnSync('cargo',['test','--offline','--test','native_pointer'],{encoding:'utf8'});
   assert.equal(native.status,0,native.stdout+native.stderr);
  }
  console.log(JSON.stringify(result,null,2));
 } finally {await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

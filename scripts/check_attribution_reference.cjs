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
 const manifest='docs/porting/attribution-source.json';
 if(process.argv.includes('--freeze'))fs.writeFileSync(manifest,JSON.stringify({files},null,2)+'\n');
 else assert.deepEqual(files,JSON.parse(fs.readFileSync(manifest)).files);
 const {output}=await bundle.generate({format:'iife',name:'LightweightCharts'});await bundle.close();
 const browser=await dependency('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 try{
  const page=await browser.newPage();await page.setContent('<div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  const result=await page.evaluate(async()=>{
   const L=LightweightCharts,settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const observe=()=>{
    const e=document.querySelector('#tv-attr-logo');if(!e)return null;
    const b=e.getBoundingClientRect(),p=e.parentElement.getBoundingClientRect(),cs=getComputedStyle(e);
    return {width:b.width,height:b.height,left:b.left-p.left,bottom:p.bottom-b.bottom,dark:e.hasAttribute('data-dark'),fill:cs.getPropertyValue('--fill').trim(),stroke:cs.getPropertyValue('--stroke').trim(),lastPane:e.parentElement===Array.from(document.querySelectorAll('td')).filter(t=>t.querySelector('canvas')&&t.style.position==='relative').at(-1)};
   };
   let c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:true,textColor:'#131722'}});
   const a=c.addSeries(L.LineSeries);a.setData([{time:1,value:10},{time:2,value:20}]);
   const b=c.addSeries(L.LineSeries,{},1);b.setData([{time:1,value:10},{time:2,value:20}]);c.timeScale().fitContent();await settle();
   const light=observe();
   const hash=canvas=>Array.from(canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data);
   const before=hash(c.takeScreenshot(true,true));
   const e=document.querySelector('#tv-attr-logo');e.style.visibility='hidden';const after=hash(c.takeScreenshot(true,true));e.style.visibility='';
   const exportsExcludeLogo=before.length===after.length&&before.every((v,i)=>v===after[i]);
   c.swapPanes(0,1);await settle();const moved=observe();
   c.removePane(1);await settle();const removed=observe();
   let plotClicks=0;c.subscribeClick(()=>plotClicks++);
   document.querySelector('#tv-attr-logo').addEventListener('click',e=>e.preventDefault());document.querySelector('#tv-attr-logo').click();await settle();
   const activationDoesNotClickPlot=plotClicks===0;c.remove();
   const themes=[];
   for(const color of ['#a0a0a0','#a1a1a1','#ffffff']){
    c=L.createChart(document.getElementById('chart'),{width:300,height:90,layout:{attributionLogo:true,textColor:color}});await settle();themes.push(observe());c.remove();
   }
   c=L.createChart(document.getElementById('chart'),{width:300,height:90,layout:{attributionLogo:false}});await settle();const disabled=observe()===null;c.remove();
   return {light,moved,removed,themes,exportsExcludeLogo,activationDoesNotClickPlot,disabled};
  });
  const fixture='docs/porting/attribution-states.json';
  if(process.argv.includes('--freeze'))fs.writeFileSync(fixture,JSON.stringify(result,null,2)+'\n');
  else assert.deepEqual(result,JSON.parse(fs.readFileSync(fixture)));
  assert.ok(result.exportsExcludeLogo&&result.activationDoesNotClickPlot&&result.disabled);
  if(!process.argv.includes('--source-only')) {
   const generated=spawnSync('python',['scripts/generate_attribution_paths.py','--check'],{encoding:'utf8'});
   assert.equal(generated.status,0,generated.stdout+generated.stderr);
   const native=spawnSync('cargo',['test','--offline','--test','attribution','--','--nocapture'],{encoding:'utf8'});
   assert.equal(native.status,0,native.stdout+native.stderr);
   const row=native.stdout.split('\n').find(l=>l.startsWith('ATTRIBUTION_REFERENCE '));assert.ok(row);
   assert.deepEqual(JSON.parse(row.slice('ATTRIBUTION_REFERENCE '.length)),[result.light.width,result.light.height,result.light.left,result.light.bottom,result.light.dark,...result.themes.map(t=>t.dark)]);
  }
  console.log(JSON.stringify(result,null,2));
 }finally{await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

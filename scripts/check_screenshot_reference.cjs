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
 const manifest = 'docs/porting/screenshot-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({files}, null, 2)+'\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files);
 const {output}=await bundle.generate({format:'iife',name:'LightweightCharts'});await bundle.close();
 const browser=await dependency('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 try {
  const page=await browser.newPage();await page.setContent('<div id="chart"></div>');
  await page.addScriptTag({content:output[0].code});
  const result=await page.evaluate(async()=>{
   const L=LightweightCharts,c=L.createChart(document.getElementById('chart'),{width:800,height:500,layout:{attributionLogo:false}});
   const s=c.addSeries(L.LineSeries);s.setData([{time:1,value:10},{time:2,value:20},{time:3,value:15}]);
   const volume=c.addSeries(L.HistogramSeries,{},1);volume.setData([{time:1,value:3},{time:2,value:2},{time:3,value:4}]);
   const view={zOrder:()=> 'top',renderer:()=>({draw(target){target.useMediaCoordinateSpace(({context})=>{context.fillStyle='#ff00ff';context.fillRect(40,40,20,20);});}})};
   s.attachPrimitive({paneViews:()=>[view]});c.timeScale().fitContent();
   c.setCrosshairPosition(15,2,s);
   await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
   const images=[];
   for(const top of [false,true])for(const crosshair of [false,true]){
    const canvas=c.takeScreenshot(top,crosshair),ctx=canvas.getContext('2d');
    images.push({top,crosshair,width:canvas.width,height:canvas.height,pixels:Array.from(ctx.getImageData(0,0,canvas.width,canvas.height).data)});
   }
   const mode=c.options().crosshair.mode;c.remove();return {images,mode};
  });
  const images=result.images.map(({pixels,...rest})=>({...rest,sha256:createHash('sha256').update(Buffer.from(pixels)).digest('hex')}));
  const states={images,mode:result.mode};
  assert.equal(images[0].sha256,images[1].sha256,'crosshair does not force top canvas');
  assert.notEqual(images[0].sha256,images[2].sha256,'top primitive must be included');
  assert.notEqual(images[2].sha256,images[3].sha256,'crosshair is opt-in');
  const fixture='docs/porting/screenshot-states.json';
  if(process.argv.includes('--freeze'))fs.writeFileSync(fixture,JSON.stringify(states,null,2)+'\n');
  else assert.deepEqual(states,JSON.parse(fs.readFileSync(fixture)));
  if(!process.argv.includes('--source-only')) {
   const native=spawnSync('cargo',['test','--offline','--test','screenshot_export','--','--nocapture'],{encoding:'utf8'});
   assert.equal(native.status,0,native.stdout+native.stderr);
   const row=native.stdout.split('\n').find(l=>l.startsWith('SCREENSHOT_REFERENCE '));
   assert.deepEqual(JSON.parse(row.slice(21)),[images[0].sha256===images[1].sha256,images[0].sha256!==images[2].sha256,images[2].sha256!==images[3].sha256]);
  }
  console.log(JSON.stringify(states,null,2));
 }finally{await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

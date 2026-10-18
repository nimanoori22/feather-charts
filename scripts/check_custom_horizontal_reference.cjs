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
 const manifest='docs/porting/custom-horizontal-source.json';
 if(process.argv.includes('--freeze'))fs.writeFileSync(manifest,JSON.stringify({files},null,2)+'\n');
 else assert.deepEqual(files,JSON.parse(fs.readFileSync(manifest)).files);
 const {output}=await bundle.generate({format:'iife',name:'LightweightCharts'});await bundle.close();
 const browser=await dependency('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true});
 try{
  const page=await browser.newPage();await page.setContent('<div id="chart"></div>');await page.addScriptTag({content:output[0].code});
  const result=await page.evaluate(async()=>{
   const phases=['Open','Auction','Close'];
   class Sessions {
    constructor(){this.opts={};this.setCount=0;this.formatCount=0;}
    options(){return this.opts;}
    setOptions(o){this.opts=o;this.setCount++;}
    preprocessData(){}
    updateFormatter(){this.formatCount++;}
    convertHorzItemToInternal(item){const phase=phases.indexOf(item.phase);if(item.session<=0||phase<0)throw Error('invalid session');return {rank:item.session*3+phase};}
    createConverterToInternalObj(){return item=>this.convertHorzItemToInternal(item);}
    key(item){return item.rank??this.convertHorzItemToInternal(item).rank;}
    cacheKey(item){return item.rank;}
    formatHorzItem(item){return `${this.opts.sessionPrefix}${Math.floor(item.rank/3)} ${phases[item.rank%3]}`;}
    formatTickmark(mark){return this.formatHorzItem(mark.time);}
    maxTickMarkWeight(marks){return Math.max(0,...marks.map(m=>m.weight));}
    fillWeightsForPoints(points,start){for(let i=start;i<points.length;i++)points[i].timeWeight=points[i].time.rank%3===0?7:1;}
   }
   const b=new Sessions(),L=LightweightCharts,c=L.createChartEx(document.getElementById('chart'),b,{width:800,height:500,sessionPrefix:'S',layout:{attributionLogo:false}});
   const s=c.addSeries(L.LineSeries),point=(session,phase)=>({session,phase});
   const a=point(1,'Open'),z=point(1,'Close'),v=point(3,'Auction'),end=point(8,'Open');
   s.setData([{time:a,value:10},{time:z,value:20},{time:v,value:15}]);c.timeScale().fitContent();
   const settle=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));await settle();
   const initialTimes=s.data().map(r=>r.time),coords=initialTimes.map(t=>c.timeScale().timeToCoordinate(t));
   const equallySpaced=Math.abs((coords[1]-coords[0])-(coords[2]-coords[1]))<1e-8;
   s.update({time:v,value:16});s.update({time:a,value:12},true);s.update({time:end,value:18});
   const latestValue=s.data().at(-1).value,historyValue=s.data()[0].value,removedTimes=s.pop(2).map(r=>r.time);
   const whitespace=c.addSeries(L.LineSeries);whitespace.setData([{time:point(2,'Open')}]);
   c.timeScale().fitContent();await settle();const range=c.timeScale().getVisibleRange();
   c.applyOptions({sessionPrefix:'Run '});await settle();
   const formatted=b.formatHorzItem(b.convertHorzItemToInternal(z));
   let rejected=false;try{s.update({time:point(0,'Open'),value:1});}catch{rejected=true;}
   const observations={initialTimes,equallySpaced,latestValue,historyValue,removedTimes,range,formatted,rejected,initialized:b.setCount>0&&b.formatCount>0};
   c.remove();return observations;
  });
  const fixture='docs/porting/custom-horizontal-states.json';
  if(process.argv.includes('--freeze'))fs.writeFileSync(fixture,JSON.stringify(result,null,2)+'\n');
  else assert.deepEqual(result,JSON.parse(fs.readFileSync(fixture)));
  if(!process.argv.includes('--source-only')) {
   const native=spawnSync('cargo',['test','--offline','--test','custom_horizontal','source_reference','--','--nocapture'],{encoding:'utf8'});
   assert.equal(native.status,0,native.stderr+'\n'+native.stdout);
   const line=native.stdout.split('\n').find(s=>s.startsWith('CUSTOM_HORIZONTAL_REFERENCE '));
   assert.ok(line,'native reference was not emitted');
   assert.deepEqual(JSON.parse(line.slice('CUSTOM_HORIZONTAL_REFERENCE '.length)),result);
  }
  console.log(JSON.stringify(result,null,2));
 }finally{await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

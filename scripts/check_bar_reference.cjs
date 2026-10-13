const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const width=new Function(`${source('renderers/optimal-bar-width.ts')}\nreturn optimalBarWidth;`)();
const Renderer=new Function('BitmapCoordinatesPaneRenderer','optimalBarWidth','ensureNotNull',`${source('renderers/bars-renderer.ts')}\nreturn PaneRendererBars;`)(class {},width,x=>x);
const output=spawnSync(path.resolve('target/debug/examples/bar_reference'),{encoding:'utf8',maxBuffer:64*1024*1024});
if(output.status!==0)throw new Error(output.stderr||'Build bar_reference first');
const cases=JSON.parse(output.stdout);
for(const [index,c] of cases.entries()){
    const result=[];
    const context={fillStyle:'',fillRect(x,y,w,h){result.push([x,y,w,h,this.fillStyle]);}};
    const renderer=new Renderer();renderer.setData(c);
    renderer._drawImpl({context,horizontalPixelRatio:c.h,verticalPixelRatio:c.v});
    assert.equal(result.length,c.result.length,`case ${index}`);
    for(let i=0;i<result.length;i++){for(let j=0;j<4;j++)assert.ok(Math.abs(result[i][j]-c.result[i][j])<1e-8,`case ${index} rect ${i} component ${j}`);assert.equal(result[i][4],c.result[i][4]);}
}
console.log(`${cases.length} Bar stem/tick rectangle cases match original TypeScript (including signed extents and colors).`);

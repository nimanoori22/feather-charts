// Execute the actual pinned TS factory, defaults, view and renderer with fixed metrics.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const manifest=JSON.parse(fs.readFileSync('docs/porting/text-watermark-source.json'));
for(const [file,hash] of Object.entries(manifest.sources))assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'),hash,file);
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const factory=new Function([
'helpers/make-font.ts','plugins/text-watermark/options.ts','plugins/text-watermark/pane-renderer.ts','plugins/text-watermark/pane-view.ts','plugins/pane-primitive-wrapper.ts','plugins/text-watermark/primitive.ts'
].map(source).join('\n')+';return createTextWatermark;')();
const run=spawnSync('cargo',['test','--offline','--lib','text_watermark_renderer::tests','--','--nocapture','--test-threads=1'],{encoding:'utf8'});assert.equal(run.status,0,run.stderr);
function create(options){let primitive,updates=0;const pane={attachPrimitive(p){primitive=p;p.attached({requestUpdate:()=>updates++});},detachPrimitive(p){assert.equal(p,primitive);p.detached();}};const api=factory(pane,options);return {api,get primitive(){return primitive;},get updates(){return updates;}};}
function draw(probe,size,widths){const calls=[],measures=[];let x,y,z,alignment;const ctx={font:'',save(){},restore(){},translate(a,b){x=a;y=b;},scale(a,b){assert.equal(a,b);z=a;},set textAlign(v){alignment=v;},measureText(text){measures.push(text);return {width:widths[text]};},fillText(text){calls.push([x,y,z,text,alignment]);}};probe.primitive.updateAllViews();probe.primitive.paneViews()[0].renderer().draw({useMediaCoordinateSpace:f=>f({context:ctx,mediaSize:{width:size[0],height:size[1]}})});return {calls,measures};}
let count=0;
for(const line of run.stdout.split('\n').filter(l=>l.includes('WATERMARK_LAYOUT '))){
 const c=JSON.parse(line.split('WATERMARK_LAYOUT ')[1]);
 const scenarios=[ [['A'],[120],[undefined]], [['A','','B'],[120,0,240],[undefined,undefined,18]], [['A','B'],[c.size[0],5],[60,12]], [['A','  ','B'],[500,20,60],[0,13.5,undefined]], [['BTCUSD','日本語'],[0,800],[150,40]] ];
 const [texts,widths,heights]=scenarios[c.scenario],options={horzAlign:c.horizontal.toLowerCase(),vertAlign:c.vertical.toLowerCase(),lines:texts.map((text,i)=>({text,lineHeight:heights[i]}))};
 const probe=create(options),actual=draw(probe,c.size,Object.fromEntries(texts.map((t,i)=>[t,widths[i]]))).calls;
 assert.equal(actual.length,c.actual.length);
 actual.forEach((a,i)=>a.slice(0,3).forEach((v,j)=>assert.ok(Math.abs(v-c.actual[i][j])<1e-9,JSON.stringify(c))));count++;
}
assert.equal(count,270);
const probe=create({lines:[{text:'A',fontSize:24,fontStyle:'bold',color:'red'},{text:'B',fontSize:30}]});
probe.api.applyOptions({horzAlign:'right',visible:false});assert.equal(probe.updates,1);assert.deepEqual(draw(probe,[400,200],{A:100,B:100}).calls,[]);
probe.api.applyOptions({visible:true});assert.equal(probe.primitive._options.lines[0].fontSize,24);assert.equal(probe.primitive._options.lines[0].color,'red');
probe.api.applyOptions({lines:[{text:'C'}]});assert.equal(probe.primitive._options.lines.length,1);assert.equal(probe.primitive._options.lines[0].fontSize,48);assert.equal(probe.primitive._options.lines[0].color,'rgba(0, 0, 0, 0.5)');
probe.api.detach();assert.equal(probe.primitive.requestUpdate,undefined);
assert.equal(probe.primitive.paneViews()[0].zOrder,undefined);
console.log(`${count} layout states match actual TS defaults/view/renderer; factory option-update/detach/default-layer checks pass.`);

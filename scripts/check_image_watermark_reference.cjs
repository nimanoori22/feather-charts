// Execute the pinned built-in TS factory/view/renderer, including image load and detach.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
for(const [file,hash] of Object.entries(JSON.parse(fs.readFileSync('docs/porting/image-watermark-source.json')).files))
 assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'),hash,file);
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const factory=new Function('Image', ['plugins/image-watermark/options.ts','plugins/image-watermark/pane-renderer.ts','plugins/image-watermark/pane-view.ts','plugins/pane-primitive-wrapper.ts','plugins/image-watermark/primitive.ts'].map(source).join('\n')+';return createImageWatermark;')(class {});
function create(size,options){let primitive,updates=0;const pane={attachPrimitive(p){primitive=p;p.attached({requestUpdate:()=>updates++});},detachPrimitive(p){assert.equal(p,primitive);p.detached();}};const api=factory(pane,'owned-test-image',options);Object.assign(primitive._imgElement,{naturalWidth:size[0],naturalHeight:size[1]});return {api,primitive,load(){primitive._imgElement.onload();},get updates(){return updates;}};}
function draw(probe,pane){const calls=[],ctx={globalAlpha:1,drawImage(image,...placement){calls.push({placement,alpha:this.globalAlpha});}};probe.primitive.updateAllViews();probe.primitive.paneViews()[0].renderer().draw({useMediaCoordinateSpace:f=>f({context:ctx,mediaSize:{width:pane[0],height:pane[1]}})});return calls;}
const run=spawnSync('cargo',['test','--offline','--lib','image_watermark_renderer::tests','--','--nocapture','--test-threads=1'],{encoding:'utf8'});assert.equal(run.status,0,run.stderr);
let count=0;
for(const line of run.stdout.split('\n').filter(l=>l.includes('IMAGE_LAYOUT '))){const c=JSON.parse(line.split('IMAGE_LAYOUT ')[1]);const p=create(c.image,{padding:c.padding,maxWidth:c.max_width??undefined,maxHeight:c.max_height??undefined});assert.deepEqual(draw(p,c.pane),[]);p.load();const actual=draw(p,c.pane);assert.equal(actual.length,1);actual[0].placement.forEach((v,i)=>assert.ok(Math.abs(v-c.actual[i])<1e-9,JSON.stringify(c)));count++;}
assert.equal(count,112);
const p=create([10,20],{alpha:.5,padding:2,maxWidth:30});assert.equal(p.updates,0);p.load();assert.equal(p.updates,1);assert.equal(p.primitive.paneViews()[0].zOrder(),'bottom');assert.equal(draw(p,[100,100])[0].alpha,.5);p.api.applyOptions({alpha:.75});assert.equal(draw(p,[100,100])[0].alpha,.75);assert.equal(p.primitive._options.padding,2);assert.equal(p.primitive._options.maxWidth,30);p.api.detach();assert.equal(p.primitive._imgElement,null);const before=p.updates;p.primitive.requestUpdate();assert.equal(p.updates,before);
console.log(`${count} valid placement states match actual TS; load/update/detach/default bottom-layer checks pass. Eight nonpositive drawable cases follow the explicit native skip policy.`);

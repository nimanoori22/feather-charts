const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const math=new Function(`${source('helpers/mathex.ts')}\nreturn {ceiledOdd,ceiledEven};`)();
const utils=new Function('ceiledOdd','ceiledEven',`${source('plugins/series-markers/utils.ts')}\nreturn {shapeSize,calculateShapeHeight,shapeMargin};`)(math.ceiledOdd,math.ceiledEven);
const helpers={};
for(const name of ['circle','square','arrow','text'])Object.assign(helpers,new Function('shapeSize','ceiledOdd',`${source('plugins/series-markers/series-markers-'+name+'.ts')}\nreturn {${name==='text'?'hitTestText':`draw${name[0].toUpperCase()+name.slice(1)},hitTest${name[0].toUpperCase()+name.slice(1)}`}};`)(utils.shapeSize,math.ceiledOdd));
const place=new Function('ensureNotNull','ensureNever','isNumber','calculateShapeHeight','calculateShapeMargin',`${source('plugins/series-markers/pane-view.ts')}\nreturn fillSizeAndY;`)(x=>x,()=>{throw Error('unreachable');},x=>typeof x==='number',utils.calculateShapeHeight,utils.shapeMargin);
const out=spawnSync(path.resolve('target/debug/examples/series_markers_reference'),{encoding:'utf8',maxBuffer:64*1024*1024});
if(out.status!==0)throw Error(out.stderr||'Build series_markers_reference first');
function near(a,b){if(Array.isArray(a)){assert.equal(a.length,b.length);a.forEach((v,i)=>near(v,b[i]));}else if(typeof a==='number')assert.ok(Math.abs(a-b)<1e-7,`${a} != ${b}`);else assert.equal(a,b);}
const cases=JSON.parse(out.stdout);let hits=0;
for(const c of cases){
 if(c.kind==='placement'){
  const offsets={aboveBar:utils.shapeMargin(c.spacing),belowBar:utils.shapeMargin(c.spacing)};
  const prices=new Map([90,103,105,110].map((p,i)=>[p,c.prices[i]]));
  const series={priceScale:()=>({options:()=>({invertScale:c.inverted})}),priceToCoordinate:p=>prices.get(p)};
  const chart={timeScale:()=>({options:()=>({barSpacing:c.spacing})})};
  const result=c.positions.map(position=>{const item={text:{}};place(item,{position,price:103,size:c.size},{open:100,high:110,low:90,close:105},offsets,12,utils.shapeMargin(c.spacing),series,chart);return [item.y,item.size,item.text.y];});
  near(result,c.result);const margin=utils.calculateShapeHeight(c.spacing)*1.5+utils.shapeMargin(c.spacing)*2;near(c.margins,[margin,margin]);continue;
 }
 const coords={x:Math.round(-0.5*c.ratio)+(Math.max(1,Math.floor(c.ratio))%2)/2,y:20.25*c.ratio,pixelRatio:c.ratio};
 let command=[];const points=[];
 const ctx={beginPath(){},arc(x,y,r){command=['circle',x,y,r];},fill(){},fillRect(x,y,w){command=['square',x,y,w];},moveTo(x,y){points.push([x,y]);},lineTo(x,y){points.push([x,y]);}};
 if(c.size!==0){if(c.shape==='circle')helpers.drawCircle(ctx,coords,c.size);else if(c.shape==='square')helpers.drawSquare(ctx,coords,c.size);else {helpers.drawArrow(c.shape==='arrowUp',ctx,coords,c.size);command=['polygon',...points];}}
 near(command,c.command);
 for(const [dx,dy,expected] of c.hits){const actual=c.size===0?false:c.shape==='circle'?helpers.hitTestCircle(-0.5,20.25,c.size,-0.5+dx,20.25+dy):c.shape==='square'?helpers.hitTestSquare(-0.5,20.25,c.size,-0.5+dx,20.25+dy):helpers.hitTestArrow(c.shape==='arrowDown',-0.5,20.25,c.size,-0.5+dx,20.25+dy);assert.equal(actual,expected);hits++;}
}
console.log(`${cases.length} marker placement/bitmap cases and ${hits} shape hits match actual TypeScript.`);

// Run the actual frozen source view and renderer with a monotonic clock/canvas.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const manifest=JSON.parse(fs.readFileSync('docs/porting/last-price-animation-source.json'));
for(const [file,hash] of Object.entries(manifest.sources)) assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'),hash,file);
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const Renderer=new Function('BitmapCoordinatesPaneRenderer',source('renderers/series-last-price-animation-renderer.ts')+';return SeriesLastPriceAnimationRenderer;')(class{});
let now=0,mode=0;
const View=new Function('SeriesLastPriceAnimationRenderer','LastPriceAnimationMode','performance','assert',source('views/pane/series-last-price-animation-pane-view.ts')+';return SeriesLastPriceAnimationPaneView;')(Renderer,{Disabled:0,Continuous:1,OnDataUpdate:2},{now:()=>now},assert);
const series={options:()=>({lastPriceAnimation:mode,lineWidth:3}),firstValue:()=>({value:100}),lastValueData:()=>({noData:false,index:31,price:102,color:'series-color'}),priceScale:()=>({priceToCoordinate:()=>17.25}),model:()=>({timeScale:()=>({visibleStrictRange:()=>({contains:()=>true}),indexToCoordinate:()=>23.7}),colorParser:()=>({applyAlpha:(color,alpha)=>({color,alpha})})})};
const modes={Disabled:0,Continuous:1,OnDataUpdate:2};
const run=spawnSync('cargo',['test','--offline','--lib','model::last_price_animation::tests','--','--nocapture','--test-threads=1'],{encoding:'utf8'});assert.equal(run.status,0,run.stderr);
let view=new View(series), count=0;
function compare(c){ mode=modes[c.mode];now=c.ms;view.invalidateStage();const data=view.renderer().data();assert.equal(view.animationActive(),c.active);for(const [key,val] of [['radius',data.radius],['fill',data.fillColor.alpha],['stroke',data.strokeColor.alpha]])assert.ok(Math.abs(c[key]-val)<1e-9,`${key} ${JSON.stringify(c)} != ${val}`);count++; }
for(const line of run.stdout.split('\n').filter(l=>l.includes('PRICE_SAMPLE ')))compare(JSON.parse(line.split('PRICE_SAMPLE ')[1]));
now=0;view=new View(series);
for(const line of run.stdout.split('\n').filter(l=>l.includes('PRICE_TRANSITION '))){const c=JSON.parse(line.split('PRICE_TRANSITION ')[1]);mode=modes[c.mode];now=c.ms;if(c.update)view.onNewRealtimeDataReceived();else if(c.clear)view.onDataCleared();compare(c);}
assert.equal(count,58);
// Verify the source renderer's exact circle geometry at fractional DPR.
const data=view.renderer().data();
for(const dpr of [1,1.25,1.5,2]){
 const arcs=[],ops=[];const ctx={beginPath(){},arc(...a){arcs.push(a.slice(0,3));},fill(){ops.push('fill');},stroke(){ops.push('stroke');}};
 view.renderer()._drawImpl({context:ctx,horizontalPixelRatio:dpr,verticalPixelRatio:dpr});
 const width=Math.max(1,Math.floor(dpr)), x=Math.round(23.7*dpr)+(width%2)/2;
 assert.deepEqual(arcs,[[x,17.25*dpr,4.5*dpr],[x,17.25*dpr,data.radius*dpr],[x,17.25*dpr,data.radius*dpr+width/2]]);
 assert.deepEqual(ops,['fill','fill','stroke']);
}
console.log(`${count} source timing states and 4 DPR renderer cases match.`);
const ensureDefined=v=>{assert.notEqual(v,undefined);return v;};
const creators=new Function('ensureDefined',source('model/data-consumer.ts')+'\n'+source('model/get-series-plot-row-creator.ts')+';return {getSeriesPlotRowCreator,isSeriesPlotRow};')(ensureDefined);
const lowerBound=new Function(source('helpers/algorithms.ts')+';return lowerBound;')();
const Layer=new Function('ensureDefined','lowerBound','getSeriesPlotRowCreator','isSeriesPlotRow',source('model/data-layer.ts')+';return DataLayer;')(ensureDefined,lowerBound,creators.getSeriesPlotRowCreator,creators.isSeriesPlotRow);
const layer=new Layer({key:v=>v,createConverterToInternalObj:()=>v=>v,preprocessData(){},fillWeightsForPoints(){}});
const target={seriesType:()=> 'Line',customSeriesPlotValuesBuilder:()=>undefined,customSeriesWhitespaceCheck:()=>undefined,isConflationEnabled:()=>false};
const flags=spawnSync('cargo',['test','--offline','--test','last_price_animation','source_update_flags','--','--nocapture'],{encoding:'utf8'});assert.equal(flags.status,0,flags.stderr);
let flagCount=0;
for(const line of flags.stdout.split('\n').filter(l=>l.includes('PRICE_FLAG '))){
 const c=JSON.parse(line.split('PRICE_FLAG ')[1]);const rows=c.times.map(t=>({time:1700000000+t*60,...(c.whitespace?{}:{value:100})}));
 const response=c.op==='set'?layer.setSeriesData(target,rows):layer.updateSeriesData(target,rows[0],c.historical);
 const actual=response.series.get(target)?.info?.lastBarUpdatedOrNewBarsAddedToTheRight??null;
 assert.equal(c.flag,actual,JSON.stringify(c));flagCount++;
}
assert.equal(flagCount,9);console.log(`${flagCount} actual DataLayer update/full replacement/whitespace/clear flags match.`);
const endpoints=spawnSync('cargo',['test','--offline','--test','last_price_animation','all_families_canonical_endpoint_dpr_expiry_and_retained_frames','--','--nocapture'],{encoding:'utf8'});assert.equal(endpoints.status,0,endpoints.stderr);
let endpointCount=0;
for(const line of endpoints.stdout.split('\n').filter(l=>l.includes('PRICE_ENDPOINT '))){
 const c=JSON.parse(line.split('PRICE_ENDPOINT ')[1]),renderer=new Renderer();
 renderer.setData({center:{x:c.x,y:c.y},radius:c.radius,seriesLineWidth:3,seriesLineColor:'red',fillColor:'red',strokeColor:'red'});
 const arcs=[],ctx={beginPath(){},arc(x,y,r){arcs.push([x,y,r]);},fill(){},stroke(){}};
 renderer._drawImpl({context:ctx,horizontalPixelRatio:c.dpr,verticalPixelRatio:c.dpr});
 const expected=[[c.center_x*c.dpr,c.center_y*c.dpr,c.dot*c.dpr],[c.center_x*c.dpr,c.center_y*c.dpr,c.radius*c.dpr],[c.center_x*c.dpr,c.center_y*c.dpr,(c.radius+c.stroke/2)*c.dpr]];
 arcs.flat().forEach((v,i)=>assert.ok(Math.abs(v-expected.flat()[i])<0.0002,`endpoint rounding mismatch ${v}`));
 endpointCount++;
}
assert.equal(endpointCount,12);console.log(`${endpointCount} Rust canonical endpoint/DPR payloads match the actual source renderer.`);
const Parser=new Function(source('model/colors.ts')+';return ColorParser;')();
const parser=new Parser([],new Map([['half',[255,0,0,0.5]]]));
assert.equal(parser.applyAlpha('half',0.25),'rgba(255, 0, 0, 0.125)');assert.equal(parser.applyAlpha('transparent',0.25),'transparent');
const Colorer=new Function('ensureNotNull','PlotRowValueIndex',source('model/series-bar-colorer.ts')+';return SeriesBarColorer;')(ensureDefined,{Open:0,High:1,Low:2,Close:3});
for(const [kind,style,row,expected] of [
 ['Line',{color:'blue'},{value:[1,1,1,1],color:'red'},'red'],
 ['Area',{lineColor:'blue'},{value:[1,1,1,1],lineColor:'red'},'red'],
 ['Baseline',{baseValue:{price:100},topLineColor:'green',bottomLineColor:'blue'},{value:[101,101,101,101],topLineColor:'purple'},'green'],
 ['Baseline',{baseValue:{price:100},topLineColor:'green',bottomLineColor:'blue'},{value:[99,99,99,99],bottomLineColor:'purple'},'blue'],
]) assert.equal(new Colorer({seriesType:()=>kind,options:()=>style,bars:()=>({valueAt:()=>row})}).barStyle(0).barColor,expected);
console.log('Actual source alpha multiplication, transparency and canonical family color rules verified.');

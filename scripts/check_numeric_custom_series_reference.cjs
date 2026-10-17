// Actual source numeric custom data and inherited yield construction; no DOM claim.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve('../lightweight-charts');
for(const [file,hash] of Object.entries(JSON.parse(fs.readFileSync('docs/porting/numeric-custom-series-source.json')).files))
 assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'),hash,file);
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const ensureDefined=v=>{assert.notEqual(v,undefined);return v;};
const lowerBound=new Function(source('helpers/algorithms.ts')+';return lowerBound;')();
const creators=new Function('ensureDefined',source('model/data-consumer.ts')+source('model/get-series-plot-row-creator.ts')+';return {getSeriesPlotRowCreator,isSeriesPlotRow};')(ensureDefined);
const DataLayer=new Function('lowerBound','ensureDefined','getSeriesPlotRowCreator','isSeriesPlotRow',source('model/data-layer.ts')+';return DataLayer;')(lowerBound,ensureDefined,creators.getSeriesPlotRowCreator,creators.isSeriesPlotRow);
const queue=[];
const Yield=new Function('queueMicrotask',source('helpers/delegate.ts')+source('model/yield-curve-horz-scale-behavior/yield-curve-horz-scale-behavior.ts')+';return YieldCurveHorzScaleBehavior;')(f=>queue.push(f));
const Price=new Function(source('model/horz-scale-behavior-price/horz-scale-behaviour-price.ts')+';return HorzScaleBehaviorPrice;')();
const defaults=new Function('LineStyle','PriceLineSource',source('api/options/series-options-defaults.ts')+';return seriesOptionsDefaults;')({Solid:0,Dashed:2},{LastBar:0});
const definition=new Function('ensure',source('model/series/custom-series.ts')+';return createCustomSeriesDefinition;')(ensureDefined);
const SourceApi=new Function('ensure','createCustomSeriesDefinition',source('api/chart-api.ts')+';return ChartApi;')(ensureDefined,definition);
const merge=new Function(source('helpers/strict-type-checks.ts')+';return merge;')();
class Base {
 constructor(container,behavior,options){this.opts=options;this.behavior=behavior;this.added=[];}
 options(){return this.opts;} horzBehaviour(){return this.behavior;}
 _addSeriesImpl(def,options,pane){const result={def,options:{...defaults,...def.defaultOptions,...options},pane};this.added.push(result);return result;}
 addSeries(def,options,pane){const result=this._addSeriesImpl(def,options,pane);result.setData=()=>{};return result;}
}
Base.prototype.addCustomSeries=SourceApi.prototype.addCustomSeries;
const YieldApi=new Function('ChartApi','lineSeries','YieldCurveHorzScaleBehavior','merge',source('api/options/yield-curve-chart-options-defaults.ts')+source('api/yield-chart-api.ts')+';return YieldChartApi;')(Base,{type:'Line',isBuiltIn:true},Yield,merge);
const chart=new YieldApi({},{});
const custom=chart.addCustomSeries({defaultOptions:()=>({})});
assert.equal(custom.options.lastValueVisible,true);assert.equal(custom.options.priceLineVisible,true);
assert.equal(chart.opts.localization.priceFormatter(5.378),'5.378%');
assert.throws(()=>chart.addSeries({type:'Histogram',isBuiltIn:true}));queue.length=0;
const helpers=new Function('ChartApi','lineSeries','YieldCurveHorzScaleBehavior','merge',source('api/options/yield-curve-chart-options-defaults.ts')+source('api/yield-chart-api.ts')+';return {generateWhitespaceData,buildWhitespaceState};')(Base,{},Yield,merge);
const states=[];
for(const family of ['price','yield']){
 const b=family==='price'?new Price():new Yield();const yieldOpts={baseResolution:1,minimumTimeRange:120,startTimeRange:0};
 b.setOptions(family==='price'?{localization:{precision:2}}:{localization:{},yieldCurve:yieldOpts});
 const layer=new DataLayer(b);let current=[],signature='';
 const owners=[0,1,2].map(id=>({id,seriesType:()=>id===1?'Custom':'Line',isConflationEnabled:()=>false,customSeriesPlotValuesBuilder:()=>id===1?(d=>[d.low,d.high,d.value]):undefined,customSeriesWhitespaceCheck:()=>id===1?(d=>d.value===undefined):undefined}));
 const accept=u=>{if(u.timeScale.points)current=u.timeScale.points;};
 if(family==='yield')b.whitespaceInvalidated().subscribe(last=>{const state=helpers.buildWhitespaceState(yieldOpts,last),key=JSON.stringify(state);if(key!==signature){signature=key;accept(layer.setSeriesData(owners[0],helpers.generateWhitespaceData(state)));}});
 function flush(){let guard=0;while(queue.length){assert.ok(++guard<20);queue.shift()();}}
 function snap(name){flush();const rows=layer._seriesRowsBySeries.get(owners[1])||[];states.push([family,name,current.map(p=>p.time).join(','),current.filter(p=>[...p.pointData.mapping.values()].some(r=>r.value!==undefined)).map(p=>p.time).join(','),rows.map(r=>r.value[3]).join(',')].join('\t'));}
 const keys=family==='price'?[90,100,150,175,200]:[1,6,360,420,480];
 const row=(time,value)=>({time,value,low:value-1,high:value+1});
 if(family==='yield'){accept(layer.setSeriesData(owners[0],helpers.generateWhitespaceData(helpers.buildWhitespaceState(yieldOpts,0))));}snap('empty');
 accept(layer.setSeriesData(owners[1],keys.slice(0,3).map((t,i)=>row(t,i+2))));snap('set');
 accept(layer.setSeriesData(owners[2],[{time:keys[3],value:5}]));snap('union');
 accept(layer.updateSeriesData(owners[1],row(keys[1],7),true));snap('history');
 accept(layer.updateSeriesData(owners[1],row(keys[4],9),false));snap('append');
 accept(layer.updateSeriesData(owners[1],{time:keys[4]},false));snap('whitespace');
 accept(layer.setSeriesData(owners[1],[row(keys[0],2)]));snap('shorter');
 accept(layer.removeSeries(owners[2]));snap('remove_peer');
 accept(layer.setSeriesData(owners[1],[]));snap('clear');
}
const file='docs/porting/numeric-custom-series-states.tsv',fixture=states.join('\n')+'\n';
if(process.argv.includes('--freeze'))fs.writeFileSync(file,fixture);else assert.equal(fs.readFileSync(file,'utf8'),fixture);
if(!process.argv.includes('--source-only')){
 const run=spawnSync('cargo',['test','--offline','--test','numeric_custom_series','numeric_custom_source_states'],{encoding:'utf8'});assert.equal(run.status,0,run.stdout+run.stderr);assert.ok(run.stdout.includes('1 passed'));
}
if(!process.argv.includes('--source-only')){
 const DataConflater=new Function('PlotRowValueIndex',source('model/conflation/constants.ts')+source('model/data-conflater.ts')+';return DataConflater;')({Open:0,High:1,Low:2,Close:3});
 const run=spawnSync('cargo',['test','--offline','--test','numeric_custom_series','numeric_custom_reducer_contexts_and_sparse_fallback','--','--nocapture'],{encoding:'utf8'});assert.equal(run.status,0,run.stdout+run.stderr);
 const records=run.stdout.split('\n').filter(l=>l.startsWith('NUMERIC_CUSTOM_REDUCTION ')).map(l=>JSON.parse(l.slice(25)));assert.equal(records.length,4);
 for(const c of records.filter(c=>c.reduce))for(const [name,last] of [['initial',7],['latest',9]]){
  const rows=Array.from({length:7},(_,i)=>{const v=i===6?last:i+1;return {index:i,time:90+i*2.5,originalTime:90+i*2.5,value:[v,v+1,v-1,v],data:{time:90+i*2.5,value:v}};});
  const actual=new DataConflater().conflateByFactor(rows,8,(a,b)=>({time:a.originalTime,value:a.data.value*10+b.data.value}),true,d=>[d.value]);
  assert.deepEqual(c[name],actual.map(r=>r.data.value));
 }
}
console.log('18 actual-source numeric custom states and inherited yield custom defaults pass.');

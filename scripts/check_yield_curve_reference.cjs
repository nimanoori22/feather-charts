// Execute pinned upstream behavior, scaffold helpers and DataLayer with explicit microtasks.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve('../lightweight-charts');
for(const [file,hash] of Object.entries(JSON.parse(fs.readFileSync('docs/porting/yield-curve-source.json')).files))
 assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'),hash,file);
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const ensureDefined=v=>{assert.notEqual(v,undefined);return v;};
const lowerBound=new Function(source('helpers/algorithms.ts')+';return lowerBound;')();
const creators=new Function('ensureDefined',source('model/data-consumer.ts')+source('model/get-series-plot-row-creator.ts')+';return {getSeriesPlotRowCreator,isSeriesPlotRow};')(ensureDefined);
const DataLayer=new Function('lowerBound','ensureDefined','getSeriesPlotRowCreator','isSeriesPlotRow',source('model/data-layer.ts')+';return DataLayer;')(lowerBound,ensureDefined,creators.getSeriesPlotRowCreator,creators.isSeriesPlotRow);
const queue=[];
const Behavior=new Function('queueMicrotask',source('helpers/delegate.ts')+source('model/yield-curve-horz-scale-behavior/yield-curve-horz-scale-behavior.ts')+';return YieldCurveHorzScaleBehavior;')(f=>queue.push(f));
const helpers=new Function('ChartApi','lineSeries','YieldCurveHorzScaleBehavior','merge',source('api/options/yield-curve-chart-options-defaults.ts')+source('api/yield-chart-api.ts')+';return {generateWhitespaceData,buildWhitespaceState,defaultOptions,lineStyleDefaultOptionOverrides};')(class{}, {},Behavior,()=>{});
// Exercise the real public YieldChartApi constructor/defaults/restriction boundary.
const merge=new Function(source('helpers/strict-type-checks.ts')+';return merge;')();
class BaseChart {
 constructor(container,behavior,options){this.value=options;this.behavior=behavior;this.added=[];}
 options(){return this.value;} horzBehaviour(){return this.behavior;}
 addSeries(definition,options,pane){const rows=[];this.added.push({definition,options,pane,rows});return {setData(data){rows.splice(0,rows.length,...data);}};}
}
const factory=new Function('ChartApi','lineSeries','YieldCurveHorzScaleBehavior','merge','fetchHtmlElement',source('api/options/yield-curve-chart-options-defaults.ts')+source('api/yield-chart-api.ts')+source('api/create-yield-curve-chart.ts')+';return createYieldCurveChart;')(BaseChart,{type:'Line',isBuiltIn:true},Behavior,merge,v=>v);
const publicChart=factory({},{});assert.equal(publicChart.added[0].rows.length,121);
assert.throws(()=>publicChart.addSeries({type:'Candlestick',isBuiltIn:true}));
publicChart.addSeries({type:'Area',isBuiltIn:true},{lastValueVisible:true});
assert.equal(publicChart.added[1].options.lastValueVisible,true);assert.equal(publicChart.added[1].options.priceLineVisible,false);
queue.length=0;
const defaults={baseResolution:1,minimumTimeRange:120,startTimeRange:0};
assert.equal(helpers.defaultOptions.localization.priceFormatter(5.378),'5.378%');
assert.equal(helpers.defaultOptions.timeScale.ignoreWhitespaceIndices,true);
assert.equal(helpers.defaultOptions.leftPriceScale.visible,true);assert.equal(helpers.defaultOptions.rightPriceScale.visible,false);
assert.deepEqual(helpers.lineStyleDefaultOptionOverrides,{lastValueVisible:false,priceLineVisible:false});
const b=new Behavior();b.setOptions({yieldCurve:defaults,localization:{}});
const samples=[-12,-0,0,0.25,1,2,3,6,11.5,12,13,18,24,36,60,120,121.5,240,360,420];
const labels=samples.map(n=>b.formatHorzItem(n)), points=samples.map(time=>({time,timeWeight:0}));b.fillWeightsForPoints(points,0);
const weights=points.map(p=>p.timeWeight);queue.length=0;
const scaffold=[];
for(const [start,minimum,resolution,last] of [[0,120,1,0],[3,10,1,360],[3,10,12,360],[-3,-5,0.25,6],[20,10,1,0],[0,3,1,2.5],[0.5,5,1.5,7],[0,0,1,0]]) {
 const options={startTimeRange:start,minimumTimeRange:minimum,baseResolution:resolution};
 scaffold.push(helpers.generateWhitespaceData(helpers.buildWhitespaceState(options,last)).map(p=>p.time));
}
const custom=samples.map(n=>{b.updateFormatter({timeFormatter:v=>'X'+v});return b.formatHorzItem(n);});
const layerBehavior=new Behavior();layerBehavior.setOptions({yieldCurve:defaults,localization:{}});
const layer=new DataLayer(layerBehavior);
const owners=[0,1,2].map(id=>({id,seriesType:()=>id===2?'Area':'Line',isConflationEnabled:()=>false,customSeriesPlotValuesBuilder:()=>undefined,customSeriesWhitespaceCheck:()=>undefined}));
let current=[],support=[],signature='',rebuilds=0;const states=[];
function accept(update){if(update.timeScale.points)current=update.timeScale.points;}
function refresh(last) {const state=helpers.buildWhitespaceState(defaults,last),key=JSON.stringify(state);if(signature!==key){signature=key;support=helpers.generateWhitespaceData(state);rebuilds++;accept(layer.setSeriesData(owners[0],support));}}
layerBehavior.whitespaceInvalidated().subscribe(refresh);
function flush(){let guard=0;while(queue.length){assert.ok(++guard<20,'recursive whitespace rebuild');queue.shift()();}}
function snap(name){flush();states.push({name,keys:current.map(p=>p.time),weights:current.map(p=>p.timeWeight),support:support.map(p=>p.time),rebuilds});}
refresh(0);snap('empty');
accept(layer.setSeriesData(owners[1],[{time:1,value:5},{time:6,value:4},{time:360,value:3}]));snap('line');
accept(layer.setSeriesData(owners[2],[{time:3,value:5},{time:12,value:4},{time:240,value:3}]));snap('area');
accept(layer.updateSeriesData(owners[1],{time:6,value:4.2},true));snap('historical');
accept(layer.updateSeriesData(owners[2],{time:420,value:3.1},false));snap('extend');
accept(layer.updateSeriesData(owners[2],{time:420},false));snap('whitespace');
accept(layer.setSeriesData(owners[1],[{time:1,value:5},{time:6,value:4}]));snap('shorter');
accept(layer.removeSeries(owners[2]));snap('remove_longest');
accept(layer.setSeriesData(owners[1],[]));snap('clear');
const expected={labels,weights,scaffold,custom,states};
fs.mkdirSync('target/yield-curve',{recursive:true});fs.writeFileSync('target/yield-curve/source-contract.json',JSON.stringify(expected,null,2)+'\n');
if(!process.argv.includes('--source-only')){
 const run=spawnSync('cargo',['test','--offline','--lib','yield_curve_reference','--','--nocapture'],{encoding:'utf8'});assert.equal(run.status,0,run.stderr+run.stdout);
 const line=run.stdout.split('\n').find(l=>l.includes('YIELD_REFERENCE '));assert.ok(line,'native reference missing');assert.deepEqual(JSON.parse(line.split('YIELD_REFERENCE ')[1]),expected);
}
console.log(`${samples.length} labels/weights/custom labels, ${scaffold.length} scaffolds, ${states.length} real DataLayer/microtask lifecycle states pass.`);
console.log('Extent after shortening/removing/clearing:',states.slice(-3).map(s=>s.keys.at(-1)));

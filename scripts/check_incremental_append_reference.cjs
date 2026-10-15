// Executes the original DataLayer, plot-row creator, weights and TickMarks.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const ts = require(require.resolve('typescript', {paths:[root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText
        .replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
}
const ensureDefined = value => { assert.notEqual(value,undefined); return value; };
const lowerBound = new Function(`${source('helpers/algorithms.ts')}\nreturn lowerBound;`)();
const isWhitespaceData = new Function(`${source('model/data-consumer.ts')}\nreturn isWhitespaceData;`)();
const creators = new Function('ensureDefined','isWhitespaceData',`${source('model/get-series-plot-row-creator.ts')}\nreturn {getSeriesPlotRowCreator,isSeriesPlotRow};`)(ensureDefined,isWhitespaceData);
const DataLayer = new Function('lowerBound','ensureDefined','getSeriesPlotRowCreator','isSeriesPlotRow',`${source('model/data-layer.ts')}\nreturn DataLayer;`)(lowerBound,ensureDefined,creators.getSeriesPlotRowCreator,creators.isSeriesPlotRow);
const fillWeightsForPoints = new Function(`${source('model/horz-scale-behavior-time/types.ts')}\n${source('model/horz-scale-behavior-time/time-scale-point-weight-generator.ts')}\nreturn fillWeightsForPoints;`)();
const TickMarks = new Function('lowerBound','ensureDefined',`${source('model/tick-marks.ts')}\nreturn TickMarks;`)(lowerBound,ensureDefined);
const layer = new DataLayer({key:p=>p.timestamp,preprocessData:()=>{},createConverterToInternalObj:()=>time=>({timestamp:time}),fillWeightsForPoints});
const owners = [null,...[1,2].map(id=>({id,seriesType:()=>id===1?'Line':'Custom',customSeriesPlotValuesBuilder:()=>data=>[data.value],customSeriesWhitespaceCheck:()=>isWhitespaceData,isConflationEnabled:()=>false}))];
const events = [
    [1,true,false,1704067200,10],[2,true,false,1704067200,20],
    [1,false,false,1704067260,11],[2,false,false,1704067320,null],
    [2,false,false,1704067320,21],[1,false,false,1704067380,12],
    [2,false,false,1704067350,22],[1,false,true,1704067260,13],
    [2,false,false,1704153600,23],[1,false,false,1706832000,null],
    [1,false,false,1706832000,14],[1,true,false,1706918400,15],
    [2,false,false,1709251200,24],
];
const output=spawnSync(path.resolve('target/debug/examples/incremental_append_reference'),{encoding:'utf8'});
if(output.status!==0) throw new Error(output.stderr || 'Build incremental_append_reference first');
const actual=JSON.parse(output.stdout), rows = new Map();
let points=[];
for(const [step,[id,replace,historical,time,value]] of events.entries()) {
    const data=value===null?{time}:{time,value};
    const update=replace?layer.setSeriesData(owners[id],[data]):layer.updateSeriesData(owners[id],data,historical);
    if(update.timeScale.points) points=update.timeScale.points;
    for(const [owner,change] of update.series) rows.set(owner.id,change.data);
    const fulfilled=new Map([...rows.values()].flatMap(rows=>rows.map(row=>[row.index,true])));
    const ticks=[];
    for(const uniform of [false,true]) for(const filter of [false,true]) for(const spacing of [1,3,13]) {
        const marks=new TickMarks();marks.setUniformDistribution(uniform);marks.setTimeScalePoints(points,0);
        ticks.push(marks.build(1,spacing,filter,fulfilled,step).map(mark=>[mark.index,mark.weight]));
    }
    const expected={step,points:points.map(point=>[point.time.timestamp,point.timeWeight]),rows:[...rows].sort((a,b)=>a[0]-b[0]).map(([id,data])=>[id,data.map(row=>[row.index,row.value[3]])]),base:update.timeScale.baseIndex,ticks};
    assert.deepEqual(actual[step],expected,`event ${step}`);
}
console.log(`${actual.length} mixed built-in/custom update states and ${actual.length*12} tick selections match original TypeScript.`);

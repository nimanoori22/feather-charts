// Execute the frozen local TypeScript implementation against Rust's private core.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {createHash} = require('node:crypto'), {spawnSync} = require('node:child_process');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const manifest = JSON.parse(fs.readFileSync('docs/porting/conflation-source.json'));
for (const [file, expected] of Object.entries(manifest.sources)) {
    assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex'), expected, file);
}
const ts = require(require.resolve('typescript', {paths:[root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'), {
        compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
}
const DataConflater = new Function('PlotRowValueIndex',
    `${source('model/conflation/constants.ts')}\n${source('model/data-conflater.ts')}\nreturn DataConflater;`
)({Open:0,High:1,Low:2,Close:3});
const LineView = new Function('SeriesPaneViewBase','PlotRowValueIndex', `${source('model/series/line-pane-view-base.ts')}\nreturn LinePaneViewBase;`)(class {constructor(series){this._series=series;}},{Open:0,High:1,Low:2,Close:3});
class Probe extends LineView { _createRawItem(time,price){ return price; } }
const run = spawnSync('cargo',['test','--offline','--lib','model::data_conflater::tests','--','--nocapture','--test-threads=1'],
    {encoding:'utf8',maxBuffer:16*1024*1024});
assert.equal(run.status,0,run.stderr);
let count = 0;
for (const line of run.stdout.split('\n').filter(l=>l.includes('CONFLATION '))) {
    const c = JSON.parse(line.slice(line.indexOf('CONFLATION ')+11));
    const rows = c.rows.map(v=>({index:v[0],time:v[0],originalTime:v[0],value:v.slice(1,5),originalDataCount:v[5]}));
    const expected = new DataConflater().conflateByFactor(rows,c.factor)
        .map(r=>[r.index,...r.value,r.originalDataCount]);
    assert.deepEqual(c.actual,expected,`rows=${rows.length}, factor=${c.factor}`);
    const view = new Probe({barColorer:()=>null,conflatedBars:()=>({rows:()=>new DataConflater().conflateByFactor(rows,c.factor)})});
    view._fillRawPoints();
    assert.deepEqual(c.line, view._items, 'line-family representative prices');
    count++;
}
assert.equal(count,182,'Incomplete reference output');
console.log(`${count} aggregation/remainder/index/count states match the frozen TypeScript source.`);

const custom = spawnSync('cargo',['test','--offline','--test','public_custom_series','custom_conflation_preserves_reduction_order_and_no_reducer_passthrough','--','--exact','--nocapture'],{encoding:'utf8'});
assert.equal(custom.status,0,custom.stderr);
const record = JSON.parse(custom.stdout.split('\n').find(l=>l.startsWith('CUSTOM_CONFLATION ')).slice(18));
for (const [key,last] of [['initial',7],['latest',9]]) {
    const rows = Array.from({length:7},(_,i)=>{const value=i===6?last:i+1;return {index:i,time:i,originalTime:i,value:[value,value,value,value],data:{time:i,median:value}};});
    const result = new DataConflater().conflateByFactor(rows,8,(a,b)=>({time:a.data.time,median:a.data.median*10+b.data.median}),true,d=>[d.median]);
    assert.equal(record[key],result[0].data.median,`custom ${key}`);
}
console.log('Custom reduction order and public latest replacement match the frozen TypeScript source.');

let factors=0;
for (const line of run.stdout.split('\n').filter(l=>l.includes('CONFLATION_FACTOR '))) {
    const c=JSON.parse(line.slice(line.indexOf('CONFLATION_FACTOR ')+18));
    assert.equal(c.actual,new DataConflater().calculateConflationLevelWithSmoothing(c.spacing,c.dpr,c.threshold));factors++;
}
assert.equal(factors,60);console.log(`${factors} spacing/DPR/threshold selections match the actual TypeScript calculator.`);

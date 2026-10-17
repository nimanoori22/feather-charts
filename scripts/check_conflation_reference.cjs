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
const run = spawnSync('cargo',['test','--offline','--lib','model::data_conflater::tests::source_oracle','--','--exact','--nocapture'],
    {encoding:'utf8',maxBuffer:16*1024*1024});
assert.equal(run.status,0,run.stderr);
let count = 0;
for (const line of run.stdout.split('\n').filter(l=>l.startsWith('CONFLATION '))) {
    const c = JSON.parse(line.slice('CONFLATION '.length));
    const rows = c.rows.map(v=>({index:v[0],time:v[0],originalTime:v[0],value:v.slice(1,5),originalDataCount:v[5]}));
    const expected = new DataConflater().conflateByFactor(rows,c.factor)
        .map(r=>[r.index,...r.value,r.originalDataCount]);
    assert.deepEqual(c.actual,expected,`rows=${rows.length}, factor=${c.factor}`);
    count++;
}
assert.equal(count,140,'Incomplete reference output');
console.log(`${count} aggregation/remainder/index/count states match the frozen TypeScript source.`);

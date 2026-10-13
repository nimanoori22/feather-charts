const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
const code=ts.transpileModule(fs.readFileSync(path.join(root,'src/model/kinetic-animation.ts'),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
const Kinetic=new Function('ensureNotNull',`${code}\nreturn KineticAnimation;`)(value=>{assert.notEqual(value,null);return value;});
const output=spawnSync(path.resolve('target/debug/examples/kinetic_reference'),{encoding:'utf8'});
if(output.status!==0)throw new Error(output.stderr||'Build kinetic_reference first');
const cases=JSON.parse(output.stdout);
for(const [index,c] of cases.entries()) {
    const animation=new Kinetic(.2/c.spacing,7/c.spacing,.997,15/c.spacing);
    for(const [ms,position] of c.samples)animation.addPosition(position,ms);
    animation.start(c.position,c.start);
    const started=animation._animationStartPosition!==null;
    assert.equal(started,c.result!==null,`case ${index} start`);
    if(started)for(const [elapsed,position,finished] of c.result) {
        assert.ok(Math.abs(animation.getPosition(c.start+elapsed)-position)<1e-8,`case ${index} elapsed ${elapsed} position`);
        assert.equal(animation.finished(c.start+elapsed),finished,`case ${index} elapsed ${elapsed} completion`);
    }
}
console.log(`${cases.length} kinetic sampling/decay cases match the original TypeScript.`);

// Build neighboring TypeScript source into /tmp without modifying its checkout.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = path.resolve(process.argv[3] || '../lightweight-charts');
const output = path.resolve(process.argv[2] || '/tmp/feather-lwc-reference.js');
const resolveModule = name => require.resolve(name, { paths: [root] });
(async () => {
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'feather-lwc-reference-'));
    const compile = spawnSync(process.execPath, [resolveModule('typescript/lib/tsc.js'), '-p',
        path.join(root, 'tsconfig.prod.json'), '--outDir', temporary, '--incremental', 'false'], { encoding: 'utf8' });
    if (compile.status !== 0) throw new Error(compile.stdout + compile.stderr);
    const { rollup } = require(resolveModule('rollup'));
    const { nodeResolve } = require(resolveModule('@rollup/plugin-node-resolve'));
    const bundle = await rollup({ input: path.join(temporary, 'src/standalone.js'),
        plugins: [nodeResolve({ modulePaths: [path.join(root, 'node_modules')] })] });
    try { await bundle.write({ file: output, format: 'iife', name: 'LightweightCharts' }); }
    finally { await bundle.close(); }
    console.log(`Source bundle: ${output}; temporary compilation: ${temporary}`);
})().catch(e => { console.error(e); process.exitCode = 1; });

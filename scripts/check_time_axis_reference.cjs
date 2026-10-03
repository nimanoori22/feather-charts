// Runs actual original TimeAxisWidget methods without a DOM or drawing backend.
// cargo build --example time_axis_reference
// node scripts/check_time_axis_reference.cjs [path/to/lightweight-charts]
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const ts = require(require.resolve('typescript', { paths: [root] }));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root, 'src', file), 'utf8'), {
        compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020 },
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm, '').replace(/\bexport /g, '');
}
const suggest = new Function(`${source('gui/internal-layout-sizes-hints.ts')}\nreturn suggestTimeScaleHeight;`)();
let recorded;
const Widget = new Function('clearRect', 'makeFont', 'TextWidthCache',
    `${source('gui/time-axis-widget.ts')}\nreturn TimeAxisWidget;`)(
    (_ctx, _x, _y, _w, _h, color) => recorded.push(['bg', color]),
    (size, family, weight) => `${weight || ''} ${size}px ${family}`,
    class { reset() {} },
);
const result = spawnSync(path.resolve('target/debug/examples/time_axis_reference'), { encoding: 'utf8' });
if (result.status !== 0) throw new Error(result.stderr || 'Build the oracle example first');
const cases = JSON.parse(result.stdout);
function compare(a, b) {
    if (typeof a === 'number') assert.ok(Math.abs(a - b) < 1e-8, `${a} != ${b}`);
    else if (Array.isArray(a)) { assert.equal(a.length, b.length); a.forEach((v, i) => compare(v, b[i])); }
    else assert.equal(a, b);
}
for (const c of cases) {
    recorded = [];
    let rects = [];
    const ctx = {
        beginPath() { rects = []; },
        fillRect(x,y,w,h) { recorded.push(['rect',x/c.ratio,y/c.ratio,w/c.ratio,h/c.ratio,this.fillStyle]); },
        rect(x,y,w,h) { rects.push(['rect',x/c.ratio,y/c.ratio,w/c.ratio,h/c.ratio,this.fillStyle]); },
        fill() { recorded.push(...rects); },
        fillText(label,x,y) { recorded.push(['text',label,x,y,this.font.startsWith('bold'),this.fillStyle]); },
    };
    const marks = c.empty ? [] : [[1,'Jan',70,true],[50.25,'12:30',20,false],[99,'Feb',70,true]]
        .map(([coord,label,weight,needAlignCoordinate]) => ({coord,label,weight,needAlignCoordinate}));
    const options = { borderVisible:c.border, ticksVisible:c.ticks, borderColor:'#123', allowBoldLabels:c.bold };
    const widget = Object.create(Widget.prototype);
    Object.assign(widget, {
        _size:{width:c.plotWidth,height:40},
        _options:{fontSize:12,fontFamily:'sans-serif',textColor:'#456'},
        _rendererOptions:null,
        _chart:{ options:()=>({timeScale:options}),model:()=>({backgroundBottomColor:()=> '#ddd',timeScale:()=>({marks:()=>marks,options:()=>options})}) },
        _horzScaleBehavior:{maxTickMarkWeight:()=>70},
        _widthCache:{measureText:(ctx,label)=>[...label].length*(ctx.font.startsWith('bold')?8:6)},
    });
    // Fresh renderer-options cache needs only reset, never cached metrics.
    const rendererOptions = widget._getRendererOptions();
    rendererOptions.widthCache.reset = () => {};
    compare(suggest(widget.optimalHeight()),c.height);
    const scope = {context:ctx,bitmapSize:{width:c.plotWidth*c.ratio,height:40*c.ratio},horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio};
    widget._drawBackground(scope); widget._drawBorder(scope);
    widget._drawTickMarks({useBitmapCoordinateSpace:cb=>cb(scope),useMediaCoordinateSpace:cb=>cb({context:ctx})});
    compare(recorded,c.commands);
}
console.log(`${cases.length} time-axis cases match original height, background, ticks, label order, emphasis and alignment.`);

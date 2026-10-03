// Execute the actual original one-pane ChartWidget allocation, no DOM required.
// cargo build --example chart_layout_reference
// node scripts/check_chart_layout_reference.cjs
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText
        .replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
}
const hints=new Function('size',`${source('gui/internal-layout-sizes-hints.ts')}\nreturn {suggestChartSize,suggestPriceScaleWidth,suggestTimeScaleHeight};`)(v=>v);
const makeWidget=new Function('isChromiumBased','isWindows','ensureNotNull','size','suggestPriceScaleWidth','suggestTimeScaleHeight','SeparatorConstants','window',
    `${source('gui/chart-widget.ts')}\nreturn ChartWidget;`);
const output=spawnSync(path.resolve('target/debug/examples/chart_layout_reference'),{encoding:'utf8'});
if(output.status!==0)throw new Error(output.stderr||'Build the reference example first');
const cases=JSON.parse(output.stdout);
for(const c of cases) {
    // The source captures window by reference; obtain a per-case class instead.
    const W=makeWidget(()=>false,()=>false,v=>v,v=>v,hints.suggestPriceScaleWidth,hints.suggestTimeScaleHeight,{SeparatorHeight:1},{devicePixelRatio:c.ratio});
    const widget=Object.create(W.prototype),bounds=hints.suggestChartSize({width:c.width,height:c.height});
    let paneSize,leftWidth=0,rightWidth=0,timeSize;
    const state={leftPriceScale:()=>({options:()=>({visible:c.left})}),rightPriceScale:()=>({options:()=>({visible:c.right})})};
    Object.assign(widget,{_width:bounds.width,_height:bounds.height,_paneSeparators:[],
        _options:{leftPriceScale:{visible:c.left,minimumWidth:c.fractional?81.5:0},rightPriceScale:{visible:c.right,minimumWidth:0},timeScale:{visible:c.time,minimumHeight:c.fractional?31.5:0}},
        _model:{panes:()=>[state],setPaneHeight(_state,height){assert.equal(height,paneSize.height);},setWidth(width){assert.equal(width,paneSize.width);}},
        _paneWidgets:[{stretchFactor:()=>1,setState(){},state:()=>state,leftPriceAxisWidget:()=>({optimalWidth:()=>62}),rightPriceAxisWidget:()=>({optimalWidth:()=>80}),setSize(size){paneSize=size;},setPriceAxisSize(width,side){if(side==='left')leftWidth=width;else rightWidth=width;}}],
        _timeAxisWidget:{optimalHeight:()=>28,setSizes(size,left,right){timeSize=size;assert.equal(left,c.time?leftWidth:0);assert.equal(right,c.time?rightWidth:0);}},
    });
    widget._adjustSizeImpl();
    const actual=[paneSize.width,paneSize.height,leftWidth,rightWidth,timeSize.height];
    actual.forEach((v,i)=>assert.ok(Math.abs(v-c.result[i])<1e-9,`${v} != ${c.result[i]}`));
}
console.log(`${cases.length} allocation cases match original ChartWidget rounding, minima, visibility and undersized-pane behavior.`);

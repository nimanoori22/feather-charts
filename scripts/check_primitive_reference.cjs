// Execute the original algorithms, with DOM/rendering dependencies replaced by fixtures.
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
const priority = { Range: 0, Line: 1, Point: 2 };
const better = new Function('HitTestPriority', `${source('model/internal-hit-test.ts')}\nreturn isBetterHit;`)(priority);
const paneHit = new Function('HitTestPriority', 'isBetterHit', `${source('model/pane-hit-test.ts')}\nreturn hitTestPane;`)(priority, better);
const PriceAxis = new Function(`${source('gui/price-axis-widget.ts')}\nreturn PriceAxisWidget;`)();
const drawing=new Function(`${source('gui/draw-functions.ts')}\nreturn {drawBackground,drawForeground,drawSourceViews,drawPaneViews};`)();
const PaneWidget=new Function('ensureNotNull',...Object.keys(drawing),`${source('gui/pane-widget.ts')}\nreturn {PaneWidget,sourcePaneViews,sourceBottomPaneViews,sourceTopPaneViews};`)(v=>v,...Object.values(drawing));
const PriceRangeImpl = new Function(`${source('model/price-range-impl.ts')}\nreturn PriceRangeImpl;`)();
const AutoscaleInfoImpl = new Function('PriceRangeImpl',`${source('model/autoscale-info-impl.ts')}\nreturn AutoscaleInfoImpl;`)(PriceRangeImpl);
// Extract a transpiled method body, retaining the source algorithm verbatim.
function method(file,name) {
    const text=source(file); const start=text.indexOf(`\n    ${name}(`); assert.ok(start>=0);
    const open=text.indexOf('{',start); let depth=1,end=open+1;
    for(;depth && end<text.length;end++) {if(text[end]==='{')depth++;if(text[end]==='}')depth--;}
    assert.equal(depth,0); return text.slice(open+1,end-1);
}
const autoscale=new Function('PriceRangeImpl','AutoscaleInfoImpl','isInteger','PlotRowValueIndex',
    `return function(startTimePoint,endTimePoint){${method('model/series.ts','_autoscaleInfoImpl')}}`)(PriceRangeImpl,AutoscaleInfoImpl,Number.isInteger,{Close:3,Low:2,High:1});
const result = spawnSync(path.resolve('target/debug/examples/primitive_reference'), {encoding:'utf8', maxBuffer:100*1024*1024});
assert.equal(result.status,0,result.stderr);
const cases = JSON.parse(result.stdout);
for (const c of cases) {
    if(c.kind==='marker') {
        const primitive={zOrder:['bottom','normal','top'][c.layer],distance:0,hitTestPriority:c.priority===0?0:2,externalId:'primitive'};
        const marker={zOrder:'normal',itemType:'marker',externalId:'marker'};
        const owner={paneViews:()=>[],primitiveHitTest:()=>c.markerFirst?[marker,primitive]:[primitive,marker]};
        const actual=paneHit({orderedSources:()=>[owner]},50,50);
        assert.equal(actual.object.externalId,c.winner,JSON.stringify(c));continue;
    }
    if(c.kind==='draw') {
        const output=[];
        function owner(name) {
            const view=i=>({renderer:()=>({drawBackground:()=>output.push(`bg:${name}:${i}`),draw:()=>output.push(`fg:${name}:${i}`)})});
            return {paneViews:()=>[view(1)],bottomPaneViews:()=>[view(0)],topPaneViews:()=>[view(2)]};
        }
        const [a,b,pane]=['a','b','pane'].map(owner);
        const widget=Object.create(PaneWidget.PaneWidget.prototype);
        widget._state={primitives:()=>[pane],orderedSources:()=>[a,b],orderedSourcesForRendering:()=>c.raised?[b,a]:[a,b],model:()=>({hoveredSource:()=>null})};
        widget._hoveredSplitSource=()=>c.filled && c.raised?a:null;
        widget._hoveredSplitSourcePaneViews=()=>({normalPaneViews:a.paneViews(),topPaneViews:[]});
        for(const getter of [PaneWidget.sourceBottomPaneViews,PaneWidget.sourcePaneViews,PaneWidget.sourceTopPaneViews]) widget._drawSources(null,getter);
        assert.deepEqual(output,c.output,JSON.stringify(c)); continue;
    }
    if(c.kind==='autoscale') {
        const receiver={_seriesType:'Line',seriesType:()=> 'Line',_data:{isEmpty:()=>c.empty,minMaxOnRangeCached:()=>({min:100,max:109})},
            _primitives:[0,1].map(i=>({autoscaleInfo:()=>({priceRange:c.rangeMask & (1<<i)?{minValue:20+i*60,maxValue:130+i*20}:null,
                margins:c.marginMask & (1<<i)?{above:7-i*4,below:7-i*4}:null})}))};
        const actual=autoscale.call(receiver,0,9)?.toRaw() ?? null;
        // Raw TS omits null fields; normalize the same optional contract.
        assert.deepEqual(actual && {priceRange:actual.priceRange??null,margins:actual.margins??null},c.output,JSON.stringify(c));
        continue;
    }
    if (c.kind === 'labels') {
        const views = c.coordinates.map((y,i) => ({coordinate:()=>y, render:y,
            height:()=>17, getRenderCoordinate(){return this.render;},setRenderCoordinate(y){this.render=y;}}));
        if(c.fixed) views[1].render=35;
        const widget = Object.create(PriceAxis.prototype); widget._size={height:100};
        widget._fixLabelOverlap(views.filter((_,i)=>!c.fixed || i!==1),{},50);
        assert.deepEqual(views.map(v=>v.render),c.output,JSON.stringify(c));
        continue;
    }
    const layers=['bottom','normal','top'];
    const hits=[{owner:c.owner,zOrder:layers[c.layer],isBackground:c.background,hitTestPriority:c.priority,distance:c.distance,externalId:'first'},
        {owner:c.secondOwner,zOrder:layers[c.secondLayer],distance:3,externalId:'second'}];
    const sources=[0,1].map(i=>({primitiveHitTest:()=>hits.filter(h=>h.owner===i),
        paneViews:()=> c.mask & (1<<i) ? [{hitTest:()=>({distance:i*2,priority:0,externalId:`builtin${i}`})}] : []}));
    const pane={orderedSources:()=>sources,primitiveHitTest:()=>hits.filter(h=>h.owner===2)};
    const actual=paneHit(pane,50,50);
    assert.equal(actual?.object.externalId ?? null,c.winner,JSON.stringify(c));
}
console.log(`${cases.length} primitive overlap, autoscale, label-placement and drawing-order cases match actual TypeScript.`);

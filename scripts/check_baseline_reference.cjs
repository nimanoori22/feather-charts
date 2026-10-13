// Execute actual upstream Area renderer, gradient cache and line walker.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const root = path.resolve(process.argv[2] || '../lightweight-charts');
const ts = require(require.resolve('typescript', {paths:[root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'), {
        compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
}
const draw = new Function(`${source('renderers/draw-line.ts')}\nreturn {setLineStyle};`)();
const walkLine = new Function('LineType', `${source('renderers/walk-line.ts')}\nreturn walkLine;`)({Simple:0,WithSteps:1,Curved:2});
const GradientStyleCache = new Function('clamp', `${source('renderers/gradient-style-cache.ts')}\nreturn GradientStyleCache;`)((v,a,b)=>Math.min(Math.max(v,a),b));
const Base = new Function('BitmapCoordinatesPaneRenderer','setLineStyle','walkLine', `${source('renderers/area-renderer-base.ts')}\nreturn PaneRendererAreaBase;`)(class{},draw.setLineStyle,walkLine);
const Area = new Function('PaneRendererAreaBase','GradientStyleCache', `${source('renderers/baseline-renderer-area.ts')}\nreturn PaneRendererBaselineArea;`)(Base,GradientStyleCache);
const markers = new Function(`${source('renderers/draw-series-point-markers.ts')}\nreturn drawSeriesPointMarkers;`)();
const length = new Function(`${source('renderers/draw-line.ts')}\nreturn getDashPatternLength;`)();
const LineBase = new Function('BitmapCoordinatesPaneRenderer','setLineStyle','walkLine','getDashPatternLength','drawSeriesPointMarkers',`${source('renderers/line-renderer-base.ts')}\nreturn PaneRendererLineBase;`)(class{},draw.setLineStyle,walkLine,length,markers);
const Line = new Function('PaneRendererLineBase','GradientStyleCache',`${source('renderers/baseline-renderer-line.ts')}\nreturn PaneRendererBaselineLine;`)(LineBase,GradientStyleCache);
const result=spawnSync(path.resolve('target/debug/examples/baseline_reference'),{encoding:'utf8',maxBuffer:64*1024*1024});
assert.equal(result.status,0,result.stderr || String(result.error || "Build baseline_reference first"));
const cases=JSON.parse(result.stdout);
const points=[[10,50],[30,20],[40,60],[70,10],[90,40]];
const fillsColors=[['green','white','white','red'],['blue','white','white','red'],['blue','white','white','red'],['green','black','black','red'],['green','white','white','red']];
const strokeColors=[['green','red'],['green','red'],['blue','yellow'],['blue','yellow'],['green','red']];
function near(a,b) {
    if(typeof a==='number') assert.ok(Math.abs(a-b)<1e-8,`${a} != ${b}`);
    else if(Array.isArray(a)) {assert.equal(a.length,b.length);a.forEach((v,i)=>near(v,b[i]));}
    else if(a&&typeof a==='object') {assert.deepEqual(Object.keys(a).sort(),Object.keys(b).sort());Object.keys(a).forEach(k=>near(a[k],b[k]));}
    else assert.equal(a,b);
}
for(const c of cases) {
    let operations=[],arcs=[];
    const fills=[],strokes=[],markerGroups=[];
    const context={lineDashOffset:0,dash:[],
        beginPath(){operations=[];arcs=[];}, moveTo(x,y){operations.push(['M',x,y]);},
        lineTo(x,y){operations.push([operations.length?'L':'M',x,y]);},
        bezierCurveTo(...args){operations.push(['C',...args]);},closePath(){},setLineDash(dash){this.dash=dash;},
        arc(x,y,r){arcs.push([x,y,r]);},
        createLinearGradient(_x,top,_x2,bottom){return {top,bottom,stops:[],addColorStop(offset,color){this.stops.push([offset,color]);}};},
        fill(){if(arcs.length)markerGroups.push({gradient:this.fillStyle,markers:arcs});else fills.push({path:operations,gradient:this.fillStyle});},
        stroke(){strokes.push({path:operations,gradient:this.strokeStyle,width:this.lineWidth,offset:this.lineDashOffset,dash:this.dash});}
    };
    const data={items:points.map(([x,y],i)=>({x,y,topFillColor1:fillsColors[i][0],topFillColor2:fillsColors[i][1],bottomFillColor1:fillsColors[i][2],bottomFillColor2:fillsColors[i][3],topLineColor:strokeColors[i][0],bottomLineColor:strokeColors[i][1]})),
        visibleRange:{from:c.from,to:c.to},barWidth:6,lineType:c.kind,lineWidth:3,lineStyle:2,
        topCoordinate:c.top??undefined,bottomCoordinate:c.bottom??undefined,baseLevelCoordinate:c.base,invertFilledArea:false};
    const scope={context,horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio,mediaSize:{height:80},bitmapSize:{height:80*c.ratio}};
    const area=new Area();area.setData(data);area._drawImpl(scope);
    const line=new Line();line.setData({...data,lineType:c.visible?c.kind:undefined,pointMarkersRadius:3});line._drawImpl(scope);
    // Strip mock addColorStop methods to compare owned serializable commands.
    try {near(c.fills,JSON.parse(JSON.stringify(fills)));near(c.strokes,JSON.parse(JSON.stringify(strokes)));near(c.markers,JSON.parse(JSON.stringify(markerGroups)));}
    catch(e){throw new Error(`Baseline case ${JSON.stringify(c)}: ${e.message}`);}
}
console.log(`${cases.length} Baseline fill/stroke/marker paths and ordered gradient stops match upstream TypeScript.`);

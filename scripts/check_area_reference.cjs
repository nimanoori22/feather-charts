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
const Area = new Function('PaneRendererAreaBase','GradientStyleCache', `${source('renderers/area-renderer.ts')}\nreturn PaneRendererArea;`)(Base,GradientStyleCache);
const result=spawnSync(path.resolve('target/debug/examples/area_reference'),{encoding:'utf8',maxBuffer:16*1024*1024});
assert.equal(result.status,0,result.stderr || String(result.error || "Build area_reference first"));
const cases=JSON.parse(result.stdout);
const points=[[10,50],[30,20],[40,60],[70,10],[90,40]];
const colors=[['red','white'],['blue','white'],['blue','white'],['red','black'],['green','black']];
function near(a,b) {
    if(typeof a==='number') assert.ok(Math.abs(a-b)<1e-8,`${a} != ${b}`);
    else if(Array.isArray(a)) {assert.equal(a.length,b.length);a.forEach((v,i)=>near(v,b[i]));}
    else if(a&&typeof a==='object') {assert.deepEqual(Object.keys(a).sort(),Object.keys(b).sort());Object.keys(a).forEach(k=>near(a[k],b[k]));}
    else assert.equal(a,b);
}
for(const c of cases) {
    let operations=[];
    const fills=[];
    const context={
        beginPath(){operations=[];}, moveTo(x,y){operations.push(['M',x,y]);},
        lineTo(x,y){operations.push([operations.length?'L':'M',x,y]);},
        bezierCurveTo(...args){operations.push(['C',...args]);},closePath(){},setLineDash(){},
        createLinearGradient(_x,top,_x2,bottom){return {top,bottom,stops:[],addColorStop(offset,color){this.stops.push([offset,color]);}};},
        fill(){fills.push({path:operations,gradient:{top:this.fillStyle.top,bottom:this.fillStyle.bottom,stops:this.fillStyle.stops}});}
    };
    const area=new Area();
    area.setData({items:points.map(([x,y],i)=>({x,y,topColor:colors[i][0],bottomColor:colors[i][1]})),
        visibleRange:{from:c.from,to:c.to},barWidth:6,lineType:c.kind,lineWidth:3,lineStyle:2,
        topCoordinate:c.top,baseLevelCoordinate:null,invertFilledArea:c.inverted});
    area._drawImpl({context,horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio,mediaSize:{height:80},bitmapSize:{height:80*c.ratio}});
    near(c.fills,fills);
}
console.log(`${cases.length} Area fill paths, independent styles and gradient cases match upstream TypeScript.`);

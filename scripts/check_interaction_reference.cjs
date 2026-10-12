// Execute original synchronous scale and crosshair renderer methods.
// cargo build --example interaction_reference
// node scripts/check_interaction_reference.cjs
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText
        .replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
}
const Scale=new Function('clamp','isInteger','ensureNotNull',`${source('model/time-scale.ts')}\nreturn TimeScale;`)((v,a,b)=>Math.min(Math.max(v,a),b),Number.isInteger,v=>v);
class Base {}
let paths;
const lines=new Function(`${source('renderers/draw-line.ts')}\nreturn {setLineStyle,drawVerticalLine,drawHorizontalLine};`)();
const Renderer=new Function('BitmapCoordinatesPaneRenderer','setLineStyle','drawVerticalLine','drawHorizontalLine',`${source('renderers/crosshair-renderer.ts')}\nreturn CrosshairRenderer;`)(Base,lines.setLineStyle,lines.drawVerticalLine,lines.drawHorizontalLine);
class Series {
    constructor(value) {this.value=value;}
    visible(){return true;}
    priceScale(){return priceScale;}
    firstValue(){return {value:1};}
    bars(){return {contains:()=>true,valueAt:()=>({value:Array.isArray(this.value)?this.value:[this.value,this.value,this.value,this.value]})};}
}
const priceScale={firstValue:()=>1,isEmpty:()=>false,priceToCoordinate:p=>p,coordinateToPrice:y=>y};
const Magnet=new Function('ensure','CrosshairMode','PlotRowValueIndex','Series',`${source('model/magnet.ts')}\nreturn Magnet;`)(v=>v,{Normal:0,Magnet:1,MagnetOHLC:3},{Open:0,High:1,Low:2,Close:3},Series);
const PriceLabel=new Function(`${source('renderers/price-axis-view-renderer.ts')}\nreturn PriceAxisViewRenderer;`)();
const TimeLabel=new Function('ensureNotNull',`${source('renderers/time-axis-view-renderer.ts')}\nreturn TimeAxisViewRenderer;`)(v=>v);
const output=spawnSync(path.resolve('target/debug/examples/interaction_reference'),{encoding:'utf8'});
if(output.status!==0)throw new Error(output.stderr||'Build the reference example first');
const cases=JSON.parse(output.stdout);
function near(a,b) {
    if(Array.isArray(a)){assert.equal(a.length,b.length);a.forEach((v,i)=>near(v,b[i]));}
    else assert.ok(Math.abs(a-b)<1e-8,`${a} != ${b}`);
}
for(const c of cases) {
    if(c.kind==='price-label') {
        const renderer=new PriceLabel({visible:true,text:'98.11',tickVisible:true,separatorVisible:false},{coordinate:c.coordinate,additionalPaddingTop:2,additionalPaddingBottom:2});
        const g=renderer._calculateGeometry({context:{},bitmapSize:{width:100*c.ratio,height:80*c.ratio},mediaSize:{width:100,height:80},horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio},{fontSize:12,borderSize:1,tickLength:5,paddingTop:2.5,paddingBottom:2.5,paddingInner:5,paddingOuter:5},{measureText:()=>35,yMidCorrection:()=>0},c.left?'right':'left').bitmap;
        near([(c.left?g.xOutside:g.xInside)/c.ratio,g.yTop/c.ratio,g.totalWidth/c.ratio,g.totalHeight/c.ratio,Math.min(g.xInside,g.xTick)/c.ratio,g.yMid/c.ratio,Math.abs(g.xTick-g.xInside)/c.ratio,g.tickHeight/c.ratio],c.result);
    } else if(c.kind==='time-label') {
        const renderer=new TimeLabel();renderer.setData({width:100,text:'98.11',coordinate:c.coordinate,visible:true,tickVisible:true});
        const points=[];const context={beginPath(){},moveTo(x,y){points.push([x,y]);},lineTo(x,y){points.push([x,y]);},arcTo(x,y,a,b){points.push([x,y],[a,b]);},fill(){},fillRect(){},translate(){},fillText(){}};
        renderer.draw({useMediaCoordinateSpace:f=>f({context}),useBitmapCoordinateSpace:f=>f({context,horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio})},{fontSize:12,borderSize:1,tickLength:5,paddingTop:3,paddingBottom:3,paddingHorizontal:9,widthCache:{measureText:()=>35,yMidCorrection:()=>0}});
        const xs=points.map(p=>p[0]),ys=points.map(p=>p[1]);
        near([Math.min(...xs)/c.ratio,Math.min(...ys)/c.ratio,(Math.max(...xs)-Math.min(...xs))/c.ratio,(Math.max(...ys)-Math.min(...ys))/c.ratio],c.result);
    } else if(c.kind==='navigation') {
        const scale=Object.create(Scale.prototype);
        Object.assign(scale,{_width:c.width,_barSpacing:c.initial[0],_rightOffset:c.initial[1],_baseIndexOrNull:99,_points:Array(100),_options:{minBarSpacing:0.5,maxBarSpacing:0,fixLeftEdge:c.fixed,fixRightEdge:c.fixed,rightBarStaysOnScroll:c.stay},_model:{recalculateAllPanes(){},lightUpdate(){}},_scrollStartPoint:null,_commonTransitionStartState:null});
        scale.zoom(c.width*0.4,c.zoom);near([scale.barSpacing(),scale.rightOffset()],c.zoomed);
        scale.startScroll(20);scale.scrollTo(65);scale.endScroll();near([scale.barSpacing(),scale.rightOffset()],c.scrolled);
    } else if(c.kind==='ohlc-magnet') {
        const pane={defaultPriceScale:()=>priceScale,dataSources:()=>c.values.map(v=>new Series(v)),isOverlay:()=>false};
        near(new Magnet({mode:c.mode}).align(c.origin,0,pane),c.result);
    } else if(c.kind==='magnet') {
        const pane={defaultPriceScale:()=>priceScale,dataSources:()=>c.candidates.map(v=>new Series(v)),isOverlay:()=>false};
        near(new Magnet({mode:1}).align(10,0,pane),c.result);
    } else {
        paths=[];
        const renderer=new Renderer({x:20.2,y:30.8,vertLine:{visible:true,lineWidth:c.width},horzLine:{visible:true,lineWidth:c.width}});
        let first,last; const context={beginPath(){},setLineDash(){},moveTo(x,y){first=[x,y];},lineTo(x,y){last=[x,y];},stroke(){paths.push([...first,...last,this.lineWidth]);}};
        renderer._drawImpl({context,bitmapSize:{width:100*c.ratio,height:60*c.ratio},horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio});near(paths,c.result);
    }
}
console.log(`${cases.length} navigation, crosshair-line, floating-label and magnet cases match original TypeScript methods.`);

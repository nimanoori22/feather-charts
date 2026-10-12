// Compare Rust fixtures with the actual upstream line, label and overlap methods.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const draw=new Function(`${source('renderers/draw-line.ts')}\nreturn {setLineStyle,drawHorizontalLine};`)();
const Line=new Function('BitmapCoordinatesPaneRenderer','setLineStyle','drawHorizontalLine',`${source('renderers/horizontal-line-renderer.ts')}\nreturn HorizontalLineRenderer;`)(class{},draw.setLineStyle,draw.drawHorizontalLine);
const Label=new Function(`${source('renderers/price-axis-view-renderer.ts')}\nreturn PriceAxisViewRenderer;`)();
const Series=new Function('PriceDataSource','PlotRowValueIndex','MismatchDirection',`${source('model/series.ts')}\nreturn Series;`)(class{},{Close:3},{NearestLeft:-1});
const Axis=new Function(`${source('gui/price-axis-widget.ts')}\nreturn PriceAxisWidget;`)();
const output=spawnSync(path.resolve('target/debug/examples/price_lines_reference'),{encoding:'utf8'});
if(output.status!==0)throw new Error(output.stderr||'Build price_lines_reference first');
const cases=JSON.parse(output.stdout);
function near(a,b){if(Array.isArray(a)){assert.equal(a.length,b.length);a.forEach((v,i)=>near(v,b[i]));}else assert.ok(Math.abs(a-b)<1e-8,`${a} != ${b}`);}
for(const c of cases){
 let actual;
 if(c.kind==='line'){
  let first,last;actual=[];
  const context={beginPath(){},setLineDash(){},moveTo(x,y){first=[x,y];},lineTo(x,y){last=[x,y];},stroke(){actual=[...first,...last,this.lineWidth];}};
  const line=new Line();line.setData({visible:true,y:c.y,color:'red',lineWidth:c.width,lineStyle:2});
  line._drawImpl({context,bitmapSize:{width:100*c.ratio,height:60*c.ratio},horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio});
 }else if(c.kind==='label'){
  const [fontSize,borderSize,tickLength,paddingTop,paddingBottom,paddingInner,paddingOuter]=c.options;
  const label=new Label({visible:true,text:c.pane?'Entry':'123.45',tickVisible:!c.pane,separatorVisible:c.pane,moveTextToInvisibleTick:c.pane},{coordinate:c.y,additionalPaddingTop:0,additionalPaddingBottom:0});
  const g=label._calculateGeometry({context:{},mediaSize:{width:100,height:80},bitmapSize:{width:100*c.ratio,height:80*c.ratio},horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio},{fontSize,borderSize,tickLength,paddingTop,paddingBottom,paddingInner,paddingOuter},{measureText:(_,text)=>text.length*7,yMidCorrection:()=>0},c.left!==c.pane?'right':'left');
  const b=g.bitmap,m=g.media;actual=[(g.alignRight?b.xOutside:b.xInside)/c.ratio,b.yTop/c.ratio,b.totalWidth/c.ratio,b.totalHeight/c.ratio,m.xText,(m.yTop+m.yBottom)/2];
 }else if(c.kind==='last'){
  const rows=[0,2,5,9,12].map(index=>({index,value:[100+index,100+index,100+index,100+index]}));
  const scale={isEmpty:()=>false,priceToCoordinate:p=>p,formatPrice:p=>String(p),formatPriceAbsolute:p=>String(p),formatPricePercentage:p=>String(p)};
  const series=Object.create(Series.prototype);
  Object.assign(series,{_model:{timeScale:()=>({isEmpty:()=>false,visibleStrictRange:()=>({right:()=>c.right})})},
   _data:{isEmpty:()=>false,last:()=>rows.at(-1),search:right=>rows.filter(r=>r.index<=right).at(-1),valueAt:index=>rows.find(r=>r.index===index)},
   model(){return this._model;},priceScale:()=>scale,firstValue:()=>({value:100}),barColorer:()=>({barStyle:()=>({barColor:'red'})})});
  const last=series.lastValueData(c.global);actual=[last.index,last.price];assert.equal(last.color,'red');
 }else{
  const views=c.coordinates.map(y=>({coordinate:()=>y,height:()=>c.labelHeight,render:y,getRenderCoordinate(){return this.render;},setRenderCoordinate(y){this.render=y;}}));
  const axis=Object.create(Axis.prototype);axis._size={height:c.height};axis._fixLabelOverlap(views,{},c.center);actual=views.map(v=>v.render);
 }
 near(actual,c.result);
}
console.log(`${cases.length} last-value selection, price-line, axis/pane label geometry and overlap cases match upstream TypeScript.`);

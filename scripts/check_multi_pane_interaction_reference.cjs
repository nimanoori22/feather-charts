// Exercise original CrosshairPaneView/Renderer and PaneSeparator methods.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
function source(file){return ts.transpileModule(fs.readFileSync(path.join(root,'src',file),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');}
const lines=new Function(`${source('renderers/draw-line.ts')}\nreturn {setLineStyle,drawVerticalLine,drawHorizontalLine};`)();
const Renderer=new Function('BitmapCoordinatesPaneRenderer','setLineStyle','drawVerticalLine','drawHorizontalLine',`${source('renderers/crosshair-renderer.ts')}\nreturn CrosshairRenderer;`)(class {},lines.setLineStyle,lines.drawVerticalLine,lines.drawHorizontalLine);
const View=new Function('CrosshairMode','CrosshairRenderer',`${source('views/pane/crosshair-pane-view.ts')}\nreturn CrosshairPaneView;`)({Hidden:2},Renderer);
const Separator=new Function('clamp',`${source('gui/pane-separator.ts')}\nreturn PaneSeparator;`)((v,a,b)=>Math.min(Math.max(v,a),b));
const VolumeFormatter=new Function('FormatterBase',`${source('formatters/volume-formatter.ts')}\nreturn VolumeFormatter;`)(class {});
const output=spawnSync(path.resolve('target/debug/examples/multi_pane_interaction_reference'),{encoding:'utf8'});
if(output.status!==0)throw new Error(output.stderr||'Build multi_pane_interaction_reference first');
const cases=JSON.parse(output.stdout);
for(const c of cases){
    if(c.kind==='volume'){assert.equal(new VolumeFormatter(c.precision).format(c.value),c.result);continue;}
    let actual;
    if(c.kind==='divider'){
        let top=2,bottom=1;
        const widget=(upper)=>({state:()=>({stretchFactor:()=>upper?top:bottom,setStretchFactor:v=>{if(upper)top=v;else bottom=v;}}),getSize:()=>({height:upper?c.upperHeight:c.lowerHeight})});
        const separator=Object.create(Separator.prototype);
        Object.assign(separator,{_topPane:widget(true),_bottomPane:widget(false),_handle:{backgroundElement:{style:{}}},_resizeInfo:null,_chartWidget:{model:()=>({fullUpdate(){}})}});
        separator._mouseDownEvent({pageY:0});separator._pressedMouseMoveEvent({pageY:c.delta});
        actual=[top,bottom];
    }else{
        const options={mode:0,horzLine:{width:1,style:3,color:'#758696',visible:true},vertLine:{width:1,style:3,color:'#758696',visible:true}};
        const pane={model:()=>({options:()=>({crosshair:options})})};
        const view=new View({visible:()=>true,horzLineVisible:()=>c.active===c.pane,vertLineVisible:()=>true,appliedX:()=>20.2,appliedY:()=>30.8},pane);
        const renderer=view.renderer(pane);actual=[];let first,last;
        const context={beginPath(){},setLineDash(){},moveTo(x,y){first=[x,y];},lineTo(x,y){last=[x,y];},stroke(){actual.push([...first,...last,this.lineWidth]);}};
        renderer._drawImpl({context,bitmapSize:{width:100*c.ratio,height:60*c.ratio},horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio});
    }
    function near(a,b){if(Array.isArray(a)){assert.equal(a.length,b.length);a.forEach((v,i)=>near(v,b[i]));}else assert.ok(Math.abs(a-b)<1e-8,`${a} != ${b}`);}
    near(actual,c.result);
}
console.log(`${cases.length} cross-pane crosshair, divider and volume formatter cases match original TypeScript methods.`);

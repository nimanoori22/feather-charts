// Read-only differential oracle: execute the original TypeScript walker/markers.
// cargo build --example line_reference
// node scripts/check_line_reference.cjs [path/to/lightweight-charts] [--visual]
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const root = path.resolve(process.argv[2] && !process.argv[2].startsWith('--') ? process.argv[2] : '../lightweight-charts');
const ts = require(require.resolve('typescript', {paths:[root]}));
function source(file) {
    const compiled = ts.transpileModule(fs.readFileSync(path.join(root, 'src', file), 'utf8'), {
        compilerOptions:{target:ts.ScriptTarget.ES2020, module:ts.ModuleKind.ES2020}
    }).outputText;
    return compiled.replace(/^import .*;$/gm, '').replace(/\bexport /g, '');
}
const walker = source('renderers/walk-line.ts');
const markerSource = source('renderers/draw-series-point-markers.ts');
const {walkLine, drawSeriesPointMarkers} = new Function('LineType', `${walker}\n${markerSource}\nreturn {walkLine,drawSeriesPointMarkers};`)({Simple:0,WithSteps:1,Curved:2});
const result = spawnSync(path.resolve('target/debug/examples/line_reference'), {encoding:'utf8',maxBuffer:16*1024*1024});
if (result.status !== 0) throw new Error(result.stderr || 'Build the line_reference example first');
const cases = JSON.parse(result.stdout);
const items = [[10,50,'red'],[30,20,'blue'],[40,60,'blue'],[70,10,'red'],[90,40,'green']].map(([x,y,color])=>({x,y,color}));
const patterns = [[],[1,1],[2,2],[6,6],[1,4]];
function near(actual, expected, location='result') {
    if (typeof actual === 'number') {
        assert.ok(Math.abs(actual-expected)<1e-8, `${location}: ${actual} != ${expected}`);
    } else if (Array.isArray(actual)) {
        assert.equal(actual.length,expected.length,location);
        actual.forEach((v,i)=>near(v,expected[i],`${location}[${i}]`));
    } else if (actual && typeof actual === 'object') {
        assert.deepEqual(Object.keys(actual).sort(),Object.keys(expected).sort());
        for (const key of Object.keys(actual)) near(actual[key],expected[key],`${location}.${key}`);
    } else assert.equal(actual,expected,location);
}
for (const c of cases) {
    let operations = [], phase=0;
    const strokes=[], markers=[];
    const dash=patterns[c.style].map(v=>v*3*c.ratio);
    const context={
        beginPath(){operations=[];},
        moveTo(x,y){operations.push(['M',x,y]);},
        lineTo(x,y){operations.push([operations.length?'L':'M',x,y]);},
        bezierCurveTo(...args){operations.push(['C',...args]);},
        get lineDashOffset(){return phase;}, set lineDashOffset(v){phase=v;},
        fill(){}, arc(x,y,r){markers.push({color:this.fillStyle,center:[x,y],radius:r});}
    };
    const scope={context,horizontalPixelRatio:c.ratio,verticalPixelRatio:c.ratio};
    walkLine(scope,items,c.kind,{from:c.from,to:c.to},6,(_,item)=>item.color,(_,color)=>{
        strokes.push({color,width:3*c.ratio,dash,offset:phase,path:operations});
    },dash.reduce((sum,n)=>sum+n,0));
    drawSeriesPointMarkers(scope,items,3.5,{from:c.from,to:c.to},(_,item)=>item.color);
    near({strokes:c.strokes,markers:c.markers},{strokes,markers},`case ${JSON.stringify([c.kind,c.style,c.ratio,c.from,c.to])}`);
}
console.log(`${cases.length} differential cases match original TypeScript paths, colors, dash phase, and markers.`);

if (process.argv.includes('--visual')) (async()=>{
    const puppeteer = require(require.resolve('puppeteer',{paths:[root]}));
    const browser = await puppeteer.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
    try {
        const page=await browser.newPage();await page.setViewport({width:1000,height:760,deviceScaleFactor:1});
        await page.setContent('<body style="background:#fff;font:16px sans-serif"><h3>Original TypeScript (left) / Iced-adapted Rust paths (right)</h3><div id="charts" style="display:grid;grid-template-columns:460px 460px;gap:10px"></div></body>');
        const selected=cases.filter(c=>c.ratio===2&&c.from===0&&c.to===5&&c.style===2);
        await page.evaluate(({walker,markerSource,items,selected})=>{
            const {walkLine,drawSeriesPointMarkers}=new Function('LineType',`${walker}\n${markerSource}\nreturn {walkLine,drawSeriesPointMarkers};`)({Simple:0,WithSteps:1,Curved:2});
            for (const c of selected) for (const original of [true,false]) {
                const box=document.createElement('div');box.textContent=['Simple','Stepped','Curved'][c.kind]+(original?' / original':' / Rust');
                const canvas=document.createElement('canvas');canvas.width=240;canvas.height=160;canvas.style='width:440px;height:205px;border:1px solid #ddd';box.append(canvas);document.querySelector('#charts').append(box);
                const ctx=canvas.getContext('2d');ctx.lineWidth=6;ctx.lineCap='butt';ctx.lineJoin='round';ctx.setLineDash([12,12]);
                if(original){
                    const scope={context:ctx,horizontalPixelRatio:2,verticalPixelRatio:2};
                    walkLine(scope,items,c.kind,{from:0,to:5},6,(_,item)=>item.color,(_,color)=>{ctx.strokeStyle=color;ctx.stroke();},24);
                    drawSeriesPointMarkers(scope,items,3.5,{from:0,to:5},(_,item)=>item.color);
                } else {
                    ctx.setLineDash([]);ctx.scale(c.ratio,c.ratio);
                    for(const s of c.adapter){ctx.beginPath();ctx.strokeStyle=s.color;ctx.lineWidth=s.width;
                        for(const [op,...args] of s.path) ctx[op==='M'?'moveTo':op==='L'?'lineTo':'bezierCurveTo'](...args);ctx.stroke();}
                    ctx.resetTransform();
                    for(const m of c.markers){ctx.beginPath();ctx.fillStyle=m.color;ctx.arc(...m.center,m.radius,0,Math.PI*2);ctx.fill();}
                }
            }
        },{walker,markerSource,items,selected});
        await page.screenshot({path:'/tmp/feather-line-reference.png'});
        console.log('Visual comparison saved to /tmp/feather-line-reference.png');
    } finally {await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});

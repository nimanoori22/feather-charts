// Execute actual upstream gesture arbitration and pinch methods, without a DOM timer.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(process.argv[2]||'../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
const code=ts.transpileModule(fs.readFileSync(path.join(root,'src/gui/mouse-event-handler.ts'),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ES2020}}).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm,'').replace(/\bexport /g,'');
const Handler=new Function('ensureNotNull',`${code}\nreturn MouseEventHandler;`)(value=>{assert.notEqual(value,null);return value;});
const output=spawnSync(path.resolve('target/debug/examples/touch_reference'),{encoding:'utf8'});
if(output.status!==0)throw new Error(output.stderr||'Build touch_reference first');
const cases=JSON.parse(output.stdout);
for(const [index,c] of cases.entries()) {
    const handler=Object.create(Handler.prototype);
    if(c.kind==='direction') {
        let pan=false;
        Object.assign(handler,{_activeTouchId:1,_startPinchMiddlePoint:null,_preventTouchDragProcess:false,_touchMoveExceededManhattanDistance:false,_touchMoveStartPosition:{x:200,y:150},_longTapTimeoutId:null,
            _handler:{touchMoveEvent(){pan=true;}},_options:{treatVertTouchDragAsPageScroll:()=>!c.vertical,treatHorzTouchDragAsPageScroll:()=>!c.horizontal},
            _resetTapTimeout(){},_makeCompatEvent(){return {};}});
        handler._touchMoveHandler({timeStamp:10,changedTouches:[{identifier:1,pageX:200+c.dx,pageY:150+c.dy}],cancelable:false});
        assert.equal(pan,c.pan,`direction case ${index}`);
    } else {
        Object.assign(handler,{_target:{getBoundingClientRect:()=>({left:0,top:0})},_handler:{},_longTapTimeoutId:null});
        const touches=[c.first,c.second].map(([clientX,clientY])=>({clientX,clientY}));
        handler._startPinch(touches);
        assert.deepEqual([handler._startPinchMiddlePoint.x,handler._startPinchMiddlePoint.y],c.anchor,`anchor case ${index}`);
        touches[1].clientX+=c.delta[0]; touches[1].clientY+=c.delta[1];
        const distance=Math.hypot(touches[1].clientX-touches[0].clientX,touches[1].clientY-touches[0].clientY);
        assert.ok(Math.abs((distance/handler._startPinchDistance-1)*5-c.zoom)<1e-8,`zoom case ${index}`);
    }
}
console.log(`${cases.length} native touch direction/pinch cases match original TypeScript methods.`);

// Differential oracle: actual original widget methods, without a DOM or clock.
// cargo build --example price_axis_reference
// node scripts/check_price_axis_reference.cjs [path/to/lightweight-charts]
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
const suggest = new Function(`${source('gui/internal-layout-sizes-hints.ts')}\nreturn suggestPriceScaleWidth;`)();
let recorded;
const Widget = new Function('makeFont', 'ensureNotNull', 'suggestPriceScaleWidth', 'clearRect', 'clearRectWithGradient',
    `${source('gui/price-axis-widget.ts')}\nreturn PriceAxisWidget;`)(
    (size, family) => `${size}px ${family}`, value => value, suggest,
    (_ctx, _x, _y, _w, _h, color) => recorded.push(['bg', color]),
    (_ctx, _x, _y, _w, _h, top, bottom) => recorded.push(['gradient', top, bottom]),
);
const result = spawnSync(path.resolve('target/debug/examples/price_axis_reference'), { encoding: 'utf8' });
if (result.status !== 0) throw new Error(result.stderr || 'Build the oracle example first');
const cases = JSON.parse(result.stdout);
function compare(a, b) {
    if (typeof a === 'number') assert.ok(Math.abs(a - b) < 1e-8, `${a} != ${b}`);
    else if (Array.isArray(a)) { assert.equal(a.length, b.length); a.forEach((v, i) => compare(v, b[i])); }
    else assert.equal(a, b);
}
for (const c of cases) {
    recorded = [];
    let tickRects = [];
    const ctx = {
        save() {}, restore() {}, beginPath() { tickRects = []; },
        fillRect(x, y, w, h) { recorded.push(['rect', x/c.ratio, y/c.ratio, w/c.ratio, h/c.ratio, this.fillStyle]); },
        rect(x, y, w, h) { tickRects.push(['rect', x/c.ratio, y/c.ratio, w/c.ratio, h/c.ratio, this.fillStyle]); },
        fill() { recorded.push(...tickRects); },
        fillText(label, x, y) { recorded.push(['text', label, x, y, this.fillStyle]); },
    };
    const marks = c.empty ? [] : [[10.5, '−123456.78'], [35.25, '0.00'], [60.75, '1234567.89']].map(([coord, label]) => ({coord, label}));
    const options = { borderVisible: c.border, ticksVisible: c.ticks, borderColor: '#123', textColor: 'red' };
    const widget = Object.create(Widget.prototype);
    Object.assign(widget, {
        _isLeft: c.left, _size: {width:100, height:80},
        _layoutOptions: {fontSize:12, fontFamily:'sans-serif', textColor:'red'},
        _priceScale: {marks: () => marks, options: () => options, firstValue: () => null},
        _rendererOptionsProvider: {options: () => ({font:'12px sans-serif', borderSize:1, tickLength:5, paddingInner:5, paddingOuter:5})},
        _widthCache: {reset() {}, measureText: (_ctx, label) => [...label].length * 7, yMidCorrection: () => 3},
        _canvasBinding: {canvasElement: {getContext: () => ctx}},
        _backLabels: () => [],
        _pane: {state: () => ({model: () => ({backgroundTopColor: () => '#fff', backgroundBottomColor: () => c.gradient ? '#ddd' : '#fff'})}),
            chart: () => ({options: () => ({layout: {colorSpace: 'srgb'}})})},
    });
    compare(widget.optimalWidth(), c.width);
    const scope = {context:ctx, bitmapSize:{width:100*c.ratio, height:80*c.ratio}, horizontalPixelRatio:c.ratio, verticalPixelRatio:c.ratio};
    widget._drawBackground(scope); widget._drawBorder(scope);
    widget._drawTickMarks({useBitmapCoordinateSpace: cb => cb(scope), useMediaCoordinateSpace: cb => cb({context:ctx})});
    compare(recorded, c.commands);
}
console.log(`${cases.length} price-axis cases match original widget sizing, backgrounds, borders, ticks, colors and label order/coordinates.`);

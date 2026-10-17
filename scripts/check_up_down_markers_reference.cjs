// Actual pinned factory, primitive, timers, view and renderer; no rewritten TS algorithms.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const { createHash } = require('node:crypto'), { spawnSync } = require('node:child_process');
const root = path.resolve(process.argv.slice(2).find(v => !v.startsWith('--')) || '../lightweight-charts');
for (const [file, hash] of Object.entries(JSON.parse(fs.readFileSync('docs/porting/up-down-markers-source.json')).files))
    assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root, file))).digest('hex'), hash, file);
const ts = require(require.resolve('typescript', { paths: [root] }));
function source(file) {
    return ts.transpileModule(fs.readFileSync(path.join(root, 'src', file), 'utf8'), {
        compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020 },
    }).outputText.replace(/^import[\s\S]*?from ['"][^'"]+['"];?\s*$/gm, '').replace(/\bexport /g, '');
}
function fixture(type = 'Line', options = {}) {
    let now = 1700000000000, next = 0, primitive;
    const timers = new Map();
    const window = { setTimeout(callback, delay) { const id = ++next; timers.set(id, { callback, at: now + delay }); return id; }, clearTimeout(id) { timers.delete(id); } };
    const factory = new Function('window', 'Date', 'isFulfilledData', 'isWhitespaceData', 'ensureDefined', 'ensureNotNull', 'notNull', [
        'plugins/up-down-markers-plugin/options.ts', 'plugins/up-down-markers-plugin/expiring-markers-manager.ts',
        'plugins/up-down-markers-plugin/renderer.ts', 'plugins/up-down-markers-plugin/view.ts',
        'plugins/up-down-markers-plugin/primitive.ts', 'plugins/series-primitive-adapter.ts', 'plugins/up-down-markers-plugin/wrapper.ts',
    ].map(source).join('\n') + ';return createUpDownMarkers;')(
        window, { now: () => now }, d => d.value !== undefined, d => d.value === undefined,
        v => { assert.notEqual(v, undefined); return v; }, v => { assert.notEqual(v, null); return v; }, v => v !== null);
    const series = { reject: false, seriesType: () => type, options: () => ({ color: 'blue', lineColor: 'orange' }),
        priceToCoordinate: v => v, setData() {}, update() { if (this.reject) throw Error('rejected'); },
        attachPrimitive(p) { primitive = p; p.attached({ series, chart: { timeScale: () => ({ timeToCoordinate: t => t }) }, horzScaleBehavior: { key: t => t }, requestUpdate() {} }); },
        detachPrimitive(p) { p.detached(); },
    };
    const api = factory(series, options);
    return { api, series, primitive, advance(ms) { now = 1700000000000 + ms; for (const [id, timer] of [...timers]) if (timer.at <= now && timers.delete(id)) timer.callback(); },
        draw(rx = 1, ry = rx) {
            primitive.updateAllViews(); const calls = [], colors = [];
            const context = { beginPath() {}, arc(...args) { calls.push(args.slice(0, 3)); }, fill() { colors.push(this.fillStyle); },
                moveTo(x, y) { calls.push([x, y]); }, lineTo(x, y) { calls.push([x, y]); }, stroke() { calls.push([this.lineWidth]); } };
            primitive.paneViews()[0].renderer().draw({ useBitmapCoordinateSpace: f => f({ context, horizontalPixelRatio: rx, verticalPixelRatio: ry }) });
            return { calls, colors };
        } };
}
function markers(p) { return p.api.markers().map(m => [m.time, m.value, m.sign]); }
const p = fixture(), states = {};
const snap = name => states[name] = markers(p);
p.api.setData([{ time: 60, value: 10 }, { time: 120, value: 20 }]); snap('seed');
p.api.update({ time: 60, value: 12 }, true); snap('up');
p.advance(100); p.api.update({ time: 60, value: 11 }, true); snap('repeat');
p.api.update({ time: 120, value: 20 }, true); snap('neutral');
p.advance(200); p.api.update({ time: 60, value: 9 }, true); snap('refresh_order');
p.advance(5099); snap('before_deadline'); p.advance(5100); snap('at_first_deadline'); p.advance(5200); snap('expired');
p.api.setMarkers([{ time: 60, value: 1, sign: 1 }, { time: 120, value: 2, sign: -1 }, { time: 60, value: 3, sign: 0 }]); snap('manual');
p.advance(100000); snap('manual_persists'); p.api.setData([{ time: 60, value: 5 }]); snap('reset_preserves');
p.api.update({ time: 180, value: 8 }); p.api.update({ time: 180, value: 9 }); snap('append_untracked');
p.api.update({ time: 60 }, true); p.api.update({ time: 60, value: 7 }, true); snap('whitespace_untracks');
p.api.clearMarkers(); snap('clear');
assert.deepEqual(states.repeat, [[60, 11, 1]]); assert.deepEqual(states.manual, [[120, 2, -1], [60, 3, 0]]);
assert.deepEqual(states.expired, []); assert.deepEqual(states.reset_preserves, states.manual);
const zero = fixture('Line', { updateVisibilityDuration: 0 }); zero.api.setData([{ time: 60, value: 1 }]); zero.api.update({ time: 60, value: 2 }); assert.deepEqual(markers(zero), []);
const truthy = fixture(); truthy.api.setData([{ time: 60, value: 0 }]); truthy.api.update({ time: 60, value: 2 }); assert.deepEqual(markers(truthy), []);
const failed = fixture(); failed.api.setData([{ time: 60, value: 1 }]); failed.series.reject = true; assert.throws(() => failed.api.update({ time: 60, value: 2 })); assert.deepEqual(markers(failed), [[60, 2, 1]]);
const stale = fixture(); stale.api.setMarkers([{ time: 60, value: 1, sign: 1 }]); const color = stale.draw().colors; stale.api.applyOptions({ positiveColor: 'purple' }); assert.deepEqual(stale.draw().colors, color);
const area = fixture('Area'); area.api.setMarkers([{ time: 60, value: 1, sign: 0 }]); assert.deepEqual(area.draw().colors, ['orange']);
assert.throws(() => fixture('Candlestick')); p.api.detach(); assert.throws(() => p.api.update({ time: 60, value: 2 }));
if (!process.argv.includes('--source-only')) {
    const run = spawnSync('cargo', ['test', '--offline', '--lib', 'up_down_markers', '--', '--nocapture', '--test-threads=1'], { encoding: 'utf8' }); assert.equal(run.status, 0, run.stderr + run.stdout);
    let geometry = 0, timelines = 0;
    for (const line of run.stdout.split('\n')) {
        if (line.includes('UP_DOWN_GEOMETRY ')) {
            const c = JSON.parse(line.split('UP_DOWN_GEOMETRY ')[1]); const probe = fixture();
            probe.api.setMarkers([{ time: c.x, value: c.y, sign: c.sign }]); const actual = probe.draw(c.rx, c.ry).calls.flat();
            assert.equal(actual.length, c.actual.length); actual.forEach((v, i) => assert.ok(Math.abs(v - c.actual[i]) < 1e-9, JSON.stringify(c))); geometry++;
        }
        if (line.includes('UP_DOWN_STATE ')) { const c = JSON.parse(line.split('UP_DOWN_STATE ')[1]); assert.deepEqual(c.markers, states[c.name], c.name); timelines++; }
    }
    assert.equal(geometry, 54); assert.equal(timelines, Object.keys(states).length);
    console.log(`${geometry} renderer cases and ${timelines} timeline states match actual TS.`);
}
console.log('Source factory/Line/Area, baseline-relative updates, append/whitespace, insertion order, refresh, exact expiry, manual/reset/clear/detach checks pass.');
console.log('Explicit native corrections: zero baseline presence, live color options, transactional rejected writes. Zero-duration expiry follows executed source, not its comment.');

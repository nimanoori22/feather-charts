// Capture the actual neighboring source in Chromium, not a regenerated chart.
// Requires a compiled standalone bundle, Puppeteer from the source checkout,
// and metadata emitted by line_chart --capture-dir. No network is used.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(process.argv[4] || '../lightweight-charts');
const puppeteer = require(require.resolve('puppeteer', { paths: [root] }));
const input = path.resolve(process.argv[2] || '/tmp/feather-display-captures');
const bundle = path.resolve(process.argv[3] || '/tmp/feather-lwc-reference.js');
(async () => {
    const browser = await puppeteer.launch({ executablePath: process.env.CHROMIUM || '/usr/bin/chromium',
        headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
    try {
        for (const file of fs.readdirSync(input).filter(f => f.endsWith('.json') && !f.endsWith('-reference.json'))) {
            const m = JSON.parse(fs.readFileSync(path.join(input, file), 'utf8'));
            const page = await browser.newPage();
            await page.setViewport({ width: m.logicalWidth, height: m.logicalHeight, deviceScaleFactor: m.scaleFactor });
            await page.setContent('<style>html,body{margin:0;padding:0;overflow:hidden}</style><div id="chart"></div>');
            await page.addScriptTag({ content: 'window.process = {env:{NODE_ENV:"development",BUILD_VERSION:"5.2.1-local-source"}};' });
            await page.addScriptTag({ path: bundle });
            const state = await page.evaluate(m => {
                const l = window.LightweightCharts;
                const background = m.scenario === 'gradient'
                    ? { type: 'gradient', topColor: '#101820', bottomColor: '#36536a' }
                    : { type: 'solid', color: '#101820' };
                const both = ['gradient', 'large-negative'].includes(m.scenario);
                const ticks = m.scenario !== 'default';
                const chart = l.createChart(document.querySelector('#chart'), {
                    width: m.logicalWidth, height: m.logicalHeight,
                    layout: { background, textColor: '#eee', fontFamily: m.font, fontSize: m.fontSize, attributionLogo: false },
                    leftPriceScale: { visible: both, ticksVisible: ticks }, rightPriceScale: { ticksVisible: ticks },
                    timeScale: { timeVisible: true, ticksVisible: ticks },
                    grid: { vertLines: { color: '#D6DCDE' }, horzLines: { color: '#D6DCDE' } },
                });
                window.referenceChart = chart;
                const series = chart.addSeries(l.LineSeries, { color: '#2196f3', lineWidth: 3,
                    lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false,
                    priceFormat: { type: 'price', precision: 2, minMove: 0.01 } });
                const value = i => (100 + Math.sin(i * .2) * 8 + i * .04) * (m.scenario === 'large-negative' ? -10000 : 1);
                const data = Array.from({ length: 120 }, (_,i) => ({ time: 1700000000 + i * 60, value: value(i),
                    color: Math.floor(i/20)%2 === 0 ? '#2196f3' : '#f59e0b' }));
                series.setData(data);
                if (both) {
                    const left = chart.addSeries(l.LineSeries, { priceScaleId: 'left', lineVisible: false,
                        lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
                    left.setData(data.map(d => ({ ...d, value: d.value * 100 - 12000 })));
                }
                chart.timeScale().fitContent();
                // Geometry comparison uses the same sampled animation offset,
                // not an independently advancing browser clock.
                if (m.scenario === 'historical') {
                    chart.timeScale().setVisibleLogicalRange({ from: 10, to: 50 });
                    series.update({ time: 1700000000 + 120 * 60, value: value(120) });
                }
                if (m.scenario === 'animation') chart.timeScale().applyOptions({ rightOffset: -20 });
                if (m.scenario === 'empty') chart.removeSeries(series);
                return { devicePixelRatio: window.devicePixelRatio, userAgent: navigator.userAgent,
                    logicalWidth: m.logicalWidth, logicalHeight: m.logicalHeight, scenario: m.scenario };
            }, m);
            await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
            Object.assign(state, await page.evaluate(() => ({
                backend: 'chromium-canvas2d', font: 'sans-serif', fontSize: 12,
                plotWidth: window.referenceChart.timeScale().width(),
                timeHeight: window.referenceChart.timeScale().height(),
                leftWidth: window.referenceChart.priceScale('left').width(),
                rightWidth: window.referenceChart.priceScale('right').width(),
            })));
            const target = file.replace('.json', '-reference');
            await page.screenshot({ path: path.join(input, target + '.png') });
            fs.writeFileSync(path.join(input, target + '.json'), JSON.stringify(state, null, 2));
            await page.close();
        }
    } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });

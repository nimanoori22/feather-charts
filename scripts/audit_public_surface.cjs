// Inventory declared public contracts from the pinned local source, not a parity score.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto');
const root=path.resolve('../lightweight-charts');
const ts=require(require.resolve('typescript',{paths:[root]}));
const api={
 IChartApiBase:{
  remove:'native|Drop releases chart ownership',resize:'native|Chart.prepare / ChartMessage::Bounds; host owns sizing',
  addCustomSeries:'mapped|Chart.add_custom_series',addSeries:'mapped|Chart.add_series',removeSeries:'mapped|Chart.remove_series',
  subscribeClick:'native|ChartEvent::Click; owned delivery',unsubscribeClick:'native|Host owns routing of pointer events',
  subscribeDblClick:'native|EventInterests.double_click / ChartEvent::DoubleClick',unsubscribeDblClick:'native|EventInterests.double_click',
  subscribeCrosshairMove:'native|ChartEvent::CrosshairMoved',unsubscribeCrosshairMove:'native|Host owns routing of pointer events',
  priceScale:'native|Chart methods use pane + PriceScalePosition',timeScale:'native|Chart checked range/navigation/conversion helpers; read-only engine inspection retained',
  applyOptions:'partial|F04/F05: Chart.apply_chart_options covers axis input and presentation; default visible-scale selection and color policy incomplete',options:'partial|F04/F05: runtime/localization/axis-input readbacks; default visible-scale selection and color policy incomplete',
  takeScreenshot:'mapped|Chart.take_screenshot / prepare_screenshot + ScreenshotExporter',addPane:'native|Chart.add_pane then set_preserve_empty_pane',
  panes:'mapped|Chart.model().panes()',removePane:'mapped|Chart.remove_pane',swapPanes:'mapped|Chart.swap_panes',
  autoSizeActive:'platform|Iced host controls layout; no ResizeObserver',chartElement:'platform|Iced Canvas replaces DOM element',
  setCrosshairPosition:'mapped|Chart.set_crosshair_at; source pane-default scale and silent synthetic policy',
  clearCrosshairPosition:'mapped|Chart.clear_crosshair_position',paneSize:'mapped|Chart.frame/multi_frame layouts',
  horzBehaviour:'mapped|Chart.model().time_scale().behavior()'
 },
 IPaneApi:{
  getHeight:'mapped|Chart.model().panes()[pane].height()',setHeight:'mapped|Chart.set_pane_height; persistent source stretch allocation, collapsed layouts remain inert',
  moveTo:'mapped|Chart.move_pane',paneIndex:'native|Current index selectors; stable PrimitivePaneId for attachments',
  getSeries:'mapped|Pane.attached_source_ids, sorted with Chart.series_order / Chart.model().pane_for_series',getHTMLElement:'platform|No DOM; use prepared pane layout',
  attachPrimitive:'mapped|Chart.add_pane_primitive',detachPrimitive:'mapped|Chart.remove_primitive',priceScale:'native|Pane selector + Chart price-scale methods',
  setPreserveEmptyPane:'mapped|Chart.set_preserve_empty_pane',preserveEmptyPane:'mapped|Pane.preserve_empty_pane()',
  getStretchFactor:'mapped|Pane.stretch_factor()',setStretchFactor:'mapped|Chart.set_pane_stretch_factor',
  addCustomSeries:'native|Chart.add_custom_series with pane selector',addSeries:'native|Chart.add_series with pane selector'
 },
 IPriceLine:{applyOptions:'native|Chart.replace_price_line_options; merge an owned readback',options:'mapped|Chart.price_line_options'},
 IPriceScaleApi:{applyOptions:'mapped|Chart.apply_price_scale_options',options:'mapped|Chart.price_scale_options',
  width:'native|Prepared frame layout; overlays occupy no axis width',setVisibleRange:'mapped|Chart.set_visible_price_range',
  getVisibleRange:'mapped|Chart.visible_price_range',setAutoScale:'mapped|Chart.set_price_scale_auto_scale'},
 ISeriesApi:{
  priceFormatter:'native|Chart.with_series_formatter / typed custom handle.with_formatter; scoped reads',priceToCoordinate:'mapped|Chart.price_to_coordinate; actual owning scale and visible first value',
  coordinateToPrice:'mapped|Chart.coordinate_to_price; actual owning scale and visible first value',barsInLogicalRange:'mapped|Chart.bars_in_logical_range / custom handle',
  applyOptions:'native|Typed option replacement routes checked price-scale reassignment; full replacement semantics',options:'native|Chart.with_series_options / typed custom with_options and with_view_options; scoped non-Clone callbacks',
  priceScale:'native|Chart.series_price_scale returns a fresh checked typed selector for built-in/custom series',setData:'mapped|Chart.set_data / set_custom_data',
  update:'mapped|Chart.update_data / update_custom_data',pop:'mapped|Chart.pop_data / pop_custom_data',
  dataByIndex:'mapped|Chart.data_by_index / custom handle',data:'mapped|Chart<B, M>.series_data retains typed metadata; owned canonical rows',
  subscribeDataChanged:'native|EventInterests.data_changed / ChartEvent::DataChanged',unsubscribeDataChanged:'native|EventInterests.data_changed; host filters series',
  createPriceLine:'mapped|Chart.add_price_line',removePriceLine:'mapped|Chart.remove_price_line',priceLines:'native|Chart.price_lines returns owned creation-order IDs; options queried by ID',
  seriesType:'mapped|Chart.series_type; checked built-in/custom ownership',
  lastValueData:'mapped|Chart.last_value_data; owned optional value for built-in/custom series',attachPrimitive:'mapped|Chart.add_series_primitive',detachPrimitive:'mapped|Chart.remove_primitive',
  moveToPane:'mapped|Chart.move_series_to_pane',seriesOrder:'mapped|Chart.series_order',setSeriesOrder:'mapped|Chart.set_series_order',getPane:'native|Chart.series_pane returns fresh checked pane index'
 },
 ITimeScaleApi:{
  scrollPosition:'mapped|Chart.scroll_position',scrollToPosition:'native|Chart.set_right_offset / scroll_to_offset_animated',
  scrollToRealTime:'mapped|Chart.scroll_to_real_time; configured offset and source 400ms animation',getVisibleRange:'mapped|Chart.visible_horizontal_range; owned original items without event draining',
  setVisibleRange:'mapped|Chart.set_visible_horizontal_range; behavior conversion and nearest endpoints',getVisibleLogicalRange:'mapped|Chart.visible_logical_range; owned lazy cache read',
  setVisibleLogicalRange:'mapped|Chart.set_logical_range',resetTimeScale:'mapped|Chart.reset_time_scale; configured defaults',fitContent:'mapped|Chart.fit_content',
  logicalToCoordinate:'mapped|Chart.logical_to_coordinate; source integer-index policy',coordinateToLogical:'mapped|Chart.coordinate_to_logical; source upward rounding',
  timeToIndex:'mapped|Chart.horizontal_to_index',timeToCoordinate:'mapped|Chart.horizontal_to_coordinate',
  coordinateToTime:'mapped|Chart.coordinate_to_horizontal; original items',width:'mapped|TimeScale.width()',height:'native|Prepared frame time-axis layout',
  subscribeVisibleTimeRangeChange:'native|EventInterests.visible_horizontal_range',unsubscribeVisibleTimeRangeChange:'native|EventInterests.visible_horizontal_range',
  subscribeVisibleLogicalRangeChange:'native|EventInterests.visible_logical_range',unsubscribeVisibleLogicalRangeChange:'native|EventInterests.visible_logical_range',
  subscribeSizeChange:'native|EventInterests.horizontal_axis_size',unsubscribeSizeChange:'native|EventInterests.horizontal_axis_size',
  applyOptions:'mapped|Chart.apply_time_scale_options',options:'mapped|TimeScale.options(); compare effective bar_spacing() separately'
 },
 IYieldCurveChartApi:{addSeries:'mapped|YieldCurveChart built-in add methods; shared Chart for other typed families'}
};
// Fields are checked against these real Rust structs. Presence is not behavioral proof.
const types={
 LayoutOptions:'src/model/layout_options.rs',LayoutPanesOptions:'src/model/layout_options.rs',
 HorzScaleOptions:'src/model/time_scale_options.rs',PriceScaleOptions:'src/model/price_scale.rs',
 CrosshairOptions:'src/model/crosshair.rs',CrosshairLineOptions:'src/model/crosshair.rs',GridOptions:'src/model/grid.rs',GridLineOptions:'src/model/grid.rs',
 LocalizationOptions:'src/model/localization_options.rs',LocalizationOptionsBase:'src/model/localization_options.rs',
 SeriesOptionsCommon:'src/model/series_options.rs',CandlestickStyleOptions:'src/model/series_options.rs',BarStyleOptions:'src/model/series_options.rs',LineStyleOptions:'src/model/series_options.rs',
 AreaStyleOptions:'src/model/series_options.rs',BaselineStyleOptions:'src/model/series_options.rs',HistogramStyleOptions:'src/model/series_options.rs',CustomStyleOptions:'src/model/series_options.rs',
 PriceFormatBuiltIn:'src/model/series_options.rs',PriceFormatCustom:'src/model/series_options.rs',PriceLineOptions:'src/model/price_line.rs',
 WhitespaceData:'src/model/data_consumer.rs',SingleValueData:'src/model/data_consumer.rs',OhlcData:'src/model/data_consumer.rs',LineData:'src/model/data_consumer.rs',HistogramData:'src/model/data_consumer.rs',
 AreaData:'src/model/data_consumer.rs',BaselineData:'src/model/data_consumer.rs',BarData:'src/model/data_consumer.rs',CandlestickData:'src/model/data_consumer.rs',
 ImageWatermarkOptions:'src/api/image_watermark.rs',TextWatermarkOptions:'src/api/text_watermark.rs',TextWatermarkLineOptions:'src/api/text_watermark.rs#TextWatermarkLine',
 SeriesMarkersOptions:'src/model/series_markers.rs',UpDownMarkersPluginOptions:'src/api/up_down_markers.rs#UpDownMarkersOptions',
 YieldCurveOptions:'src/model/yield_curve_horz_scale_behavior.rs',PriceChartLocalizationOptions:'src/model/horz_scale_behavior_price.rs#OptionsChartOptions',
 KineticScrollOptions:'src/api/touch.rs',TimeScaleOptions:'src/model/horz_scale_behavior_time/time_based_chart_options.rs'
};
const special={
 'CrosshairOptions.vertLine':'native|CrosshairOptions.vertical',
 'CrosshairOptions.horzLine':'native|CrosshairOptions.horizontal',
 'TextWatermarkOptions.horzAlign':'native|TextWatermarkOptions.horizontal_alignment',
 'TextWatermarkOptions.vertAlign':'native|TextWatermarkOptions.vertical_alignment',
 'TextWatermarkLineOptions.fontFamily':'native|TextWatermarkLine.font.family (Iced Font)',
 'TextWatermarkLineOptions.fontStyle':'native|TextWatermarkLine.font.style (Iced Font)',
 'LayoutOptions.attributionLogo':'native|Source default on; native default off intentionally retained; runtime toggle is native extension',
 'LayoutOptions.colorSpace':'partial|F05: stored/snapshotted; no wide-gamut native rendering contract',
 'LayoutOptions.colorParsers':'gap|F05: constructor parsers are not routed into chart rendering/validation',
 'SeriesOptionsCommon.priceScaleId':'native|Checked reassignment; creation selector wins, replacement Some relocates and None retains; stored/actual IDs agree',
 'PriceFormatCustom.tickmarksFormatter':'mapped|Series formatter/source bridge and actual final axis batches; formatting-parity.md',
 'LocalizationOptionsBase.locale':'partial|F09: constructor/runtime locale updates supported; Intl labels still differ',
 'LocalizationOptionsBase.priceFormatter':'mapped|Chart price-scale localization, including YieldCurve defaults; scalar/batch precedence tested',
 'LocalizationOptionsBase.tickmarksPriceFormatter':'mapped|Chart-owned scale batch formatting; precedence and actual axis text tested',
 'LocalizationOptionsBase.percentageFormatter':'mapped|Chart-owned percentage formatting for scale, crosshair and last-value text',
 'LocalizationOptionsBase.tickmarksPercentageFormatter':'mapped|Chart-owned scale batch formatting; precedence and actual axis text tested',
 'LocalizationOptions.timeFormatter':'native|Construction plus Chart.apply_localization_options / localization_options; explicit callback Clear',
 'LocalizationOptions.dateFormat':'native|Construction plus Chart.apply_localization_options / localization_options; omitted string retains current value',
 'TimeScaleOptions.tickMarkFormatter':'native|TimeBehaviorOptions via with_custom_behavior / configure_horizontal_behavior',
 'YieldCurveOptions.formatTime':'native|YieldCurveChart.set_maturity_formatter',
 'WhitespaceData.customValues':'native|Chart<B, M> typed optional metadata; Rc payloads retain identity through updates/queries/events',
 'PriceLineOptions.id':'mapped|PriceLineOptions.external_id',
 'PriceFormatBuiltIn.type':'native|PriceFormatBuiltIn.kind enum',
 'PriceFormatCustom.type':'native|PriceFormat::Custom enum variant'
};
const extra=new Set(['ChartOptionsBase','ChartOptionsImpl','HandleScrollOptions','HandleScaleOptions','AxisPressedMouseMoveOptions','AxisDoubleClickOptions','TrackingModeOptions','MouseEventParams','HoveredInfo']);
const chartFields={width:'native|Chart.prepare and Iced layout',height:'native|Chart.prepare and Iced layout',autoSize:'platform|Iced host layout replaces ResizeObserver',
 layout:'mapped|ChartModelOptions.layout',leftPriceScale:'mapped|PaneOptions.left_price_scale',rightPriceScale:'mapped|PaneOptions.right_price_scale',overlayPriceScales:'mapped|PaneOptions.overlay_price_scale',
 defaultVisiblePriceScaleId:'partial|F04: PaneOptions.default_visible_price_scale construction only',timeScale:'mapped|HorzScaleOptions / Chart.apply_time_scale_options',
 crosshair:'mapped|Chart.set_crosshair_options / apply_chart_options',grid:'mapped|ChartModelOptions.grid',handleScroll:'native|MouseNavigationOptions / TouchNavigationOptions',
 handleScale:'mapped|AxisInteractionOptions plus MouseNavigationOptions.wheel_zoom / TouchNavigationOptions.pinch',kineticScroll:'mapped|KineticScrollOptions',trackingMode:'mapped|TouchNavigationOptions.tracking_exit',
 localization:'native|LocalizationOptions + Chart.apply_localization_options / localization_options; locale limitations tracked as F09',addDefaultPane:'mapped|ChartModelOptions.add_default_pane',hoveredSeriesOnTop:'mapped|Chart.set_hovered_series_on_top'};
const files=[]; function walk(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())walk(p);else if(p.endsWith('.ts'))files.push(p)}}walk(path.join(root,'src'));
const rows=[], declarations=[], hashes={};
const snake=s=>s.replace(/([a-z0-9])([A-Z])/g,'$1_$2').toLowerCase();
for(const file of files.sort()){
 const source=fs.readFileSync(file,'utf8'),sf=ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true);
 for(const node of sf.statements){
  if(!ts.isInterfaceDeclaration(node))continue;
  const owner=node.name.text, rel=path.relative(root,file);
  if(!(owner in api||owner in types||extra.has(owner)))continue;
  hashes[rel]=createHash('sha256').update(source).digest('hex');
  declarations.push({name:owner,file:rel,line:sf.getLineAndCharacterOfPosition(node.getStart(sf)).line+1,extends:(node.heritageClauses||[]).map(n=>n.getText(sf))});
  for(const m of node.members){
   const member=m.name?.getText(sf)||'<call>',key=owner+'.'+member;
   let finding=special[key]||api[owner]?.[member];
   if(!finding&&owner in api)throw Error('Unclassified public API method '+key);
   if(!finding&&owner in types){
    const [rust,rename]=types[owner].split('#'),name=rename||owner;
    const code=fs.readFileSync(rust,'utf8');
    const body=code.match(new RegExp('pub struct '+name+'[^\\{]*\\{([\\s\\S]*?)^\\}','m'))?.[1]||'';
    const field=snake(member);
    finding=new RegExp('pub '+field+'\\s*:').test(body)?'represented|'+rust+' '+name+'.'+field:'review|Check renamed/native ownership field in '+rust;
   }
   if(!finding&&owner==='ChartOptionsBase')finding=chartFields[member];
   if(!finding&&owner==='ChartOptionsImpl')finding='native|Generic LocalizationOptions + LocalizationOptionsPatch; typed domain formatter and owned callback readback';
   if(!finding&&owner==='HandleScrollOptions')finding='native|MouseNavigationOptions / TouchNavigationOptions';
   if(!finding&&owner==='HandleScaleOptions')finding=['mouseWheel','pinch'].includes(member)?'native|MouseNavigationOptions.wheel_zoom / TouchNavigationOptions.pinch':'native|AxisInteractionOptions; independent drag/reset for both axes';
   if(!finding&&['AxisPressedMouseMoveOptions','AxisDoubleClickOptions'].includes(owner))finding='native|AxisInteractionOptions.'+(member==='time'?'time':'price')+(owner==='AxisPressedMouseMoveOptions'?'_drag':'_reset');
   if(!finding&&owner==='TrackingModeOptions')finding='native|TouchNavigationOptions.tracking_exit';
   if(!finding&&owner==='MouseEventParams')finding=member==='sourceEvent'?'native|PointerEvent.source_event owns optional native device/kind/button, chart/window logical positions and observed modifiers; DOM-only coordinates omitted':'native|PointerEvent typed data/hit/custom/primitive payload; legacy aliases not duplicated';
   if(!finding&&owner==='HoveredInfo')finding='native|ChartHit / PrimitiveHit / MarkerHit / CustomHitData and PointerEvent.pane';
   const [status,...note]=String(finding||'review|Unclassified contract').split('|');
   rows.push({id:key,file:rel,line:sf.getLineAndCharacterOfPosition(m.getStart(sf)).line+1,kind:ts.isMethodSignature(m)?'method':'property',signature:m.getText(sf).replace(/\s+/g,' '),status,note:note.join('|')});
  }
 }
}
for(const [owner,methods] of Object.entries(api))assert.equal(rows.filter(r=>r.id.startsWith(owner+'.')).length,Object.keys(methods).length,owner+' interface drift');
for(const name of Object.keys(types))assert(declarations.some(d=>d.name===name),'Missing option/data interface '+name);
const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
for(const spec of Object.values(types)){const p=spec.split('#')[0];hashes['native:'+p]=hash(p);}
const entry=ts.createSourceFile('index.ts',fs.readFileSync(path.join(root,'src/index.ts'),'utf8'),ts.ScriptTarget.Latest,true);
const reexports=entry.statements.flatMap(n=>ts.isExportDeclaration(n)&&n.exportClause&&ts.isNamedExports(n.exportClause)?n.exportClause.elements.map(e=>({name:e.name.text,from:n.moduleSpecifier.text})):[]);
const counts={declarations:declarations.length,members:rows.length,methods:rows.filter(r=>r.kind==='method').length,statuses:{}};
for(const r of rows)counts.statuses[r.status]=(counts.statuses[r.status]||0)+1;
const result={schema:1,scope:'Declared members of the listed core API/options/data/payload interfaces; inheritance recorded, not expanded. Representation is not proven behavioral parity. Extension traits and exports reviewed separately in public-parity-audit.md.',counts,reexports,declarations,hashes,rows};
const output='docs/porting/public-parity-inventory.json';
if(process.argv.includes('--check'))assert.deepEqual(JSON.parse(fs.readFileSync(output)),result,'Inventory or reviewed surface changed');
else {
 const {rows: members, ...metadata}=result;
 const header=JSON.stringify(metadata,null,2).slice(0,-2);
 fs.writeFileSync(output,header+',\n  "rows": [\n'+members.map(r=>'    '+JSON.stringify(r)).join(',\n')+'\n  ]\n}\n');
}
console.log(JSON.stringify(counts));
for(const r of rows.filter(r=>r.status==='review'))console.log(r.id+' '+r.note);

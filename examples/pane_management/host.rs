use super::*;
use feather_charts::model::{
    horz_scale_behavior_price::HorzScaleBehaviorPrice,
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        types::{Time, TimePoint, UtcTimestamp},
    },
    yield_curve_horz_scale_behavior::YieldCurveHorzScaleBehavior,
};
type BuiltInRow<C> = BuiltInSeriesDataItem<<C as Host>::Item, <C as Host>::Metadata>;
type CustomRow<C> = CustomSeriesDataItem<
    interval::Datum<<C as Host>::Item>,
    <C as Host>::Item,
    <C as Host>::Metadata,
>;
type HostEvent<C> = ChartEvent<<C as Host>::Item, <C as Host>::Metadata>;
pub trait Host: region::RegionHost + Sized + 'static {
    type Metadata: metadata::DemoMetadata;
    fn record_update(
        &mut self,
        id: SeriesId,
        row: BuiltInRow<Self>,
        historical: bool,
    ) -> Result<(), ChartError>;
    fn pane_height(&mut self, pane: usize, height: f64) -> Result<(), ChartError>;
    fn pane_stretch(&mut self, pane: usize, factor: f64) -> Result<(), ChartError>;
    fn helper_action(
        &mut self,
        action: helpers::Action,
        id: SeriesId,
    ) -> Result<String, ChartError>;
    fn synthetic(
        &mut self,
        id: SeriesId,
        item: Option<&Self::Item>,
        price: f64,
    ) -> Result<(), ChartError>;
    fn fresh_screenshot(
        &mut self,
        layers: ScreenshotLayers,
        now: Instant,
    ) -> Result<ChartScreenshot, ChartError>;
    fn interests(&mut self, interests: EventInterests);
    fn take_events(&mut self) -> Vec<HostEvent<Self>>;
    fn axis_visible(&mut self, visible: bool);
    fn offset(&mut self, offset: f64);
    fn assign_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<(), ChartError>;
    fn assign_options(
        &mut self,
        id: SeriesId,
        custom: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
        use_custom: bool,
        target: &str,
        pulse: bool,
    ) -> Result<(), ChartError>;
    fn localize(&mut self, patch: LocalizationOptionsPatch<Self::Item>);
    fn format_series(
        &mut self,
        id: SeriesId,
        custom: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
        enabled: bool,
        pulse: bool,
    ) -> Result<(), ChartError>;
    fn configure(&mut self, patch: ChartOptionsPatch) -> Result<(), ChartError>;
    fn set_range(
        &mut self,
        pane: usize,
        side: &PriceScalePosition,
        range: PriceRange,
    ) -> Result<(), ChartError>;
    fn set_auto(
        &mut self,
        pane: usize,
        side: &PriceScalePosition,
        enabled: bool,
    ) -> Result<(), ChartError>;
    fn scale_patch(
        &mut self,
        pane: usize,
        side: &PriceScalePosition,
        patch: feather_charts::model::price_scale::PriceScaleOptionsPatch,
    ) -> Result<(), ChartError>;
    type Item: Clone + PartialEq + std::fmt::Debug + 'static;
    type Internal: Clone + std::fmt::Debug + 'static;
    type Behavior: ChartBehavior<Item = Self::Item, InternalItem = Self::Internal>;
    fn new() -> Self;
    fn key(i: usize) -> Self::Item;
    fn side() -> PriceScalePosition;
    fn core(&self) -> &Chart<Self::Behavior, Self::Metadata>;
    fn line(&mut self, pane: usize, pulse: bool) -> SeriesId;
    fn area(&mut self, pane: usize) -> SeriesId;
    fn pop_data(&mut self, id: SeriesId, count: usize)
    -> Result<Vec<BuiltInRow<Self>>, ChartError>;
    fn pop_custom_data(
        &mut self,
        h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
        count: usize,
    ) -> Result<Vec<interval::Datum<Self::Item>>, ChartError>;
    fn raw_data(&mut self, id: SeriesId, rows: Vec<BuiltInRow<Self>>) -> Result<(), ChartError>;
    fn set_series_order(&mut self, id: SeriesId, order: usize) -> Result<(), ChartError>;
    fn set_hovered_series_on_top(&mut self, enabled: bool);
    fn custom(
        &mut self,
        pane: usize,
    ) -> CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>;
    fn update_custom(
        &mut self,
        h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
        row: CustomRow<Self>,
    ) -> Result<(), ChartError>;
    fn custom_data(
        &mut self,
        h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
        rows: Vec<CustomRow<Self>>,
    ) -> Result<(), ChartError>;
    fn add_pane(&mut self) -> usize;
    fn move_pane(&mut self, from: usize, to: usize) -> Result<(), ChartError>;
    fn swap_panes(&mut self, first: usize, second: usize) -> Result<(), ChartError>;
    fn move_series_to_pane(&mut self, id: SeriesId, to: usize) -> Result<(), ChartError>;
    fn remove_pane(&mut self, pane: usize) -> Result<(), ChartError>;
    fn markers(&mut self, id: SeriesId) -> UpDownMarkersId;
    fn data(&mut self, id: UpDownMarkersId, rows: Vec<BuiltInRow<Self>>) -> Result<(), ChartError>;
    fn update_data(
        &mut self,
        id: UpDownMarkersId,
        row: BuiltInRow<Self>,
        at: Instant,
    ) -> Result<(), ChartError>;
    fn prepare(&mut self, size: Size, ratio: f32, at: Instant) -> Result<(), ChartError>;
    fn update(&mut self, message: ChartMessage) -> Result<Vec<HostEvent<Self>>, ChartError>;
    fn fit(&mut self);
    fn decorate(&mut self, pane: usize);
}
macro_rules! common {
    () => {
        fn record_update(&mut self, id: SeriesId, row: BuiltInRow<Self>, historical: bool) -> Result<(), ChartError> { self.update_data(id, row, historical) }
        fn pane_height(&mut self, pane: usize, height: f64) -> Result<(), ChartError> {
            self.set_pane_height(pane, height)
        }
        fn pane_stretch(&mut self, pane: usize, factor: f64) -> Result<(), ChartError> {
            self.set_pane_stretch_factor(pane, factor)
        }
        fn synthetic(
            &mut self,
            id: SeriesId,
            item: Option<&Self::Item>,
            price: f64,
        ) -> Result<(), ChartError> {
            if let Some(item) = item {
                self.set_crosshair_at(id, item, price)?;
            } else {
                self.clear_crosshair_position()?;
            }
            Ok(())
        }
        fn helper_action(
            &mut self,
            action: helpers::Action,
            id: SeriesId,
        ) -> Result<String, ChartError> {
            use helpers::Action;
            match action {
                Action::Range => self.set_visible_horizontal_range(HorizontalRange {
                    from: Self::key(4),
                    to: Self::key(18),
                })?,
                Action::Reset => self.reset_time_scale(),
                Action::Live => self.scroll_to_real_time(),
                Action::Invalid => {
                    let before = self.visible_logical_range();
                    assert!(
                        self.set_visible_horizontal_range(HorizontalRange {
                            from: Self::key(18),
                            to: Self::key(4)
                        })
                        .is_err()
                    );
                    assert_eq!(self.visible_logical_range(), before);
                    return Ok("Invalid reversed range rejected; current range preserved".into());
                }
                Action::Crosshair => {
                    let price = self
                        .last_value_data(id, true)?
                        .map_or(4.5, |value| value.price);
                    let placed = self.set_crosshair_at(id, &Self::key(12), price)?;
                    return Ok(format!(
                        "Synthetic crosshair placed={placed}; no pointer event emitted"
                    ));
                }
                Action::Clear => self.clear_crosshair_position()?,
                Action::Lines => {
                    if let Some(line) = self.price_lines(id)?.first() {
                        self.remove_price_line(*line)?;
                    } else {
                        self.add_price_line(id, PriceLineOptions::new(4.5))?;
                    }
                }
                Action::Inspect | Action::Sync => {}
            }
            let range = self.visible_horizontal_range();
            let logical = self.visible_logical_range();
            let last = self.last_value_data(id, true)?;
            let roundtrip = if let Some(last) = &last {
                if let Some(y) = self.price_to_coordinate(id, last.price)? {
                    self.coordinate_to_price(id, y)?
                } else {
                    None
                }
            } else {
                None
            };
            let formatted =
                self.with_series_formatter(id, |f| f.format(last.as_ref().map_or(4.5, |v| v.price)))?;
            Ok(format!(
                "{:?} pane={} scale={:?} lines={} item={range:?} logical={logical:?} last={:?} roundtrip={roundtrip:?} formatted={formatted}",
                self.series_type(id)?,
                self.series_pane(id)?,
                self.series_price_scale(id)?,
                self.price_lines(id)?.len(),
                last.map(|v| v.price)
            ))
        }

        fn fresh_screenshot(
            &mut self,
            layers: ScreenshotLayers,
            now: Instant,
        ) -> Result<ChartScreenshot, ChartError> {
            self.prepare_screenshot(layers, now)
        }
        fn interests(&mut self, interests: EventInterests) {
            self.set_event_interests(interests);
        }
        fn take_events(&mut self) -> Vec<HostEvent<Self>> {
            self.take_events()
        }
        fn axis_visible(&mut self, visible: bool) {
            self.apply_time_scale_options(
                feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                    visible: Some(visible),
                    ..Default::default()
                },
            );
        }
        fn offset(&mut self, offset: f64) {
            self.set_right_offset(offset);
        }
        fn assign_scale(
            &mut self,
            id: SeriesId,
            position: PriceScalePosition,
        ) -> Result<(), ChartError> {
            self.set_series_price_scale(id, position)
        }
        fn assign_options(
            &mut self,
            id: SeriesId,
            custom: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
            use_custom: bool,
            target: &str,
            pulse: bool,
        ) -> Result<(), ChartError> {
            if use_custom {
                let mut opts = CustomSeriesOptions::default();
                opts.common.price_scale_id = Some(target.into());
                self.replace_custom_series_options(custom, opts)
            } else {
                let mut opts = options(pulse);
                opts.common.price_scale_id = Some(target.into());
                self.replace_series_options(id, SeriesOptionsMap::Line(opts))
            }
        }
        fn localize(&mut self, patch: LocalizationOptionsPatch<Self::Item>) {
            self.apply_localization_options(patch);
        }
        fn format_series(
            &mut self,
            id: SeriesId,
            custom: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
            enabled: bool,
            pulse: bool,
        ) -> Result<(), ChartError> {
            let mut line = options(pulse);
            let mut opts = feather_charts::model::series_options::CustomSeriesOptions::default();
            if enabled {
                line.common.price_format = formatting::series_format();
                opts.common.price_format = formatting::series_format();
            }
            self.replace_series_options(id, SeriesOptionsMap::Line(line))?;
            self.replace_custom_series_options(custom, opts)
        }
        fn configure(&mut self, patch: ChartOptionsPatch) -> Result<(), ChartError> {
            self.apply_chart_options(patch)
        }
        fn set_range(
            &mut self,
            pane: usize,
            side: &PriceScalePosition,
            range: PriceRange,
        ) -> Result<(), ChartError> {
            self.set_visible_price_range(pane, side, range)
        }
        fn set_auto(
            &mut self,
            pane: usize,
            side: &PriceScalePosition,
            enabled: bool,
        ) -> Result<(), ChartError> {
            self.set_price_scale_auto_scale(pane, side, enabled)
        }
        fn scale_patch(
            &mut self,
            pane: usize,
            side: &PriceScalePosition,
            patch: feather_charts::model::price_scale::PriceScaleOptionsPatch,
        ) -> Result<(), ChartError> {
            self.apply_price_scale_options(pane, side, patch)
        }
        fn pop_data(
            &mut self,
            id: SeriesId,
            count: usize,
        ) -> Result<Vec<BuiltInRow<Self>>, ChartError> {
            self.pop_data(id, count)
        }
        fn pop_custom_data(
            &mut self,
            h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
            count: usize,
        ) -> Result<Vec<interval::Datum<Self::Item>>, ChartError> {
            self.pop_custom_data(h, count)
        }
        fn raw_data(
            &mut self,
            id: SeriesId,
            rows: Vec<BuiltInRow<Self>>,
        ) -> Result<(), ChartError> {
            self.set_data(id, rows)
        }
        fn set_series_order(&mut self, id: SeriesId, order: usize) -> Result<(), ChartError> {
            self.set_series_order(id, order)
        }
        fn set_hovered_series_on_top(&mut self, enabled: bool) {
            self.set_hovered_series_on_top(enabled)
        }
        fn custom(
            &mut self,
            pane: usize,
        ) -> CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>> {
            self.add_custom_series(
                interval::View::default(),
                Default::default(),
                pane,
                Self::side(),
            )
            .unwrap()
        }
        fn update_custom(
            &mut self,
            h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
            row: CustomRow<Self>,
        ) -> Result<(), ChartError> {
            self.update_custom_data(h, row, false)
        }
        fn custom_data(
            &mut self,
            h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal, Self::Metadata>>,
            rows: Vec<CustomRow<Self>>,
        ) -> Result<(), ChartError> {
            self.set_custom_data(h, rows)
        }
        fn add_pane(&mut self) -> usize {
            self.add_pane()
        }
        fn move_pane(&mut self, a: usize, b: usize) -> Result<(), ChartError> {
            self.move_pane(a, b)
        }
        fn swap_panes(&mut self, a: usize, b: usize) -> Result<(), ChartError> {
            self.swap_panes(a, b)
        }
        fn move_series_to_pane(&mut self, id: SeriesId, to: usize) -> Result<(), ChartError> {
            self.move_series_to_pane(id, to)
        }
        fn remove_pane(&mut self, p: usize) -> Result<(), ChartError> {
            self.remove_pane(p)
        }
        fn markers(&mut self, id: SeriesId) -> UpDownMarkersId {
            self.add_up_down_markers(id, Default::default()).unwrap()
        }
        fn data(
            &mut self,
            id: UpDownMarkersId,
            rows: Vec<BuiltInRow<Self>>,
        ) -> Result<(), ChartError> {
            self.set_up_down_data(id, rows)
        }
        fn update_data(
            &mut self,
            id: UpDownMarkersId,
            row: BuiltInRow<Self>,
            at: Instant,
        ) -> Result<(), ChartError> {
            self.update_up_down_data(id, row, false, at)
        }
        fn prepare(&mut self, s: Size, r: f32, at: Instant) -> Result<(), ChartError> {
            self.prepare(s, r, at)
        }
        fn update(&mut self, m: ChartMessage) -> Result<Vec<HostEvent<Self>>, ChartError> {
            self.update(m)
        }
        fn fit(&mut self) {
            self.fit_content()
        }
        fn decorate(&mut self, p: usize) {
            self.add_pane_primitive(
                p,
                TextWatermark::new(TextWatermarkOptions {
                    lines: vec![TextWatermarkLine {
                        text: format!("Pane {}", p + 1),
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .unwrap(),
            )
            .unwrap();
            self.add_pane_primitive(
                p,
                ImageWatermark::from_rgba(
                    1,
                    1,
                    vec![30, 150, 190, 14],
                    ImageWatermarkOptions {
                        max_width: Some(160.),
                        max_height: Some(80.),
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
            .unwrap();
        }
    };
}
fn options(pulse: bool) -> LineSeriesOptions {
    let mut o = LineSeriesOptions::default();
    o.style.color = "#2196f3".into();
    o.style.last_price_animation = if pulse {
        LastPriceAnimationMode::Continuous
    } else {
        LastPriceAnimationMode::Disabled
    };
    o
}
impl<M: metadata::DemoMetadata> Host for Chart<HorzScaleBehaviorTime, M> {
    type Metadata = M;
    type Item = Time;
    type Internal = TimePoint;
    type Behavior = HorzScaleBehaviorTime;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn new() -> Self {
        Self::new_with_metadata(Default::default())
    }
    fn key(i: usize) -> Time {
        UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into()
    }
    fn core(&self) -> &Self {
        self
    }
    fn line(&mut self, p: usize, pulse: bool) -> SeriesId {
        self.add_series(
            SeriesOptionsMap::Line(options(pulse)),
            p,
            PriceScalePosition::Right,
        )
        .unwrap()
    }
    fn area(&mut self, p: usize) -> SeriesId {
        self.add_series(SeriesOptionsMap::Area(area_options()), p, Self::side())
            .unwrap()
    }
    common!();
}
impl<M: metadata::DemoMetadata> Host for Chart<session_behavior::Sessions, M> {
    type Metadata = M;
    type Item = session_behavior::SessionPoint;
    type Internal = session_behavior::SessionKey;
    type Behavior = session_behavior::Sessions;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn new() -> Self {
        Self::with_custom_behavior_and_metadata(
            || session_behavior::Sessions::new(Default::default()),
            Default::default(),
            Default::default(),
            feather_charts::model::localization_options::LocalizationOptions::new("en-US", ""),
        )
        .unwrap()
    }
    fn key(i: usize) -> Self::Item {
        session_behavior::SessionPoint::at(i * 2 + i / 5 * 3)
    }
    fn core(&self) -> &Self {
        self
    }
    fn line(&mut self, p: usize, pulse: bool) -> SeriesId {
        self.add_series(
            SeriesOptionsMap::Line(options(pulse)),
            p,
            PriceScalePosition::Right,
        )
        .unwrap()
    }
    fn area(&mut self, p: usize) -> SeriesId {
        self.add_series(SeriesOptionsMap::Area(area_options()), p, Self::side())
            .unwrap()
    }
    common!();
}
impl<M: metadata::DemoMetadata> Host for Chart<HorzScaleBehaviorPrice, M> {
    type Metadata = M;
    type Item = HorizontalPrice;
    type Internal = HorizontalPrice;
    type Behavior = HorzScaleBehaviorPrice;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn new() -> Self {
        Self::new_options_with_metadata(Default::default()).unwrap()
    }
    fn key(i: usize) -> HorizontalPrice {
        HorizontalPrice::new(90. + i as f64 * 2.5).unwrap()
    }
    fn core(&self) -> &Self {
        self
    }
    fn line(&mut self, p: usize, pulse: bool) -> SeriesId {
        self.add_series(
            SeriesOptionsMap::Line(options(pulse)),
            p,
            PriceScalePosition::Right,
        )
        .unwrap()
    }
    fn area(&mut self, p: usize) -> SeriesId {
        self.add_series(SeriesOptionsMap::Area(area_options()), p, Self::side())
            .unwrap()
    }
    common!();
}
impl<M: metadata::DemoMetadata> Host for YieldCurveChart<M> {
    type Metadata = M;
    type Item = Maturity;
    type Internal = Maturity;
    type Behavior = YieldCurveHorzScaleBehavior;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Left
    }
    fn new() -> Self {
        Self::new_with_metadata(Default::default()).unwrap()
    }
    fn key(i: usize) -> Maturity {
        Maturity::from_months(i as f64 * 6. + 1.).unwrap()
    }
    fn core(&self) -> &Chart<Self::Behavior, Self::Metadata> {
        self
    }
    fn line(&mut self, p: usize, pulse: bool) -> SeriesId {
        self.add_line_series(
            LineSeriesOptionsPatch {
                style: LineStyleOptionsPatch {
                    last_price_animation: Some(options(pulse).style.last_price_animation),
                    ..Default::default()
                },
                ..Default::default()
            },
            p,
            Self::side(),
        )
        .unwrap()
    }
    fn area(&mut self, p: usize) -> SeriesId {
        self.add_area_series(Default::default(), p, Self::side())
            .unwrap()
    }
    common!();
}

fn area_options() -> AreaSeriesOptions {
    let mut o = AreaSeriesOptions::default();
    o.common.price_format = PriceFormat::Custom(PriceFormatCustom {
        formatter: Rc::new(|p| format!("{:.4}", p.value())),
        tickmarks_formatter: None,
        min_move: 0.0001,
        base: Some(10000.),
    });
    o
}

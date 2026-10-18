use super::*;
use feather_charts::model::{
    horz_scale_behavior_price::HorzScaleBehaviorPrice,
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        types::{Time, TimePoint, UtcTimestamp},
    },
    yield_curve_horz_scale_behavior::YieldCurveHorzScaleBehavior,
};
pub trait Host: region::RegionHost + Sized + 'static {
    fn fresh_screenshot(
        &mut self,
        layers: ScreenshotLayers,
        now: Instant,
    ) -> Result<ChartScreenshot, ChartError>;
    fn interests(&mut self, interests: EventInterests);
    fn take_events(&mut self) -> Vec<ChartEvent<Self::Item>>;
    fn axis_visible(&mut self, visible: bool);
    fn offset(&mut self, offset: f64);
    fn localize(&mut self, patch: LocalizationOptionsPatch<Self::Item>);
    fn format_series(
        &mut self,
        id: SeriesId,
        custom: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
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
    fn core(&self) -> &Chart<Self::Behavior>;
    fn line(&mut self, pane: usize, pulse: bool) -> SeriesId;
    fn area(&mut self, pane: usize) -> SeriesId;
    fn pop_data(
        &mut self,
        id: SeriesId,
        count: usize,
    ) -> Result<Vec<BuiltInSeriesDataItem<Self::Item>>, ChartError>;
    fn pop_custom_data(
        &mut self,
        h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
        count: usize,
    ) -> Result<Vec<interval::Datum<Self::Item>>, ChartError>;
    fn raw_data(
        &mut self,
        id: SeriesId,
        rows: Vec<BuiltInSeriesDataItem<Self::Item>>,
    ) -> Result<(), ChartError>;
    fn set_series_order(&mut self, id: SeriesId, order: usize) -> Result<(), ChartError>;
    fn set_hovered_series_on_top(&mut self, enabled: bool);
    fn custom(
        &mut self,
        pane: usize,
    ) -> CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>;
    fn update_custom(
        &mut self,
        h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
        row: CustomSeriesDataItem<interval::Datum<Self::Item>, Self::Item>,
    ) -> Result<(), ChartError>;
    fn custom_data(
        &mut self,
        h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
        rows: Vec<CustomSeriesDataItem<interval::Datum<Self::Item>, Self::Item>>,
    ) -> Result<(), ChartError>;
    fn add_pane(&mut self) -> usize;
    fn move_pane(&mut self, from: usize, to: usize) -> Result<(), ChartError>;
    fn swap_panes(&mut self, first: usize, second: usize) -> Result<(), ChartError>;
    fn move_series_to_pane(&mut self, id: SeriesId, to: usize) -> Result<(), ChartError>;
    fn remove_pane(&mut self, pane: usize) -> Result<(), ChartError>;
    fn markers(&mut self, id: SeriesId) -> UpDownMarkersId;
    fn data(
        &mut self,
        id: UpDownMarkersId,
        rows: Vec<BuiltInSeriesDataItem<Self::Item>>,
    ) -> Result<(), ChartError>;
    fn update_data(
        &mut self,
        id: UpDownMarkersId,
        row: BuiltInSeriesDataItem<Self::Item>,
        at: Instant,
    ) -> Result<(), ChartError>;
    fn prepare(&mut self, size: Size, ratio: f32, at: Instant) -> Result<(), ChartError>;
    fn update(&mut self, message: ChartMessage) -> Result<Vec<ChartEvent<Self::Item>>, ChartError>;
    fn fit(&mut self);
    fn decorate(&mut self, pane: usize);
}
macro_rules! common {
    () => {
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
        fn take_events(&mut self) -> Vec<ChartEvent<Self::Item>> {
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
        fn localize(&mut self, patch: LocalizationOptionsPatch<Self::Item>) {
            self.apply_localization_options(patch);
        }
        fn format_series(
            &mut self,
            id: SeriesId,
            custom: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
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
        ) -> Result<Vec<BuiltInSeriesDataItem<Self::Item>>, ChartError> {
            self.pop_data(id, count)
        }
        fn pop_custom_data(
            &mut self,
            h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
            count: usize,
        ) -> Result<Vec<interval::Datum<Self::Item>>, ChartError> {
            self.pop_custom_data(h, count)
        }
        fn raw_data(
            &mut self,
            id: SeriesId,
            rows: Vec<BuiltInSeriesDataItem<Self::Item>>,
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
        ) -> CustomSeriesHandle<interval::View<Self::Item, Self::Internal>> {
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
            h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
            row: CustomSeriesDataItem<interval::Datum<Self::Item>, Self::Item>,
        ) -> Result<(), ChartError> {
            self.update_custom_data(h, row, false)
        }
        fn custom_data(
            &mut self,
            h: &CustomSeriesHandle<interval::View<Self::Item, Self::Internal>>,
            rows: Vec<CustomSeriesDataItem<interval::Datum<Self::Item>, Self::Item>>,
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
            rows: Vec<BuiltInSeriesDataItem<Self::Item>>,
        ) -> Result<(), ChartError> {
            self.set_up_down_data(id, rows)
        }
        fn update_data(
            &mut self,
            id: UpDownMarkersId,
            row: BuiltInSeriesDataItem<Self::Item>,
            at: Instant,
        ) -> Result<(), ChartError> {
            self.update_up_down_data(id, row, false, at)
        }
        fn prepare(&mut self, s: Size, r: f32, at: Instant) -> Result<(), ChartError> {
            self.prepare(s, r, at)
        }
        fn update(&mut self, m: ChartMessage) -> Result<Vec<ChartEvent<Self::Item>>, ChartError> {
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
impl Host for Chart {
    type Item = Time;
    type Internal = TimePoint;
    type Behavior = HorzScaleBehaviorTime;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn new() -> Self {
        Default::default()
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
impl Host for Chart<session_behavior::Sessions> {
    type Item = session_behavior::SessionPoint;
    type Internal = session_behavior::SessionKey;
    type Behavior = session_behavior::Sessions;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn new() -> Self {
        Chart::with_custom_behavior(
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
impl Host for OptionsChart {
    type Item = HorizontalPrice;
    type Internal = HorizontalPrice;
    type Behavior = HorzScaleBehaviorPrice;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn new() -> Self {
        Self::new_options(Default::default()).unwrap()
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
impl Host for YieldCurveChart {
    type Item = Maturity;
    type Internal = Maturity;
    type Behavior = YieldCurveHorzScaleBehavior;
    fn side() -> PriceScalePosition {
        PriceScalePosition::Left
    }
    fn new() -> Self {
        YieldCurveChart::new(Default::default()).unwrap()
    }
    fn key(i: usize) -> Maturity {
        Maturity::from_months(i as f64 * 6. + 1.).unwrap()
    }
    fn core(&self) -> &Chart<Self::Behavior> {
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

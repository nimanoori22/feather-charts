//! Demo-only forwarding keeps both real public chart ownership paths exercised.
use super::*;
use feather_charts::model::{
    chart_model::ChartModelOptions, horz_scale_behavior_price::HorzScaleBehaviorPrice,
    series_options::*, yield_curve_horz_scale_behavior::YieldCurveHorzScaleBehavior,
};
use std::fmt::Debug;
pub trait Host: Sized + 'static {
    type Item: Clone + PartialEq + Debug + 'static;
    type Behavior: ChartBehavior<Item = Self::Item, InternalItem = Self::Item>;
    fn new() -> Self;
    fn key(i: usize, dense: bool) -> Self::Item;
    fn label(t: &Self::Item) -> String;
    fn core(&self) -> &Chart<Self::Behavior>;
    fn side() -> PriceScalePosition;
    fn peer(&mut self, pulse: bool) -> SeriesId;
    fn add(&mut self) -> Result<CustomSeriesHandle<View<Self::Item>>, ChartError>;
    fn data(
        &mut self,
        h: &CustomSeriesHandle<View<Self::Item>>,
        rows: Vec<CustomSeriesDataItem<Datum<Self::Item>, Self::Item>>,
    ) -> Result<(), ChartError>;
    fn change(
        &mut self,
        h: &CustomSeriesHandle<View<Self::Item>>,
        row: CustomSeriesDataItem<Datum<Self::Item>, Self::Item>,
        historical: bool,
    ) -> Result<(), ChartError>;
    fn peer_data(
        &mut self,
        id: SeriesId,
        rows: Vec<BuiltInSeriesDataItem<Self::Item>>,
    ) -> Result<(), ChartError>;
    fn style(
        &mut self,
        h: &CustomSeriesHandle<View<Self::Item>>,
        options: Options,
    ) -> Result<(), ChartError>;
    fn remove(&mut self, id: SeriesId) -> Result<(), ChartError>;
    fn fit(&mut self, last: Self::Item);
    fn scale(&mut self, patch: HorzScaleOptionsPatch);
    fn prepare(&mut self, size: Size, ratio: f32, now: Instant) -> Result<(), ChartError>;
    fn update(&mut self, m: ChartMessage) -> Result<Vec<ChartEvent<Self::Item>>, ChartError>;
    fn decorate(&mut self);
    fn labels(&mut self, custom: bool) -> Result<(), ChartError>;
}
macro_rules! common {
    () => {
        fn add(&mut self) -> Result<CustomSeriesHandle<View<Self::Item>>, ChartError> {
            self.add_custom_series(
                View::default(),
                CustomSeriesOptions::default(),
                0,
                Self::side(),
            )
        }
        fn data(
            &mut self,
            h: &CustomSeriesHandle<View<Self::Item>>,
            rows: Vec<CustomSeriesDataItem<Datum<Self::Item>, Self::Item>>,
        ) -> Result<(), ChartError> {
            self.set_custom_data(h, rows)
        }
        fn change(
            &mut self,
            h: &CustomSeriesHandle<View<Self::Item>>,
            r: CustomSeriesDataItem<Datum<Self::Item>, Self::Item>,
            history: bool,
        ) -> Result<(), ChartError> {
            self.update_custom_data(h, r, history)
        }
        fn peer_data(
            &mut self,
            id: SeriesId,
            rows: Vec<BuiltInSeriesDataItem<Self::Item>>,
        ) -> Result<(), ChartError> {
            self.set_data(id, rows)
        }
        fn style(
            &mut self,
            h: &CustomSeriesHandle<View<Self::Item>>,
            o: Options,
        ) -> Result<(), ChartError> {
            self.replace_custom_view_options(h, o)
        }
        fn remove(&mut self, id: SeriesId) -> Result<(), ChartError> {
            self.remove_series(id)
        }
        fn fit(&mut self, last: Self::Item) {
            self.fit_content();
            if let Some(index) = self.core().model().time_scale().time_to_index(&last, false) {
                self.set_logical_range(feather_charts::model::time_data::LogicalRange {
                    from: 0.0.into(),
                    to: index.value().into(),
                });
            }
        }
        fn scale(&mut self, p: HorzScaleOptionsPatch) {
            self.apply_time_scale_options(p);
        }
        fn prepare(&mut self, s: Size, r: f32, n: Instant) -> Result<(), ChartError> {
            self.prepare(s, r, n)
        }
        fn update(&mut self, m: ChartMessage) -> Result<Vec<ChartEvent<Self::Item>>, ChartError> {
            self.update(m)
        }
        fn decorate(&mut self) {
            self.add_pane_primitive(
                0,
                TextWatermark::new(TextWatermarkOptions {
                    lines: vec![TextWatermarkLine {
                        text: "Custom intervals".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .unwrap(),
            )
            .unwrap();
            self.add_pane_primitive(
                0,
                ImageWatermark::from_rgba(
                    1,
                    1,
                    vec![50, 170, 180, 12],
                    ImageWatermarkOptions {
                        max_width: Some(180.),
                        max_height: Some(100.),
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
            .unwrap();
        }
    };
}
impl Host for OptionsChart {
    type Item = HorizontalPrice;
    type Behavior = HorzScaleBehaviorPrice;
    fn new() -> Self {
        Self::new_options(OptionsChartOptions { precision: 2 }).unwrap()
    }
    fn key(i: usize, dense: bool) -> HorizontalPrice {
        HorizontalPrice::new(if dense {
            90. + i as f64 * 0.25
        } else {
            90. + i as f64 * 2.5 + if i > 12 { 10. } else { 0. }
        })
        .unwrap()
    }
    fn label(t: &HorizontalPrice) -> String {
        format!("{:.2}", t.value())
    }
    fn core(&self) -> &Self {
        self
    }
    fn side() -> PriceScalePosition {
        PriceScalePosition::Right
    }
    fn peer(&mut self, pulse: bool) -> SeriesId {
        let mut o = LineSeriesOptions::default();
        o.style.color = "#2196f3".into();
        o.style.last_price_animation = if pulse {
            LastPriceAnimationMode::Continuous
        } else {
            LastPriceAnimationMode::Disabled
        };
        self.add_series(SeriesOptionsMap::Line(o), 0, Self::side())
            .unwrap()
    }
    fn labels(&mut self, on: bool) -> Result<(), ChartError> {
        self.replace_options_chart_options(OptionsChartOptions {
            precision: if on { 4 } else { 2 },
        })
    }
    common!();
}
impl Host for YieldCurveChart {
    type Item = Maturity;
    type Behavior = YieldCurveHorzScaleBehavior;
    fn new() -> Self {
        Self::with_chart_options(
            Default::default(),
            {
                let mut o = ChartModelOptions::default();
                o.pane.left_price_scale.visible = true;
                o.pane.right_price_scale.visible = false;
                o
            },
            feather_charts::model::localization_options::LocalizationOptions::new("en-US", ""),
        )
        .unwrap()
    }
    fn key(i: usize, dense: bool) -> Maturity {
        Maturity::from_months(if dense {
            i as f64 + 1.
        } else {
            i as f64 * 6. + 1.
        })
        .unwrap()
    }
    fn label(t: &Maturity) -> String {
        t.label()
    }
    fn core(&self) -> &Chart<Self::Behavior> {
        self
    }
    fn side() -> PriceScalePosition {
        PriceScalePosition::Left
    }
    fn peer(&mut self, pulse: bool) -> SeriesId {
        self.add_line_series(
            LineSeriesOptionsPatch {
                style: LineStyleOptionsPatch {
                    last_price_animation: Some(if pulse {
                        LastPriceAnimationMode::Continuous
                    } else {
                        LastPriceAnimationMode::Disabled
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
            0,
            Self::side(),
        )
        .unwrap()
    }
    fn labels(&mut self, on: bool) -> Result<(), ChartError> {
        self.set_maturity_formatter(
            on.then(|| std::rc::Rc::new(|v: &Maturity| format!("{} mo", v.months())) as _),
        );
        Ok(())
    }
    common!();
}

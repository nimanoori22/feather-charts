//! Offline diagnostic observations for the public-parity audit; never opens a window.
use feather_charts::{
    api::Chart,
    model::{
        bar::BarPrice,
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::{
            default_tick_mark_formatter::default_tick_mark_formatter,
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{TickMarkType, Time, TimePoint, UtcTimestamp},
        },
        localization_options::LocalizationOptions,
        pane::PriceScalePosition,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
        series_options::{LineSeriesOptions, PriceFormat, PriceFormatCustom, SeriesOptionsMap},
    },
};
use std::{cell::Cell, rc::Rc, time::Instant};

fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(time).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
fn main() {
    axis_observations();
    let calls = Rc::new(Cell::new(0));
    let mut localization = LocalizationOptions::new("en-US", "dd MMM 'yy");
    localization.base.price_formatter = Some(Rc::new(|_| "GLOBAL".into()));
    localization.base.percentage_formatter = Some(Rc::new(|_| "PERCENT".into()));
    localization.base.tickmarks_price_formatter = Some(Rc::new(|values| {
        values.iter().map(|_| "GLOBAL_TICKS".into()).collect()
    }));
    let mut chart = Chart::with_custom_behavior(
        HorzScaleBehaviorTime::default,
        Default::default(),
        Default::default(),
        localization,
    )
    .unwrap();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, vec![row(1., 10.), row(2., 20.)])
        .unwrap();
    chart.fit_content();
    chart
        .prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let scale = chart.model().panes()[0].right_price_scale();
    println!("AUDIT global_price {:?}", scale.format_price_absolute(20.));
    println!(
        "AUDIT global_ticks {:?}",
        scale.format_logical_tickmarks(&[20.])
    );
    chart
        .apply_price_scale_options(
            0,
            &PriceScalePosition::Right,
            PriceScaleOptionsPatch {
                mode: Some(PriceScaleMode::Percentage),
                ..Default::default()
            },
        )
        .unwrap();
    let scale = chart.model().panes()[0].right_price_scale();
    println!("AUDIT global_percentage {:?}", scale.format_price(20., 10.));
    let batch_calls = calls.clone();
    let mut options = LineSeriesOptions::default();
    options.common.price_scale_id = Some("left".into());
    options.common.price_format = PriceFormat::Custom(PriceFormatCustom {
        formatter: Rc::new(|_| "SINGLE".into()),
        tickmarks_formatter: Some(Rc::new(move |values: &[BarPrice]| {
            batch_calls.set(batch_calls.get() + 1);
            values.iter().map(|_| "BATCH".into()).collect()
        })),
        min_move: 0.01,
        base: None,
    });
    chart
        .replace_series_options(id, SeriesOptionsMap::Line(options))
        .unwrap();
    let series = chart.model().series(id).unwrap();
    println!(
        "AUDIT custom_ticks {:?}",
        series.formatter().format_tickmarks(&[20.])
    );
    println!("AUDIT custom_batch_calls {}", calls.get());
    drop(series);
    chart
        .prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let pane = &chart.model().panes()[0];
    assert!(pane.left_price_scale().is_empty() != pane.right_price_scale().is_empty());
    println!(
        "AUDIT scale_after_option {:?}",
        if pane.left_price_scale().is_empty() {
            "right"
        } else {
            "left"
        }
    );
    let point = TimePoint {
        timestamp: UtcTimestamp::new(1_704_196_800.),
        business_day: None,
    };
    for (name, kind, locale) in [
        ("arabic_day", TickMarkType::DayOfMonth, "ar-EG"),
        ("arabic_time", TickMarkType::Time, "ar-EG"),
        ("german_month", TickMarkType::Month, "de-DE"),
    ] {
        println!(
            "AUDIT {name} {:?}",
            default_tick_mark_formatter(&point, kind, locale).unwrap()
        );
    }
    let mut options = feather_charts::model::chart_model::ChartModelOptions::default();
    options.layout.color_parsers.push(Box::new(|color| {
        (color == "color(display-p3 1 0 0)")
            .then_some(feather_charts::model::colors::Rgba::new(255, 0, 0, 1.))
    }));
    options.layout.text_color = "color(display-p3 1 0 0)".into();
    let result = Chart::with_custom_behavior(
        HorzScaleBehaviorTime::default,
        options,
        Default::default(),
        LocalizationOptions::new("en-US", "dd MMM 'yy"),
    );
    println!("AUDIT configured_color_parser {}", result.is_ok());
}

fn axis_observations() {
    use iced::{
        Point, Rectangle, Size, mouse,
        widget::canvas::{Event, Program},
    };
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            id,
            (1..=20).map(|t| row(f64::from(t), f64::from(t))).collect(),
        )
        .unwrap();
    chart.fit_content();
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let mut state = feather_charts::api::ChartState::default();
    let axis = chart.frame().layout.time_axis;
    let point = Point::new(
        (axis.origin.x + axis.size.width * 0.6) as f32,
        (axis.origin.y + axis.size.height * 0.5) as f32,
    );
    let send = |chart: &mut Chart,
                state: &mut feather_charts::api::ChartState,
                event: Event,
                point: Point| {
        let action = chart.program(std::convert::identity).update(
            state,
            &event,
            Rectangle::with_size(Size::new(800., 500.)),
            mouse::Cursor::Available(point),
        );
        if let Some(message) = action.and_then(|a| a.into_inner().0) {
            chart.update(message).unwrap();
        }
    };
    let before = chart.model().time_scale().bar_spacing();
    send(
        &mut chart,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: point }),
        point,
    );
    send(
        &mut chart,
        &mut state,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        point,
    );
    let moved = Point::new(point.x - 100., point.y);
    send(
        &mut chart,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: moved }),
        moved,
    );
    send(
        &mut chart,
        &mut state,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        moved,
    );
    println!(
        "AUDIT time_axis_drag {}",
        chart.model().time_scale().bar_spacing() != before
    );
    for _ in 0..2 {
        send(
            &mut chart,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            point,
        );
        send(
            &mut chart,
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            point,
        );
    }
    println!(
        "AUDIT time_axis_double_reset {}",
        chart.model().time_scale().bar_spacing() == 6.
    );
}

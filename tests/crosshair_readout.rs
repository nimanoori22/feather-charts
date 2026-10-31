use feather_charts::{
    api::*,
    model::{data_consumer::*, pane::PriceScalePosition, series_options::*},
};
use iced::Size;
use std::time::Instant;
fn key(x: f64) -> HorizontalPrice {
    HorizontalPrice::new(x).unwrap()
}
fn prepare(c: &mut OptionsChart) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
#[test]
fn ohlc_readout_tracks_crosshair_formats_values_and_restores_latest() {
    let mut c = OptionsChart::new_numeric(0).unwrap();
    let mut o = CandlestickSeriesOptions::default();
    o.common.title = "Candles".into();
    let id = c
        .add_series(
            SeriesOptionsMap::Candlestick(o),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        [(1., 12., 14., 11., 13.), (2., 20., 24., 18., 22.)]
            .map(|(x, open, high, low, close)| {
                BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                    time: key(x),
                    open,
                    high,
                    low,
                    close,
                    color: None,
                    border_color: None,
                    wick_color: None,
                    custom_values: None,
                }))
            })
            .to_vec(),
    )
    .unwrap();
    prepare(&mut c);
    c.fit_content();
    prepare(&mut c);
    assert_eq!(
        c.crosshair_readout(),
        ["Candles  O: 20.00  H: 24.00  L: 18.00  C: 22.00"]
    );
    c.set_crosshair_at(id, &key(1.), 13.).unwrap();
    assert_eq!(
        c.crosshair_readout(),
        ["Candles  O: 12.00  H: 14.00  L: 11.00  C: 13.00"]
    );
    c.clear_crosshair_position().unwrap();
    assert!(c.crosshair_readout()[0].contains("C: 22.00"));
    c.remove_series(id).unwrap();
    assert!(c.crosshair_readout().is_empty());
}
#[test]
fn shared_index_gaps_never_show_stale_or_latest_values() {
    let mut c = OptionsChart::new_numeric(0).unwrap();
    let mut o = LineSeriesOptions::default();
    o.common.title = "Sparse".into();
    let line = c
        .add_series(SeriesOptionsMap::Line(o), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(
        line,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: key(1.),
            value: 1.25,
            color: None,
            custom_values: None,
        }))],
    )
    .unwrap();
    let mut o = HistogramSeriesOptions::default();
    o.common.title = "Columns".into();
    let hist = c
        .add_series(SeriesOptionsMap::Histogram(o), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_numeric_histogram_width(hist, 0.5).unwrap();
    c.set_data(
        hist,
        vec![BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(
            HistogramData {
                time: key(2.),
                value: 4.5,
                color: None,
                custom_values: None,
            },
        ))],
    )
    .unwrap();
    prepare(&mut c);
    c.fit_content();
    prepare(&mut c);
    c.set_crosshair_at(hist, &key(2.), 4.5).unwrap();
    assert_eq!(c.crosshair_readout(), ["Sparse  —", "Columns  4.50"]);
    c.set_data(hist, vec![]).unwrap();
    prepare(&mut c);
    c.clear_crosshair_position().unwrap();
    assert_eq!(c.crosshair_readout(), ["Sparse  1.25"]);
}

#[test]
fn overlay_paints_inside_chart_and_pointer_input_reaches_the_plot() {
    use iced::{
        Color, Event, Point, Rectangle, Theme,
        advanced::{
            Layout, Shell, clipboard, layout,
            renderer::{Headless, Renderer as _},
            widget::Tree,
        },
    };
    let mut c = OptionsChart::new_numeric(0).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        vec![BuiltInSeriesDataItem::Candlestick(
            CandlestickDataItem::Data(CandlestickData {
                time: key(1.),
                open: 12.,
                high: 14.,
                low: 11.,
                close: 13.,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            }),
        )],
    )
    .unwrap();
    prepare(&mut c);
    c.fit_content();
    prepare(&mut c);
    let size = Size::new(800., 500.);
    let viewport = Rectangle::with_size(size);
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut capture = |readout| {
        renderer.reset(viewport);
        let mut element = if readout {
            c.element_with_readout()
        } else {
            c.element()
        };
        let tree = Tree::new(element.as_widget());
        let mut tree = tree;
        let node =
            element
                .as_widget_mut()
                .layout(&mut tree, &renderer, &layout::Limits::new(size, size));
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &Theme::Light,
            &Default::default(),
            Layout::new(&node),
            iced::mouse::Cursor::Unavailable,
            &viewport,
        );
        renderer.screenshot(Size::new(800, 500), 1., Color::WHITE)
    };
    let plain = capture(false);
    let overlay = capture(true);
    assert_ne!(
        &plain[..800 * 60 * 4],
        &overlay[..800 * 60 * 4],
        "readout is invisible"
    );
    assert_eq!(
        &plain[800 * 100 * 4..],
        &overlay[800 * 100 * 4..],
        "readout changed chart size or unrelated pixels"
    );
    let resized = Size::new(900., 600.);
    let viewport = Rectangle::with_size(resized);
    let mut messages = Vec::new();
    {
        let mut element = c.element_with_readout();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(resized, resized),
        );
        for event in [
            Event::Window(iced::window::Event::RedrawRequested(Instant::now())),
            Event::Mouse(iced::mouse::Event::CursorMoved {
                position: Point::new(20., 20.),
            }),
        ] {
            element.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                iced::mouse::Cursor::Available(Point::new(20., 20.)),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &viewport,
            );
        }
    }
    assert!(
        messages
            .iter()
            .any(|m| matches!(m,ChartMessage::Bounds(s) if *s==resized)),
        "chart lost its full-window bounds"
    );
    for message in messages {
        c.update(message).unwrap();
    }
    assert!(
        c.model().crosshair().position().is_some(),
        "readout intercepted the pointer"
    );
}

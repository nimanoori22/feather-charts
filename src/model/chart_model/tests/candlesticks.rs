use super::*;
use crate::model::{
    data_consumer::{CandlestickData, CandlestickDataItem},
    price_scale::PriceScaleMode,
    series::candlesticks_pane_view::{CandlestickPreparationError, CandlesticksPaneView},
    series_options::CandlestickStyleOptions,
};
fn candle_options() -> SeriesOptionsMap {
    SeriesOptionsMap::Candlestick(SeriesOptions {
        common: common(),
        style: CandlestickStyleOptions {
            up_color: "green".into(),
            down_color: "red".into(),
            wick_visible: true,
            border_visible: true,
            border_color: "unused".into(),
            border_up_color: "blue".into(),
            border_down_color: "orange".into(),
            wick_color: "unused".into(),
            wick_up_color: "white".into(),
            wick_down_color: "black".into(),
        },
    })
}
fn candle(at: f64, open: f64, close: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
        time: Time::from(UtcTimestamp::new(at)),
        open,
        high: open.max(close) + 10.,
        low: open.min(close) - 10.,
        close,
        color: None,
        border_color: None,
        wick_color: None,
        custom_values: None,
    }))
}
fn candle_fixture(mode: PriceScaleMode) -> (Layer, Model, SeriesId) {
    let mut pane = PaneOptions::default();
    pane.right_price_scale.mode = mode;
    let mut model = model_with(pane, true);
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    let id = SeriesId::new(1);
    model.set_width(600.).unwrap();
    model.set_pane_height(0, 300.).unwrap();
    model
        .register_series(
            id,
            SeriesType::Candlestick,
            candle_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Candlestick,
                    (0..20)
                        .map(|i| candle(i as f64, 100. + i as f64, 102. + i as f64))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 4., 8.);
    (layer, model, id)
}
#[test]
fn strict_visible_candles_share_ohlc_scale_coordinates() {
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
        PriceScaleMode::Logarithmic,
    ] {
        let (_, mut model, id) = candle_fixture(mode);
        let data = model.prepare_candlesticks(id).unwrap().unwrap();
        assert_eq!(data.items.len(), 5);
        for (i, p) in data.items.iter().enumerate() {
            let index = 4. + i as f64;
            let scale = model.panes()[0].right_price_scale();
            assert_eq!(
                [p.open_y, p.high_y, p.low_y, p.close_y],
                [100. + index, 112. + index, 90. + index, 102. + index]
                    .map(|v| scale.price_to_coordinate(v, 106.).value())
            );
            assert_eq!(
                p.x,
                model.time_scale().index_to_coordinate(index.into()).value()
            );
        }
    }
}
#[test]
fn candle_colors_whitespace_and_options_refresh_independently() {
    let (mut layer, mut model, id) = candle_fixture(PriceScaleMode::Normal);
    let mut overridden = candle(1., 110., 100.);
    if let BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(row)) = &mut overridden {
        row.color = Some("purple".into());
        row.wick_color = Some("cyan".into());
    }
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Candlestick,
                    vec![
                        candle(0., 100., 100.),
                        overridden,
                        BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Whitespace(
                            WhitespaceData {
                                time: Time::from(UtcTimestamp::new(2.)),
                                custom_values: None,
                            },
                        )),
                        candle(3., 100., 110.),
                    ],
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 0., 3.);
    let data = model.prepare_candlesticks(id).unwrap().unwrap();
    assert_eq!(data.items.len(), 3);
    assert_eq!(
        (&*data.items[0].color, &*data.items[0].border_color),
        ("green", "blue")
    );
    assert_eq!(
        (
            &*data.items[1].color,
            &*data.items[1].border_color,
            &*data.items[1].wick_color
        ),
        ("purple", "orange", "cyan")
    );
    assert_eq!(
        data.items[2].x - data.items[1].x,
        2. * model.time_scale().bar_spacing()
    );
    let mut options = candle_options();
    if let SeriesOptionsMap::Candlestick(ref mut o) = options {
        o.style.up_color = "yellow".into();
        o.style.wick_visible = false;
    }
    model.replace_series_options(id, options).unwrap();
    let data = model.prepare_candlesticks(id).unwrap().unwrap();
    assert_eq!(data.items[0].color, "yellow");
    assert_eq!(data.items[1].color, "purple");
    assert!(!data.wick_visible);
}
#[test]
fn candle_refresh_clears_stale_output_and_rejects_wrong_types() {
    let (mut layer, mut model, id) = candle_fixture(PriceScaleMode::Normal);
    let mut pane = CandlesticksPaneView::default();
    pane.refresh(&mut model, id).unwrap();
    assert!(pane.data().is_some());
    let old = pane.data().unwrap().clone();
    model
        .apply_data_update(
            layer
                .update_series_data(id, candle(19., 150., 170.), false)
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 18., 19.);
    pane.refresh(&mut model, id).unwrap();
    assert_ne!(pane.data().unwrap(), &old);
    let mut options = candle_options();
    if let SeriesOptionsMap::Candlestick(ref mut o) = options {
        o.common.visible = false;
    }
    model.replace_series_options(id, options).unwrap();
    pane.refresh(&mut model, id).unwrap();
    assert!(pane.data().is_none());
    model.replace_series_options(id, candle_options()).unwrap();
    model.set_width(0.).unwrap();
    pane.refresh(&mut model, id).unwrap();
    assert!(pane.data().is_none());
    model.set_width(600.).unwrap();
    view(&mut model, 18., 19.);
    pane.refresh(&mut model, id).unwrap();
    assert!(pane.data().is_some());
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    assert_eq!(
        pane.refresh(&mut model, id),
        Err(CandlestickPreparationError::UnknownSeries(id))
    );
    assert!(pane.data().is_none());
    assert!(!old.items.is_empty());
    let (_, mut model, id) = fixture();
    assert_eq!(
        model.prepare_candlesticks(id),
        Err(CandlestickPreparationError::NotCandlestickSeries(id))
    );
}
#[test]
fn candle_magnet_uses_close_or_ohlc_and_never_line_markers() {
    use crate::model::crosshair::{CrosshairMode, CrosshairOptions};
    use crate::renderers::line_renderer::Point;
    let (mut layer, mut model, id) = candle_fixture(PriceScaleMode::Normal);
    let x = model.time_scale().index_to_coordinate(6.0.into()).value();
    let scale = model.panes()[0].right_price_scale();
    let base = scale.first_value().unwrap();
    let open = scale.price_to_coordinate(106., base).value();
    let close = scale.price_to_coordinate(108., base).value();
    for (mode, y) in [
        (CrosshairMode::Normal, open + 0.1),
        (CrosshairMode::Magnet, close),
        (CrosshairMode::MagnetOhlc, open),
    ] {
        model
            .set_crosshair_options(CrosshairOptions {
                mode,
                ..Default::default()
            })
            .unwrap();
        model
            .set_crosshair_position(0, Some(Point { x, y: open + 0.1 }))
            .unwrap();
        assert!((model.crosshair().position().unwrap().y.unwrap() - y).abs() < 1e-8);
        assert!(model.crosshair_markers(0).is_empty());
        let axes = model.prepare_axis_snapshots(0).unwrap();
        assert!(axes.right.crosshair_label.is_some());
        assert!(axes.time.crosshair_label.is_some());
    }
    model
        .apply_data_update(
            layer
                .set_series_data(id, SeriesType::Candlestick, vec![])
                .unwrap(),
        )
        .unwrap();
    assert!(model.crosshair().position().is_none());
    assert!(model.prepare_candlesticks(id).unwrap().is_none());
}

use super::*;
use crate::model::{
    data_consumer::{HistogramData, HistogramDataItem},
    price_scale::PriceScaleMode,
    series::histogram_pane_view::{HistogramPaneView, HistogramPreparationError},
    series_options::HistogramStyleOptions,
};
fn histogram_options() -> SeriesOptionsMap {
    SeriesOptionsMap::Histogram(SeriesOptions {
        common: common(),
        style: HistogramStyleOptions {
            color: "green".into(),
            base: 0.,
        },
    })
}
fn histogram(at: f64, _open: f64, close: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
        time: Time::from(UtcTimestamp::new(at)),
        value: close,
        color: None,
        custom_values: None,
    }))
}
fn histogram_fixture(mode: PriceScaleMode) -> (Layer, Model, SeriesId) {
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
            SeriesType::Histogram,
            histogram_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Histogram,
                    (0..20)
                        .map(|i| histogram(i as f64, 100. + i as f64, 102. + i as f64))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 4., 8.);
    (layer, model, id)
}
#[test]
fn histogram_extended_range_and_selected_scale_projection() {
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
        PriceScaleMode::Logarithmic,
    ] {
        let (_, mut model, id) = histogram_fixture(mode);
        let data = model.prepare_histogram(id).unwrap().unwrap();
        assert_eq!(data.items.len(), 7);
        assert_eq!(data.items[0].index, 3.);
        for p in &data.items {
            let scale = model.panes()[0].right_price_scale();
            assert_eq!(p.y, scale.price_to_coordinate(102. + p.index, 106.).value());
        }
        assert_eq!(
            data.baseline,
            model.panes()[0]
                .right_price_scale()
                .price_to_coordinate(0., 106.)
                .value()
        );
    }
}
#[test]
fn histogram_colors_whitespace_and_options_refresh_independently() {
    let (mut layer, mut model, id) = histogram_fixture(PriceScaleMode::Normal);
    let mut overridden = histogram(1., 110., 100.);
    if let BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(row)) = &mut overridden {
        row.color = Some("purple".into());
    }
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Histogram,
                    vec![
                        histogram(0., 100., 100.),
                        overridden,
                        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Whitespace(
                            WhitespaceData {
                                time: Time::from(UtcTimestamp::new(2.)),
                                custom_values: None,
                            },
                        )),
                        histogram(3., 100., 110.),
                    ],
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 0., 3.);
    let data = model.prepare_histogram(id).unwrap().unwrap();
    assert_eq!(data.items.len(), 3);
    assert_eq!(data.items[0].color, "green");
    assert_eq!(data.items[1].color, "purple");
    assert_eq!(
        data.items[2].x - data.items[1].x,
        2. * model.time_scale().bar_spacing()
    );
    let mut options = histogram_options();
    if let SeriesOptionsMap::Histogram(ref mut o) = options {
        o.style.color = "yellow".into();
        o.style.base = -5.;
    }
    model.replace_series_options(id, options).unwrap();
    let data = model.prepare_histogram(id).unwrap().unwrap();
    assert_eq!(data.items[0].color, "yellow");
    assert_eq!(data.items[1].color, "purple");
    assert_eq!(
        data.baseline,
        model.panes()[0]
            .right_price_scale()
            .price_to_coordinate(-5., 100.)
            .value()
    );
}
#[test]
fn histogram_refresh_clears_stale_output_and_rejects_wrong_types() {
    let (mut layer, mut model, id) = histogram_fixture(PriceScaleMode::Normal);
    let mut pane = HistogramPaneView::default();
    pane.refresh(&mut model, id).unwrap();
    assert!(pane.data().is_some());
    let old = pane.data().unwrap().clone();
    model
        .apply_data_update(
            layer
                .update_series_data(id, histogram(19., 150., 170.), false)
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 18., 19.);
    pane.refresh(&mut model, id).unwrap();
    assert_ne!(pane.data().unwrap(), &old);
    let mut options = histogram_options();
    if let SeriesOptionsMap::Histogram(ref mut o) = options {
        o.common.visible = false;
    }
    model.replace_series_options(id, options).unwrap();
    pane.refresh(&mut model, id).unwrap();
    assert!(pane.data().is_none());
    model
        .replace_series_options(id, histogram_options())
        .unwrap();
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
        Err(HistogramPreparationError::UnknownSeries(id))
    );
    assert!(pane.data().is_none());
    assert!(!old.items.is_empty());
    let (_, mut model, id) = fixture();
    assert_eq!(
        model.prepare_histogram(id),
        Err(HistogramPreparationError::NotHistogramSeries(id))
    );
}

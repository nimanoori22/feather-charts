use super::*;
use crate::model::autoscale_info_impl::{AutoscaleInfo, PriceRange};
use std::cell::Cell;

fn install_provider(model: &mut Model, id: SeriesId, calls: Rc<Cell<usize>>, high: Rc<Cell<f64>>) {
    let SeriesOptionsMap::Line(mut options) = line_options() else {
        unreachable!()
    };
    options.common.autoscale_info_provider = Some(Box::new(move |_| {
        calls.set(calls.get() + 1);
        Some(AutoscaleInfo {
            price_range: Some(PriceRange {
                min_value: 0.,
                max_value: high.get(),
            }),
            margins: None,
        })
    }));
    model
        .replace_series_options(id, SeriesOptionsMap::Line(options))
        .unwrap();
}

#[test]
fn offscreen_updates_coalesce_until_a_range_is_requested() {
    let (mut layer, mut model, id) = fixture();
    view(&mut model, 5., 10.);
    let calls = Rc::new(Cell::new(0));
    let high = Rc::new(Cell::new(30.));
    install_provider(&mut model, id, calls.clone(), high.clone());
    model.panes()[0].right_price_scale().price_range();
    calls.set(0);
    for at in [21., 22.] {
        model
            .apply_data_update(
                layer
                    .update_series_data(id, line(at, 1000.), false)
                    .unwrap(),
            )
            .unwrap();
    }
    assert_eq!(
        calls.get(),
        0,
        "unread invalidations must not execute providers"
    );
    high.set(80.);
    assert_eq!(
        model.panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .max_value(),
        80.
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(
        model.panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .max_value(),
        80.
    );
    assert_eq!(
        calls.get(),
        1,
        "a repeated read must reuse the settled range"
    );
    assert!(
        model.take_invalidation().is_some(),
        "offscreen data still requires a light redraw"
    );
}

#[test]
fn coordinate_reads_between_updates_observe_each_new_range() {
    let (mut layer, mut model, id) = fixture();
    view(&mut model, 5., 10.);
    let calls = Rc::new(Cell::new(0));
    let high = Rc::new(Cell::new(20.));
    install_provider(&mut model, id, calls.clone(), high.clone());
    model.panes()[0].right_price_scale().price_range();
    calls.set(0);
    for (at, value) in [(21., 40.), (22., 80.)] {
        model
            .apply_data_update(
                layer
                    .update_series_data(id, line(at, 1000.), false)
                    .unwrap(),
            )
            .unwrap();
        high.set(value);
        let scale = model.panes()[0].right_price_scale();
        let transform = scale.coordinate_transform(1.);
        let coord = transform.price_to_coordinate(value);
        assert!((scale.coordinate_to_price(coord, 1.) - value).abs() < 1e-9);
        assert_eq!(scale.price_range().unwrap().max_value(), value);
    }
    assert_eq!(calls.get(), 2);
}

#[test]
fn active_crosshair_still_demands_autoscale_during_an_update() {
    let (mut layer, mut model, id) = fixture();
    let calls = Rc::new(Cell::new(0));
    let high = Rc::new(Cell::new(30.));
    install_provider(&mut model, id, calls.clone(), high);
    model
        .set_crosshair_position(
            0,
            Some(crate::renderers::line_renderer::Point { x: 50., y: 50. }),
        )
        .unwrap();
    calls.set(0);
    model
        .apply_data_update(layer.update_series_data(id, line(21., 50.), false).unwrap())
        .unwrap();
    assert!(
        calls.get() > 0,
        "crosshair coordinate conversion is a real demand"
    );
}

#[test]
fn offscreen_updates_requery_unchanged_sources_at_the_next_demand() {
    let (mut layer, mut model, first) = fixture();
    let second = SeriesId::new(2);
    model
        .register_series(
            second,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    second,
                    SeriesType::Line,
                    (1..=20).map(|n| line(n as f64, 10.)).collect(),
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 5., 10.);
    let a = Rc::new(Cell::new(0));
    let b = Rc::new(Cell::new(0));
    let high = Rc::new(Cell::new(30.));
    install_provider(&mut model, first, a.clone(), high.clone());
    install_provider(&mut model, second, b.clone(), high.clone());
    model.panes()[0].right_price_scale().price_range();
    a.set(0);
    b.set(0);
    model
        .apply_data_update(
            layer
                .update_series_data(first, line(21., 100.), false)
                .unwrap(),
        )
        .unwrap();
    assert_eq!((a.get(), b.get()), (0, 0));
    high.set(90.);
    assert_eq!(
        model.panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .max_value(),
        90.
    );
    assert_eq!((a.get(), b.get()), (1, 1));
}

#[test]
fn primitive_and_custom_callbacks_settle_after_offscreen_builtin_update() {
    let (mut layer, mut model, first) = fixture();
    let custom = SeriesId::new(2);
    let custom_calls = Rc::new(Cell::new(0));
    let calls = custom_calls.clone();
    let mut options = CustomSeriesOptions {
        common: common(),
        style: CustomStyleOptions {
            color: "#000".into(),
        },
    };
    options.common.autoscale_info_provider = Some(Box::new(move |base| {
        calls.set(calls.get() + 1);
        base()
    }));
    let handle = Rc::new(RefCell::new(CustomSeries::new(custom, options)));
    model
        .register_custom_series(handle.clone(), 0, PriceScalePosition::Right)
        .unwrap();
    model
        .apply_custom_data_update(
            &handle,
            layer
                .set_custom_series_data(
                    custom,
                    (1..=20)
                        .map(|n| {
                            CustomSeriesDataItem::Data(Datum {
                                at: time(n as f64),
                                value: 10.,
                            })
                        })
                        .collect(),
                    &Builder,
                )
                .unwrap(),
        )
        .unwrap();
    let primitive_calls = Rc::new(Cell::new(0));
    let calls = primitive_calls.clone();
    model.set_primitive_autoscale(
        first,
        1,
        Some(Rc::new(move |_, _| {
            calls.set(calls.get() + 1);
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -50.,
                    max_value: 50.,
                }),
                margins: None,
            })
        })),
    );
    view(&mut model, 5., 10.);
    model.panes()[0].right_price_scale().price_range();
    custom_calls.set(0);
    primitive_calls.set(0);
    for at in [21., 22.] {
        model
            .apply_data_update(
                layer
                    .update_series_data(first, line(at, 20.), false)
                    .unwrap(),
            )
            .unwrap();
    }
    assert_eq!((custom_calls.get(), primitive_calls.get()), (0, 0));
    let range = model.panes()[0].right_price_scale().price_range().unwrap();
    assert_eq!((range.min_value(), range.max_value()), (-50., 50.));
    assert_eq!((custom_calls.get(), primitive_calls.get()), (1, 1));
}

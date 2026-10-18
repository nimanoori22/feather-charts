use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        data_layer::SeriesId,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
        time_data::{Logical, LogicalRange},
        time_scale_options::HorzScaleOptionsPatch,
    },
};
use iced::Size;
use std::time::{Duration, Instant};
fn item(i: usize) -> Time {
    UtcTimestamp::new(i as f64).into()
}
fn prepare(c: &mut Chart) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
fn chart() -> (Chart, SeriesId) {
    let mut c = Chart::default();
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        bar_spacing: Some(9.),
        right_offset: Some(3.),
        ..Default::default()
    });
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (1..=100)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: item(i),
                    value: 100. + i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    prepare(&mut c);
    (c, id)
}
#[test]
fn range_queries_navigation_and_event_queue() {
    let (mut c, _) = chart();
    c.set_event_interests(EventInterests::ALL);
    c.set_visible_horizontal_range(HorizontalRange {
        from: item(20),
        to: item(40),
    })
    .unwrap();
    prepare(&mut c);
    let range = c.visible_horizontal_range().unwrap();
    assert_eq!(
        range,
        HorizontalRange {
            from: item(20),
            to: item(40)
        }
    );
    let logical = c.visible_logical_range().unwrap();
    assert!((logical.from.value() - 19.).abs() < 1e-8);
    assert!((logical.to.value() - 39.).abs() < 1e-8);
    assert!(!c.take_events().is_empty(), "queries must not drain events");
    assert!(c.take_events().is_empty());
    let saved = c.visible_horizontal_range();
    c.scroll_to_offset_animated(-30., Duration::from_secs(2));
    c.reset_time_scale();
    prepare(&mut c);
    assert_eq!(c.scroll_position(), 3.);
    assert_eq!(c.model().time_scale().bar_spacing(), 9.);
    assert!(!c.is_animating());
    assert_eq!(saved, Some(range));
    c.scroll_to_real_time();
    let now = Instant::now();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    c.prepare(Size::new(800., 500.), 1., now + Duration::from_secs(1))
        .unwrap();
    assert_eq!(c.scroll_position(), 3.);
}
#[test]
fn empty_and_invalid_range_are_explicit() {
    let mut c = Chart::default();
    assert!(c.visible_horizontal_range().is_none());
    assert!(c.visible_logical_range().is_none());
    c.set_visible_horizontal_range(HorizontalRange {
        from: item(1),
        to: item(2),
    })
    .unwrap();
    let (mut c, _) = chart();
    let before = c.visible_logical_range();
    assert!(
        c.set_visible_horizontal_range(HorizontalRange {
            from: item(40),
            to: item(20)
        })
        .is_err()
    );
    assert_eq!(c.visible_logical_range(), before);
    c.set_logical_range(LogicalRange {
        from: Logical::new(10.25),
        to: Logical::new(30.75),
    });
    prepare(&mut c);
    assert!((c.visible_logical_range().unwrap().from.value() - 10.25).abs() < 1e-8);
}

#[path = "numeric_custom_series/support.rs"]
mod support;
#[test]
fn readbacks_follow_owners_and_retained_handles() {
    use feather_charts::model::{
        horz_scale_behavior_time::types::TimePoint, series_options::SeriesType,
    };
    let (mut c, id) = chart();
    let h = c
        .add_custom_series(
            support::View::<Time, TimePoint>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(
        &h,
        vec![support::datum(item(1), 10.), support::datum(item(2), 20.)],
    )
    .unwrap();
    let a = c
        .add_price_line(h.id(), PriceLineOptions::new(12.))
        .unwrap();
    let b = c
        .add_price_line(h.id(), PriceLineOptions::new(15.))
        .unwrap();
    assert_eq!(c.price_lines(h.id()).unwrap(), vec![a, b]);
    assert_eq!(c.series_type(h.id()).unwrap(), SeriesType::Custom);
    assert_eq!(c.series_type(id).unwrap(), SeriesType::Line);
    assert!(
        c.with_series_options(id, |o| match o {
            SeriesOptionsMap::Line(o) => o.common.visible,
            _ => false,
        })
        .unwrap()
    );
    assert!(h.with_options(|o| o.common.visible));
    assert!(h.with_view_options(&c, |o| *o).unwrap());
    assert!(
        !c.with_series_formatter(h.id(), |f| f.format(12.))
            .unwrap()
            .is_empty()
    );
    c.move_series_to_pane(h.id(), 1).unwrap();
    c.set_series_price_scale(h.id(), PriceScalePosition::Left)
        .unwrap();
    assert_eq!(c.series_pane(h.id()).unwrap(), 1);
    assert_eq!(
        h.with_options(|o| o.common.price_scale_id.clone()),
        Some("left".into())
    );
    prepare(&mut c);
    c.fit_content();
    prepare(&mut c);
    assert_eq!(c.last_value_data(h.id(), true).unwrap().unwrap().price, 20.);
    let lines = c.price_lines(h.id()).unwrap();
    c.remove_price_line(a).unwrap();
    assert_eq!(c.price_lines(h.id()).unwrap(), vec![b]);
    assert_eq!(lines, vec![a, b], "enumeration is an owned snapshot");
    c.remove_series(h.id()).unwrap();
    assert!(c.price_lines(h.id()).is_err());
    assert!(c.series_pane(h.id()).is_err());
    assert!(h.with_view_options(&c, |o| *o).is_err());
    assert!(h.with_options(|o| o.common.visible));
    assert!(!h.with_formatter(|f| f.format(12.)).is_empty());
    let foreign = Chart::default();
    assert!(foreign.series_type(id).is_err());
}

#[test]
fn conversions_follow_scale_modes_and_synthetic_crosshair_is_silent() {
    use feather_charts::model::{
        crosshair::{CrosshairMode, CrosshairOptions},
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    let (mut c, id) = chart();
    c.fit_content();
    prepare(&mut c);
    c.set_crosshair_options(CrosshairOptions {
        mode: CrosshairMode::Normal,
        ..Default::default()
    })
    .unwrap();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        c.apply_price_scale_options(
            0,
            &PriceScalePosition::Right,
            PriceScaleOptionsPatch {
                mode: Some(mode),
                ..Default::default()
            },
        )
        .unwrap();
        prepare(&mut c);
        let y = c.price_to_coordinate(id, 150.).unwrap().unwrap();
        let p = c.coordinate_to_price(id, y).unwrap().unwrap();
        assert!((p - 150.).abs() < 1e-6, "{mode:?}: {p}");
    }
    c.take_events();
    assert!(c.set_crosshair_at(id, &item(50), 150.).unwrap());
    assert!(
        c.take_events().is_empty(),
        "synthetic crosshair must not recurse through public events"
    );
    assert_eq!(c.model().crosshair().position().unwrap().index.value(), 49.);
    c.clear_crosshair_position().unwrap();
    assert!(c.model().crosshair().origin().is_none());
    c.move_series_to_pane(id, 1).unwrap();
    c.set_series_price_scale(id, PriceScalePosition::Overlay("private".into()))
        .unwrap();
    prepare(&mut c);
    assert!(c.price_to_coordinate(id, 150.).unwrap().is_some());
    assert!(c.set_crosshair_at(id, &item(50), 150.).unwrap());
    assert_eq!(
        c.model().crosshair().origin().unwrap().pane,
        c.series_pane(id).unwrap()
    );
    assert!(c.price_to_coordinate(id, f64::NAN).is_err());
    assert!(c.coordinate_to_price(id, f64::INFINITY).is_err());
    let x = c.horizontal_to_coordinate(&item(50)).unwrap().unwrap();
    assert_eq!(c.coordinate_to_horizontal(x), Some(item(50)));
    assert_eq!(c.coordinate_to_logical(x).unwrap(), 49.);
    assert_eq!(c.logical_to_coordinate(49.), Some(x));
    assert!(c.horizontal_to_coordinate(&item(0)).unwrap().is_none());
    assert!(c.coordinate_to_horizontal(-1e12).is_none());
    c.remove_series(id).unwrap();
    assert!(c.price_to_coordinate(id, 150.).is_err());
    assert!(c.set_crosshair_at(id, &item(50), 150.).is_err());
}

#[test]
fn source_public_helper_states() {
    let (mut c, id) = chart();
    c.set_visible_horizontal_range(HorizontalRange {
        from: item(20),
        to: item(40),
    })
    .unwrap();
    prepare(&mut c);
    let r = c.visible_horizontal_range().unwrap();
    assert_eq!((r.from, r.to), (item(20), item(40)));
    let logical = c.visible_logical_range().unwrap();
    let global = c.last_value_data(id, true).unwrap().unwrap().price;
    let visible = c.last_value_data(id, false).unwrap().unwrap().price;
    c.add_price_line(id, PriceLineOptions::new(120.)).unwrap();
    c.add_price_line(id, PriceLineOptions::new(150.)).unwrap();
    c.fit_content();
    prepare(&mut c);
    use feather_charts::model::{
        crosshair::{CrosshairMode, CrosshairOptions},
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    c.set_crosshair_options(CrosshairOptions {
        mode: CrosshairMode::Normal,
        ..Default::default()
    })
    .unwrap();
    let mut roundtrips = Vec::new();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        c.apply_price_scale_options(
            0,
            &PriceScalePosition::Right,
            PriceScaleOptionsPatch {
                mode: Some(mode),
                ..Default::default()
            },
        )
        .unwrap();
        prepare(&mut c);
        let y = c.price_to_coordinate(id, 150.).unwrap().unwrap();
        roundtrips.push(c.coordinate_to_price(id, y).unwrap().unwrap().round() as i32);
    }
    c.take_events();
    c.set_crosshair_at(id, &item(50), 150.).unwrap();
    let index = c.model().crosshair().position().unwrap().index.value();
    c.clear_crosshair_position().unwrap();
    let events = c.take_events().len();
    let nearest = [
        c.horizontal_to_index(&item(0), true)
            .unwrap()
            .unwrap()
            .value() as i32,
        c.horizontal_to_index(&item(101), true)
            .unwrap()
            .unwrap()
            .value() as i32,
    ];
    assert!(c.horizontal_to_index(&item(0), false).unwrap().is_none());
    println!(
        "HELPER_REFERENCE {{\"range\":[20,40],\"logical\":[{},{}],\"global\":{global},\"visible\":{visible},\"roundtrips\":{roundtrips:?},\"index\":{index},\"events\":{events},\"lines\":{},\"nearest\":{nearest:?},\"missing\":null}}",
        logical.from.value().round(),
        logical.to.value().round(),
        c.price_lines(id).unwrap().len()
    );
}

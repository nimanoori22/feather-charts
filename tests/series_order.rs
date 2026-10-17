use feather_charts::{
    api::*,
    model::{data_layer::SeriesId, pane::PriceScalePosition, series_options::SeriesOptionsMap},
};
use std::time::Instant;
fn add(c: &mut Chart, pane: usize) -> SeriesId {
    c.add_series(
        SeriesOptionsMap::Line(Default::default()),
        pane,
        PriceScalePosition::Right,
    )
    .unwrap()
}
#[test]
fn public_order_changes_the_next_prepared_layer_order() {
    let mut c = Chart::default();
    let a = add(&mut c, 0);
    let b = add(&mut c, 0);
    assert_eq!(c.series_order(a).unwrap(), 0);
    c.set_series_order(a, usize::MAX).unwrap();
    assert_eq!(c.series_order(a).unwrap(), 1);
    let frame = c
        .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(frame.panes[0].plot.series_id, Some(b));
    assert_eq!(frame.panes[0].plot.series_layers[0].series_id, Some(a));
    assert!(c.set_series_order(SeriesId::new(u64::MAX), 0).is_err());
    c.remove_series(a).unwrap();
    assert!(c.series_order(a).is_err());
    assert_eq!(c.series_order(b).unwrap(), 0);
}

#[path = "numeric_custom_series/support.rs"]
mod support;
#[test]
fn source_reference_sequence() {
    let mut c = Chart::default();
    let a = c
        .add_series(
            SeriesOptionsMap::Area(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let b = add(&mut c, 0);
    let custom = c
        .add_custom_series(
            support::View::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let mut ids = vec![a, b, custom.id()];
    fn snapshot(c: &mut Chart, ids: &[SeriesId], name: &str) {
        let order: Vec<Vec<_>> = c
            .model()
            .panes()
            .iter()
            .map(|p| {
                p.attached_source_ids()
                    .iter()
                    .map(|id| ids.iter().position(|i| i == id).unwrap())
                    .collect()
            })
            .collect();
        let indices: Vec<_> = ids.iter().map(|id| c.series_order(*id).ok()).collect();
        let frame = c
            .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        for (p, expected) in frame.panes.iter().zip(&order) {
            let layers: Vec<_> = std::iter::once(p.plot.series_id)
                .chain(p.plot.series_layers.iter().map(|s| s.series_id))
                .flatten()
                .map(|id| ids.iter().position(|i| *i == id).unwrap())
                .collect();
            assert_eq!(&layers, expected);
        }
        let indices = indices
            .iter()
            .map(|v| v.map_or("null".into(), |v| v.to_string()))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "ORDER_REFERENCE {{\"name\":\"{name}\",\"order\":{order:?},\"indices\":[{indices}]}}"
        );
    }
    snapshot(&mut c, &ids, "initial");
    c.set_series_order(custom.id(), 0).unwrap();
    snapshot(&mut c, &ids, "custom_back");
    c.set_series_order(a, 99).unwrap();
    snapshot(&mut c, &ids, "area_front");
    c.set_series_order(b, 1).unwrap();
    snapshot(&mut c, &ids, "middle");
    c.set_series_order(a, 99).unwrap();
    snapshot(&mut c, &ids, "same");
    c.add_pane();
    c.move_series_to_pane(b, 1).unwrap();
    snapshot(&mut c, &ids, "transfer");
    c.move_series_to_pane(custom.id(), 1).unwrap();
    snapshot(&mut c, &ids, "custom_transfer");
    c.set_series_order(custom.id(), 0).unwrap();
    snapshot(&mut c, &ids, "target_back");
    c.remove_series(b).unwrap();
    snapshot(&mut c, &ids, "remove");
    ids.push(add(&mut c, 1));
    snapshot(&mut c, &ids, "append");
}

#[test]
fn maturity_indices_count_support_but_its_id_stays_protected() {
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let a = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    let b = c
        .add_area_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    assert_eq!(
        [c.series_order(a).unwrap(), c.series_order(b).unwrap()],
        [1, 2]
    );
    let support = c.model().panes()[0].attached_source_ids()[0];
    assert!(c.series_order(support).is_err());
    assert!(c.set_series_order(support, 99).is_err());
    c.set_series_order(b, 0).unwrap();
    assert_eq!(
        [c.series_order(a).unwrap(), c.series_order(b).unwrap()],
        [2, 0]
    );
    c.set_series_order(a, 99).unwrap();
    c.add_pane();
    c.move_series_to_pane(a, 1).unwrap();
    assert_eq!(
        [c.series_order(a).unwrap(), c.series_order(b).unwrap()],
        [0, 0]
    );
    c.move_series_to_pane(a, 0).unwrap();
    c.set_series_order(a, 0).unwrap();
    let frame = c
        .prepare_snapshot(iced::Size::new(300., 90.), 1.5, Instant::now())
        .unwrap();
    assert_eq!(frame.panes[0].plot.series_id, Some(a));
    assert_eq!(frame.panes[0].plot.series_layers[0].series_id, Some(b));
}

#[test]
fn custom_reordering_keeps_canonical_rows_and_attachment_owners() {
    use std::{cell::Cell, rc::Rc};
    use support::*;
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let view = View::<HorizontalPrice, HorizontalPrice>::default();
    let destroyed = view.destroyed.clone();
    let h = c
        .add_custom_series(view, Default::default(), 0, PriceScalePosition::Right)
        .unwrap();
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let copies = Rc::new(Cell::new(0));
    c.set_custom_data(
        &h,
        (0..4096)
            .map(|i| {
                feather_charts::model::icustom_series::CustomSeriesDataItem::Data(Datum {
                    time: HorizontalPrice::new(i as f64).unwrap(),
                    low: 1.,
                    high: 3.,
                    value: 2.,
                    clones: Some(copies.clone()),
                })
            })
            .collect(),
    )
    .unwrap();
    let attachment = c
        .add_series_primitive(h.id(), TextWatermark::new(Default::default()).unwrap())
        .unwrap();
    let owner = c.primitive_owner(attachment).unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    copies.set(0);
    for index in [usize::MAX, 0, 1, 0] {
        c.set_series_order(h.id(), index).unwrap();
        assert_eq!(copies.get(), 0);
        assert_eq!(destroyed.get(), 0);
        assert_eq!(c.primitive_owner(attachment).unwrap(), owner);
    }
    assert_eq!(c.series_order(peer).unwrap(), 1);
    c.update_custom_data(&h, datum(HorizontalPrice::new(4095.).unwrap(), 2.5), false)
        .unwrap();
    assert_eq!(h.data_at(4095.0.into()).unwrap().value, 2.5);
    c.remove_series(h.id()).unwrap();
    assert_eq!(destroyed.get(), 1);
}

#[test]
fn reordering_refreshes_native_formatter_leadership_on_all_scales() {
    use feather_charts::model::series_options::*;
    use std::rc::Rc;
    for side in [
        PriceScalePosition::Left,
        PriceScalePosition::Right,
        PriceScalePosition::Overlay("study".into()),
    ] {
        let mut c = Chart::default();
        let mut ids = vec![];
        for prefix in ["A", "B"] {
            let mut o = LineSeriesOptions::default();
            o.common.price_format = PriceFormat::Custom(PriceFormatCustom {
                formatter: Rc::new(move |v| format!("{prefix}{:.2}", v.value())),
                tickmarks_formatter: None,
                min_move: 0.01,
                base: Some(100.),
            });
            ids.push(
                c.add_series(SeriesOptionsMap::Line(o), 0, side.clone())
                    .unwrap(),
            );
        }
        let format = |c: &Chart| {
            c.model().panes()[0]
                .price_scale_by_id(match &side {
                    PriceScalePosition::Left => "left",
                    PriceScalePosition::Right => "right",
                    PriceScalePosition::Overlay(id) => id,
                })
                .unwrap()
                .format_price_absolute(12.5)
        };
        assert_eq!(format(&c), "A12.50");
        c.set_series_order(ids[1], 0).unwrap();
        assert_eq!(format(&c), "B12.50");
        let mut hidden = LineSeriesOptions::default();
        hidden.common.visible = false;
        c.replace_series_options(ids[1], SeriesOptionsMap::Line(hidden))
            .unwrap();
        assert_eq!(format(&c), "A12.50");
    }
}

use feather_charts::model::{
    data_consumer::*,
    horz_scale_behavior_time::types::{Time, UtcTimestamp},
};
fn sample(kind: usize, i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    use feather_charts::model::data_consumer::*;
    let time = UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into();
    match kind {
        0 => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value,
            color: None,
            custom_values: None,
        })),
        1 => BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open: value - 1.,
            high: value + 2.,
            low: value - 2.,
            close: value,
            color: None,
            border_color: None,
            wick_color: None,
            custom_values: None,
        })),
        2 => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: value - 1.,
            high: value + 2.,
            low: value - 2.,
            close: value,
            color: None,
            custom_values: None,
        })),
        3 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time,
            value,
            color: None,
            custom_values: None,
        })),
        4 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
        5 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
            time,
            value,
            top_fill_color1: None,
            top_fill_color2: None,
            top_line_color: None,
            bottom_fill_color1: None,
            bottom_fill_color2: None,
            bottom_line_color: None,
            custom_values: None,
        })),
        _ => unreachable!(),
    }
}
fn options(kind: usize) -> SeriesOptionsMap {
    match kind {
        0 => SeriesOptionsMap::Line(Default::default()),
        1 => SeriesOptionsMap::Candlestick(Default::default()),
        2 => SeriesOptionsMap::Bar(Default::default()),
        3 => SeriesOptionsMap::Histogram(Default::default()),
        4 => SeriesOptionsMap::Area(Default::default()),
        5 => SeriesOptionsMap::Baseline(Default::default()),
        _ => unreachable!(),
    }
}

#[test]
fn all_builtin_owners_keep_rows_attachments_and_frames_across_order_changes() {
    use feather_charts::model::series_markers::*;
    for kind in 0..6 {
        let mut c = Chart::default();
        let a = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        let b = add(&mut c, 0);
        c.set_data(
            a,
            (0..1000)
                .map(|i| sample(kind, i, 100. + (i % 10) as f64))
                .collect(),
        )
        .unwrap();
        let markers = c
            .add_series_markers(
                a,
                vec![SeriesMarker::new(
                    UtcTimestamp::new(1_700_000_000. + 990. * 60.).into(),
                    MarkerPosition::InBar,
                    MarkerShape::Circle,
                    "red",
                )],
                Default::default(),
            )
            .unwrap();
        let line = c.add_price_line(a, PriceLineOptions::new(105.)).unwrap();
        c.fit_content();
        let rows = c.model().series(a).unwrap().bars().rows().as_ptr();
        let old = c
            .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        c.set_series_order(a, usize::MAX).unwrap();
        assert_eq!(c.model().series(a).unwrap().bars().rows().as_ptr(), rows);
        let new = c
            .prepare_snapshot(iced::Size::new(300., 90.), 1.5, Instant::now())
            .unwrap();
        assert_eq!(new.panes[0].plot.series_id, Some(b));
        assert!(
            new.panes[0]
                .plot
                .annotations
                .iter()
                .filter(|collection| collection.series == a)
                .all(|a| a.series_layer == 1)
        );
        assert_eq!(c.series_markers(markers).unwrap().len(), 1);
        c.remove_price_line(line).unwrap();
        c.update_data(a, sample(kind, 999, 104.), false).unwrap();
        c.update_data(a, sample(kind, 20, 103.), true).unwrap();
        c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        drop(c);
        assert_eq!(old.panes[0].plot.series_id, Some(a));
    }
}

#[test]
fn stable_overlap_selection_changes_with_order_and_hover_raising_does_not_rewrite_it() {
    use feather_charts::ui::{chart_input::ChartInteraction, hit_testing::HitGeometry};
    let mut c = Chart::default();
    let a = add(&mut c, 0);
    let b = add(&mut c, 0);
    for id in [a, b] {
        c.set_data(
            id,
            (0..20)
                .map(|i| sample(0, i, 100. + (i % 10) as f64))
                .collect(),
        )
        .unwrap();
    }
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let point = match &c.frame().plot.hit_series[0].geometry {
        HitGeometry::Line { data, .. } => {
            iced::Point::new(data.items[10].x as f32, data.items[10].y as f32)
        }
        _ => panic!("line geometry"),
    };
    assert_eq!(c.hit_test(0, point).unwrap().series, b);
    c.set_series_order(a, usize::MAX).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(c.hit_test(0, point).unwrap().series, a);
    for raised in [true, false] {
        c.set_hovered_series_on_top(raised);
        c.update(ChartMessage::Interaction(ChartInteraction {
            position: Some(point),
            navigation: None,
            capture: false,
        }))
        .unwrap();
        c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        assert_eq!(c.series_order(a).unwrap(), 1);
        assert_eq!(c.hit_test(0, point).unwrap().series, a);
    }
    // Test transient draw raising independently from stable selection, including
    // when the raised source would otherwise lose an equal-distance overlap.
    let mut frozen = c
        .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let plot = &mut frozen.panes[0].plot;
    plot.hovered_series = Some(b);
    plot.hovered_series_on_top = true;
    assert_eq!(plot.series_draw_passes().last().unwrap().layer, 0);
    assert_eq!(c.series_order(b).unwrap(), 0);
}

#[derive(Clone, Debug)]
struct LayerHit(PrimitiveLayer);
impl DrawingPrimitive for LayerHit {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(self.0, self.clone())],
            ..Default::default()
        }
    }
}
impl PrimitiveRenderer for LayerHit {
    fn draw(&self, _: &mut iced::widget::canvas::Frame) {}
    fn hit_test(&self, _: iced::Point) -> Option<PrimitiveHitCandidate> {
        let mut h = PrimitiveHitCandidate::new("overlap");
        h.layer = self.0;
        Some(h)
    }
}
#[test]
fn primitive_layer_ties_follow_reordered_owners_without_lifecycle_churn() {
    for layer in [
        PrimitiveLayer::Bottom,
        PrimitiveLayer::Normal,
        PrimitiveLayer::Top,
    ] {
        let mut c = Chart::default();
        let a = add(&mut c, 0);
        let b = add(&mut c, 0);
        let pa = c.add_series_primitive(a, LayerHit(layer)).unwrap();
        let pb = c.add_series_primitive(b, LayerHit(layer)).unwrap();
        c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        assert_eq!(
            c.primitive_hit(0, iced::Point::new(40., 40.))
                .unwrap()
                .primitive,
            pb
        );
        c.set_series_order(a, 1).unwrap();
        c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        assert_eq!(
            c.primitive_hit(0, iced::Point::new(40., 40.))
                .unwrap()
                .primitive,
            pa
        );
        assert_eq!(c.primitive_owner(pa).unwrap(), PrimitiveOwner::Series(a));
    }
}

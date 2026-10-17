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

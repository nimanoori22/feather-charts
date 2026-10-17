use feather_charts::{
    api::*, model::pane::PriceScalePosition, model::series_options::SeriesOptionsMap,
};
#[path = "numeric_custom_series/support.rs"]
mod support;
use feather_charts::model::icustom_series::CustomSeriesDataItem;

#[test]
fn yield_removal_relocates_private_support_without_exposing_or_shrinking_it() {
    use support::*;
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    c.add_pane();
    let h = c
        .add_custom_series(
            View::<Maturity, Maturity>::default(),
            Default::default(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    c.set_custom_data(&h, vec![datum(Maturity::from_months(360.).unwrap(), 4.)])
        .unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    c.swap_panes(0, 1).unwrap();
    c.move_pane(1, 0).unwrap();
    c.move_series_to_pane(h.id(), 1).unwrap();
    assert_eq!(
        c.model().panes().len(),
        2,
        "private support preserves its source pane"
    );
    c.remove_pane(0).unwrap();
    assert_eq!(c.series_ids().collect::<Vec<_>>(), [h.id()]);
    assert_eq!(c.model().pane_for_series(h.id()), Some(0));
    c.update_custom_data(&h, datum(Maturity::from_months(420.).unwrap(), 3.5), false)
        .unwrap();
    c.prepare(iced::Size::new(300., 90.), 1., std::time::Instant::now())
        .unwrap();
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&Maturity::from_months(420.).unwrap(), false)
            .is_some()
    );
    c.add_pane();
    c.remove_pane(0).unwrap();
    assert!(c.series_ids().next().is_none());
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&Maturity::from_months(420.).unwrap(), false)
            .is_some()
    );
}

#[test]
fn removal_cleans_requested_pane_once_and_preserves_survivors() {
    use support::*;
    for target in 0..3 {
        let mut c = OptionsChart::new_options(Default::default()).unwrap();
        c.add_pane();
        c.add_pane();
        let mut handles = vec![];
        let mut drops = vec![];
        let mut primitives = vec![];
        for pane in 0..3 {
            let view = View::<HorizontalPrice, HorizontalPrice>::default();
            drops.push(view.destroyed.clone());
            let h = c
                .add_custom_series(view, Default::default(), pane, PriceScalePosition::Right)
                .unwrap();
            c.set_custom_data(
                &h,
                vec![datum(HorizontalPrice::new(pane as f64 + 1.).unwrap(), 4.)],
            )
            .unwrap();
            primitives.push(
                c.add_pane_primitive(pane, TextWatermark::new(Default::default()).unwrap())
                    .unwrap(),
            );
            handles.push(h);
        }
        c.set_preserve_empty_pane(target, true).unwrap();
        assert!(c.remove_pane(3).is_err());
        c.remove_pane(target).unwrap();
        assert_eq!(c.model().panes().len(), 2);
        for i in 0..3 {
            assert_eq!(drops[i].get(), usize::from(i == target));
            assert_eq!(c.primitive_owner(primitives[i]).is_err(), i == target);
            assert_eq!(
                c.model().pane_for_series(handles[i].id()),
                if i == target {
                    None
                } else {
                    Some(if i > target { i - 1 } else { i })
                }
            );
        }
        assert_eq!(
            handles[target]
                .data_at((target as f64).into())
                .unwrap()
                .value,
            4.
        );
        assert!(c.set_custom_data(&handles[target], vec![]).is_err());
        c.remove_pane(0).unwrap();
        assert_eq!(c.model().panes().len(), 1);
        c.remove_pane(usize::MAX).unwrap();
        assert_eq!(c.model().panes().len(), 1);
    }
}

#[test]
fn pane_source_reference_sequence() {
    let mut c = Chart::default();
    c.add_pane();
    c.add_pane();
    let ids: Vec<_> = (0..3)
        .map(|p| {
            c.add_series(
                SeriesOptionsMap::Line(Default::default()),
                p,
                PriceScalePosition::Right,
            )
            .unwrap()
        })
        .collect();
    let snapshot = |c: &Chart, name: &str| {
        let owners: Vec<_> = ids
            .iter()
            .map(|id| c.model().pane_for_series(*id).unwrap())
            .collect();
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
        println!(
            "PANE_REFERENCE {{\"name\":\"{name}\",\"series\":{owners:?},\"order\":{order:?}}}"
        );
    };
    snapshot(&c, "initial");
    c.move_pane(0, 2).unwrap();
    snapshot(&c, "move");
    c.swap_panes(0, 2).unwrap();
    snapshot(&c, "swap");
    c.move_series_to_pane(ids[1], 0).unwrap();
    snapshot(&c, "transfer_cleanup");
    c.move_series_to_pane(ids[0], 99).unwrap();
    snapshot(&c, "append_clamped");
    c.move_series_to_pane(ids[0], c.model().panes().len() - 1)
        .unwrap();
    snapshot(&c, "same");
}

#[test]
fn custom_transfer_keeps_view_data_and_attachment_owners() {
    use std::{cell::Cell, rc::Rc, time::Instant};
    use support::*;
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    c.add_pane();
    c.set_preserve_empty_pane(0, true).unwrap();
    let view = View::<HorizontalPrice, HorizontalPrice>::default();
    let destroyed = view.destroyed.clone();
    let h = c
        .add_custom_series(view, Default::default(), 0, PriceScalePosition::Right)
        .unwrap();
    let copies = Rc::new(Cell::new(0));
    c.set_custom_data(
        &h,
        (0..2048)
            .map(|i| {
                CustomSeriesDataItem::Data(Datum {
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
    let a = c
        .add_pane_primitive(0, TextWatermark::new(Default::default()).unwrap())
        .unwrap();
    let b = c
        .add_series_primitive(h.id(), TextWatermark::new(Default::default()).unwrap())
        .unwrap();
    let before = c.primitive_owner(a).unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    copies.set(0);
    c.move_series_to_pane(h.id(), 1).unwrap();
    assert_eq!(copies.get(), 0, "transfer must not copy custom rows");
    assert_eq!(destroyed.get(), 0);
    assert_eq!(c.primitive_owner(a).unwrap(), before);
    assert_eq!(c.primitive_context(a).unwrap().pane, 0);
    assert_eq!(c.primitive_context(b).unwrap().pane, 1);
    assert_eq!(h.data_at(10.0.into()).unwrap().value, 2.);
    c.update_custom_data(&h, datum(HorizontalPrice::new(2047.).unwrap(), 2.5), false)
        .unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    c.swap_panes(0, 1).unwrap();
    assert_eq!(c.primitive_context(a).unwrap().pane, 1);
    assert_eq!(c.primitive_context(b).unwrap().pane, 0);
    c.remove_series(h.id()).unwrap();
    assert_eq!(destroyed.get(), 1);
}

#[test]
fn transfer_preserves_identity_and_clamps_new_target_before_cleanup() {
    let mut c = Chart::default();
    c.add_pane();
    c.add_pane();
    let a = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let b = c
        .add_series(
            SeriesOptionsMap::Histogram(Default::default()),
            1,
            PriceScalePosition::Left,
        )
        .unwrap();
    c.move_series_to_pane(a, 1).unwrap();
    assert_eq!(c.model().panes().len(), 2);
    assert_eq!(c.model().pane_for_series(a), Some(0));
    assert_eq!(c.model().pane_for_series(b), Some(0));
    c.move_series_to_pane(a, usize::MAX).unwrap();
    assert_eq!(c.model().panes().len(), 3);
    assert_eq!(c.model().pane_for_series(a), Some(2));
    c.move_series_to_pane(a, 2).unwrap();
    assert_eq!(c.model().panes().len(), 3);
}

#[test]
fn whole_panes_move_and_swap_with_series_associations() {
    let mut c = Chart::default();
    c.add_pane();
    c.add_pane();
    let ids: Vec<_> = (0..3)
        .map(|p| {
            c.add_series(
                SeriesOptionsMap::Line(Default::default()),
                p,
                PriceScalePosition::Right,
            )
            .unwrap()
        })
        .collect();
    c.set_pane_stretch_factor(0, 3.).unwrap();
    c.move_pane(0, 2).unwrap();
    assert_eq!(
        ids.iter()
            .map(|id| c.model().pane_for_series(*id).unwrap())
            .collect::<Vec<_>>(),
        [2, 0, 1]
    );
    assert_eq!(c.model().panes()[2].stretch_factor(), 3.);
    c.swap_panes(0, 2).unwrap();
    assert_eq!(
        ids.iter()
            .map(|id| c.model().pane_for_series(*id).unwrap())
            .collect::<Vec<_>>(),
        [0, 2, 1]
    );
    assert!(c.move_pane(0, 3).is_err());
    assert!(c.swap_panes(3, 0).is_err());
    assert_eq!(
        ids.iter()
            .map(|id| c.model().pane_for_series(*id).unwrap())
            .collect::<Vec<_>>(),
        [0, 2, 1]
    );
}

#[path = "numeric_custom_series/support.rs"]
mod support;
use feather_charts::{api::*, model::pane::PriceScalePosition};
use iced::Size;
use std::time::Instant;
use support::*;
fn p(v: f64) -> HorizontalPrice {
    HorizontalPrice::new(v).unwrap()
}
#[test]
fn price_custom_data_and_original_payloads() {
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let h = c
        .add_custom_series(
            View::<HorizontalPrice, HorizontalPrice>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(
        &h,
        vec![datum(p(90.), 2.), datum(p(100.), 3.), datum(p(150.), 4.)],
    )
    .unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert!(c.frame().plot.custom.is_some());
    assert_eq!(h.data_at(1.0.into()).unwrap().time, p(100.));
}

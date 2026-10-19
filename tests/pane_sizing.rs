use feather_charts::{api::*, model::time_scale_options::HorzScaleOptionsPatch};
use iced::Size;
use std::time::Instant;
fn prepare(c: &mut Chart, height: f32) {
    c.prepare(Size::new(800., height), 1., Instant::now())
        .unwrap();
}
fn chart(count: usize) -> Chart {
    let mut c = Chart::default();
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        visible: Some(false),
        ..Default::default()
    });
    for _ in 1..count {
        c.add_pane();
    }
    c.set_pane_stretch_factor(0, 2.).unwrap();
    prepare(&mut c, 500.);
    c
}
fn heights(c: &Chart) -> Vec<f64> {
    c.model().panes().iter().map(|p| p.height()).collect()
}
#[test]
fn requested_height_persists_through_prepare_resize_and_structure() {
    let mut c = chart(3);
    c.set_pane_height(1, 300.).unwrap();
    prepare(&mut c, 500.);
    assert_eq!(heights(&c)[1], 300.);
    prepare(&mut c, 500.);
    assert_eq!(heights(&c)[1], 300.);
    prepare(&mut c, 800.);
    assert!((heights(&c)[1] - 300. * 798. / 498.).abs() <= 1.);
    let factors: Vec<_> = c
        .model()
        .panes()
        .iter()
        .map(|p| p.stretch_factor())
        .collect();
    c.move_pane(1, 0).unwrap();
    prepare(&mut c, 800.);
    assert_eq!(c.model().panes()[0].stretch_factor(), factors[1]);
    c.remove_pane(2).unwrap();
    prepare(&mut c, 800.);
    assert!(heights(&c).iter().all(|h| h.is_finite() && *h > 0.));
}
#[test]
fn invalid_single_and_collapsed_layouts_preserve_state() {
    let mut c = Chart::default();
    c.set_pane_height(0, 100.).unwrap();
    c.add_pane();
    c.set_pane_height(0, 100.).unwrap();
    let factors = |c: &Chart| {
        c.model()
            .panes()
            .iter()
            .map(|p| p.stretch_factor())
            .collect::<Vec<_>>()
    };
    let saved = factors(&c);
    for h in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(c.set_pane_height(0, h).is_err());
    }
    assert!(c.set_pane_height(99, 100.).is_err());
    assert_eq!(factors(&c), saved);
    prepare(&mut c, 5.);
    c.set_pane_height(0, 100.).unwrap();
    assert_eq!(factors(&c), saved);
    prepare(&mut c, 500.);
    c.set_pane_height(0, -100.).unwrap();
    prepare(&mut c, 500.);
    assert_eq!(heights(&c)[0], 30.);
    c.set_pane_height(0, 1e6).unwrap();
    prepare(&mut c, 500.);
    assert_eq!(heights(&c)[1], 30.);
}
#[test]
fn source_pane_sizing_states() {
    let mut c = chart(3);
    let initial = heights(&c);
    c.set_pane_height(1, 300.).unwrap();
    prepare(&mut c, 500.);
    let middle = heights(&c);
    prepare(&mut c, 800.);
    let resize = heights(&c);
    c.set_pane_height(0, -100.).unwrap();
    prepare(&mut c, 800.);
    let minimum = heights(&c);
    c.set_pane_height(2, 1e6).unwrap();
    prepare(&mut c, 800.);
    let maximum = heights(&c);
    c.move_pane(2, 0).unwrap();
    prepare(&mut c, 800.);
    let moved = heights(&c);
    c.remove_pane(1).unwrap();
    prepare(&mut c, 800.);
    let removed = heights(&c);
    println!(
        "PANE_REFERENCE {{\"initial\":{initial:?},\"middle\":{middle:?},\"resize\":{resize:?},\"minimum\":{minimum:?},\"maximum\":{maximum:?},\"moved\":{moved:?},\"removed\":{removed:?}}}"
    );
}

#[test]
fn validation_and_noops_do_not_cancel_navigation_but_changes_do() {
    use std::time::Duration;
    let mut c = chart(1);
    c.scroll_to_offset_animated(-20., Duration::from_secs(2));
    // A queued animation is retained by an inert one-pane request.
    c.set_pane_height(0, 100.).unwrap();
    prepare(&mut c, 500.);
    assert!(c.is_animating());
    c.add_pane();
    prepare(&mut c, 500.);
    c.scroll_to_offset_animated(-40., Duration::from_secs(2));
    assert!(c.set_pane_height(0, f64::NAN).is_err());
    prepare(&mut c, 500.);
    assert!(c.is_animating());
    c.set_pane_height(0, 100.).unwrap();
    prepare(&mut c, 500.);
    assert!(!c.is_animating());
}

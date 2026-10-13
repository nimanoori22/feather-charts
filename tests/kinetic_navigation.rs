use feather_charts::model::{
    invalidate_mask::TimeScaleAnimation, kinetic_animation::KineticSampler,
};
use std::time::{Duration, Instant};

#[test]
fn kinetic_decay_uses_source_milliseconds_and_finishes_without_clamping_position() {
    let start = Instant::now();
    let mut sampler = KineticSampler::new(0.2, 7., 0.997, 1.).unwrap();
    sampler.add_position(0., start);
    sampler.add_position(10., start + Duration::from_millis(10));
    sampler.add_position(30., start + Duration::from_millis(30));
    let animation = sampler
        .start(30., start + Duration::from_millis(30))
        .unwrap();
    assert_eq!(
        animation.sample(start + Duration::from_millis(30)),
        (30., false)
    );
    let (position, finished) = animation.sample(start + Duration::from_millis(130));
    assert!((position - (30. + (0.997_f64.powf(100.) - 1.) / 0.997_f64.ln())).abs() < 1e-9);
    assert!(!finished);
    let (end, finished) = animation.sample(start + Duration::from_secs(10));
    assert!(finished && end > position);
    assert_eq!(animation.sample(start), (30., false));
}

#[test]
fn kinetic_ignores_slow_stale_and_invalid_gestures_but_replaces_equal_time() {
    let start = Instant::now();
    assert!(KineticSampler::new(0.2, 7., 1., 1.).is_none());
    assert!(KineticSampler::new(0.2, 7., f64::NAN, 1.).is_none());
    let mut sampler = KineticSampler::new(0.2, 7., 0.997, 3.).unwrap();
    sampler.add_position(0., start);
    sampler.add_position(2., start + Duration::from_millis(10));
    assert!(
        sampler
            .start(2., start + Duration::from_millis(10))
            .is_none()
    );
    sampler.add_position(10., start + Duration::from_millis(20));
    sampler.add_position(12., start + Duration::from_millis(20));
    sampler.add_position(f64::NAN, start + Duration::from_millis(30));
    sampler.add_position(999., start);
    let animation = sampler
        .start(12., start + Duration::from_millis(20))
        .unwrap();
    let (position, _) = animation.sample(start + Duration::from_millis(21));
    assert!((position - (12. + 0.6 * (0.997 - 1.) / 0.997_f64.ln())).abs() < 1e-8);
    assert!(
        sampler
            .start(12., start + Duration::from_millis(71))
            .is_none()
    );
    assert!(
        sampler
            .start(12., start + Duration::from_millis(70))
            .is_some()
    );
    assert!(
        sampler
            .start(12., start + Duration::from_micros(70_001))
            .is_none()
    );
}

#[test]
fn kinetic_reversal_uses_only_the_recent_direction_and_caps_speed() {
    let start = Instant::now();
    let mut sampler = KineticSampler::new(0.2, 2., 0.997, 0.).unwrap();
    for (ms, position) in [(0, 0.), (10, 100.), (20, 200.), (30, 150.)] {
        sampler.add_position(position, start + Duration::from_millis(ms));
    }
    let animation = sampler
        .start(150., start + Duration::from_millis(30))
        .unwrap();
    let (position, _) = animation.sample(start + Duration::from_millis(40));
    assert!((position - (150. - 2. * (0.997_f64.powf(10.) - 1.) / 0.997_f64.ln())).abs() < 1e-8);
}

use super::*;
use LastPriceAnimationMode::{Continuous, Disabled, OnDataUpdate};
fn info() -> Option<SeriesUpdateInfo> {
    Some(SeriesUpdateInfo {
        historical_update: false,
        last_bar_updated_or_new_bars_added_to_the_right: true,
    })
}
#[test]
fn source_samples() {
    let origin = Instant::now();
    let mut state = LastPriceAnimationState::new(origin);
    let cases = [
        0., 0.001, 325., 649.999, 650., 650.001, 1000., 1364.999, 1365., 1365.001, 2599.999, 2600.,
        2600.001, 5200.,
    ];
    for mode in [Disabled, Continuous, OnDataUpdate] {
        for ms in cases {
            let now = origin + Duration::from_secs_f64(ms / 1000.);
            let s = state.sample(mode, now);
            println!(
                "PRICE_SAMPLE {{\"mode\":\"{mode:?}\",\"ms\":{ms},\"active\":{},\"radius\":{},\"fill\":{},\"stroke\":{}}}",
                state.is_active(mode, now),
                s.radius,
                s.fill_alpha,
                s.stroke_alpha
            );
        }
    }
    state.on_update(info(), false, OnDataUpdate, origin);
    assert!(state.is_active(OnDataUpdate, origin + PERIOD));
    assert!(!state.is_active(OnDataUpdate, origin + PERIOD + Duration::from_nanos(1)));
    assert_eq!(state.sample(OnDataUpdate, origin + PERIOD).radius, 4.);
    assert_eq!(
        state
            .sample(OnDataUpdate, origin + PERIOD + Duration::from_nanos(1))
            .stroke_alpha,
        0.
    );
}
#[test]
fn source_transitions() {
    let origin = Instant::now();
    let mut state = LastPriceAnimationState::new(origin);
    for (ms, update, clear, mode) in [
        (0, true, false, OnDataUpdate),
        (100, true, false, OnDataUpdate),
        (1950, true, false, OnDataUpdate),
        (1951, true, false, OnDataUpdate),
        (2599, true, false, OnDataUpdate),
        (5199, false, false, OnDataUpdate),
        (5200, true, false, OnDataUpdate),
        (5300, true, false, Disabled),
        (5400, false, true, OnDataUpdate),
        (5500, false, false, OnDataUpdate),
        (5600, true, false, Continuous),
        (5700, false, false, OnDataUpdate),
        (6000, true, false, OnDataUpdate),
        (8600, false, false, OnDataUpdate),
        (8601, false, false, OnDataUpdate),
        (8700, true, false, OnDataUpdate),
    ] {
        let now = origin + Duration::from_millis(ms);
        state.on_update(if update { info() } else { None }, clear, mode, now);
        let s = state.sample(mode, now);
        println!(
            "PRICE_TRANSITION {{\"mode\":\"{mode:?}\",\"ms\":{ms},\"update\":{update},\"clear\":{clear},\"active\":{},\"radius\":{},\"fill\":{},\"stroke\":{}}}",
            state.is_active(mode, now),
            s.radius,
            s.fill_alpha,
            s.stroke_alpha
        );
    }
}
#[test]
fn historical_flag_and_clear_do_not_restart_continuous_origin() {
    let origin = Instant::now();
    let mut state = LastPriceAnimationState::new(origin);
    let historical = Some(SeriesUpdateInfo {
        historical_update: true,
        ..info().unwrap()
    });
    state.on_update(
        historical,
        false,
        OnDataUpdate,
        origin + Duration::from_secs(1),
    );
    assert_eq!(
        state
            .sample(OnDataUpdate, origin + Duration::from_secs(1))
            .radius,
        4.
    );
    let before = state.sample(Continuous, origin + Duration::from_secs(2));
    state.on_update(None, true, OnDataUpdate, origin + Duration::from_secs(2));
    assert!(!state.is_active(OnDataUpdate, origin + Duration::from_secs(2)));
    assert_eq!(
        state.sample(Continuous, origin + Duration::from_secs(2)),
        before
    );
}

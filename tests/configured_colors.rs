use feather_charts::{
    api::{Chart, ChartError},
    model::{
        chart_model::ChartModelOptions,
        colors::{ColorParser, Rgba},
        horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
        localization_options::LocalizationOptions,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const DARK: &str = "color(display-p3 0.2 0.3 0.4 / 0.5)";
const LIGHT: &str = "color(display-p3 0.8 0.9 1 / 0.25)";
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
#[test]
fn source_parser_matrix() {
    let calls = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut p = ColorParser::new(
        vec![
            Box::new({
                let c = calls.clone();
                move |v| {
                    c.borrow_mut().push(format!("first:{v}"));
                    None
                }
            }),
            Box::new({
                let c = calls.clone();
                move |v| {
                    c.borrow_mut().push(format!("second:{v}"));
                    match v {
                        DARK => Some(Rgba::new(20, 40, 60, 0.5)),
                        LIGHT => Some(Rgba::new(220, 230, 240, 0.25)),
                        _ => None,
                    }
                }
            }),
            Box::new({
                let c = calls.clone();
                move |v| {
                    c.borrow_mut().push(format!("third:{v}"));
                    None
                }
            }),
        ],
        None,
    );
    let gray = p.color_string_to_grayscale(DARK).unwrap();
    let contrast = p.generate_contrast_colors(DARK).unwrap();
    let alpha = p.apply_alpha(DARK, 0.4).unwrap();
    let transparent = p.apply_alpha("transparent", 0.3).unwrap();
    let gradient = p.gradient_color_at_percent(DARK, LIGHT, 0.5).unwrap();
    let light = p.generate_contrast_colors(LIGHT).unwrap();
    let css = p.generate_contrast_colors("#fff").unwrap();
    p.color_string_to_grayscale(DARK).unwrap();
    let mut failures = 0;
    for _ in 0..2 {
        if p.parse_color("color(display-p3 0 1 0)").is_err() {
            failures += 1;
        }
    }
    let contrast_json = |c: feather_charts::model::colors::ContrastColors| {
        format!(
            "{{\"foreground\":{},\"background\":{}}}",
            quote(&c.foreground),
            quote(&c.background)
        )
    };
    println!(
        "CONFIGURED_COLORS_REFERENCE {{\"gray\":{gray},\"contrast\":{},\"alpha\":{},\"transparent\":{},\"gradient\":{},\"light\":{},\"css\":{},\"failures\":{failures},\"calls\":[{}]}}",
        contrast_json(contrast),
        quote(&alpha),
        quote(&transparent),
        quote(&gradient),
        contrast_json(light),
        contrast_json(css),
        calls
            .borrow()
            .iter()
            .map(|v| quote(v))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(failures, 2);
    assert_eq!(calls.borrow().len(), 10);
}
#[test]
fn checked_chart_invokes_its_configured_parser() {
    let calls = Rc::new(Cell::new(0));
    let mut options = ChartModelOptions::default();
    options.layout.text_color = DARK.into();
    options.layout.color_parsers.push(Box::new({
        let calls = calls.clone();
        move |v| {
            calls.set(calls.get() + 1);
            (v == DARK).then_some(Rgba::new(20, 40, 60, 0.5))
        }
    }));
    let result: Result<Chart, ChartError> = Chart::with_custom_behavior(
        HorzScaleBehaviorTime::default,
        options,
        Default::default(),
        LocalizationOptions::new("en-US", "dd MMM yyyy"),
    );
    assert!(
        result.is_ok(),
        "configured parser rejected: {:?}",
        result.err()
    );
    assert!(calls.get() > 0);
}

#[test]
fn cloned_resolver_shares_cache_and_releases_callbacks_after_final_owner() {
    let calls = Rc::new(Cell::new(0));
    let lifetime = Rc::new(());
    let weak = Rc::downgrade(&lifetime);
    let mut options = ChartModelOptions::default();
    options.layout.color_parsers.push(Box::new({
        let calls = calls.clone();
        move |v| {
            let _keep = &lifetime;
            calls.set(calls.get() + 1);
            (v == DARK).then_some(Rgba::new(20, 40, 60, 0.5))
        }
    }));
    let chart = Chart::new(options);
    let mut a = chart.color_parser();
    let mut b = chart.color_parser();
    assert_eq!(a.parse_color(DARK).unwrap(), b.parse_color(DARK).unwrap());
    assert_eq!(calls.get(), 1);
    drop(chart);
    drop(a);
    assert!(weak.upgrade().is_some());
    assert!(b.parse_color(DARK).is_ok());
    assert_eq!(calls.get(), 1);
    drop(b);
    assert!(weak.upgrade().is_none());
}
#[test]
fn callback_can_reenter_a_shared_resolver_and_invalid_alpha_is_never_cached() {
    let slot = Rc::new(RefCell::new(None::<ColorParser>));
    let calls = Rc::new(Cell::new(0));
    let mut p = ColorParser::new(
        vec![Box::new({
            let slot = Rc::downgrade(&slot);
            let calls = calls.clone();
            move |_| {
                calls.set(calls.get() + 1);
                assert!(
                    slot.upgrade()
                        .unwrap()
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .parse_color("white")
                        .is_ok()
                );
                Some(Rgba::new(1, 2, 3, f32::NAN))
            }
        })],
        None,
    );
    *slot.borrow_mut() = Some(p.clone());
    assert!(p.parse_color(DARK).is_err());
    assert!(p.parse_color(DARK).is_err());
    assert_eq!(calls.get(), 2);
}

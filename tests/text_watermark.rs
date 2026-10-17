use feather_charts::{
    api::{Chart, TextWatermark, TextWatermarkLine, TextWatermarkOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{
    Size,
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
};
use std::time::Instant;
fn chart() -> Chart {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (0..32)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
                    value: 100. + (i % 7) as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c
}
fn prepare(c: &mut Chart) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
fn pixels(frame: &feather_charts::ui::chart_frame::IcedChartFrame) -> Vec<u8> {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    renderer.reset(iced::Rectangle::with_size(Size::new(800., 500.)));
    let mut canvas = iced::widget::canvas::Frame::new(&renderer, Size::new(800., 500.));
    frame.draw(&mut canvas);
    renderer.draw_geometry(canvas.into_geometry());
    renderer.screenshot(Size::new(800, 500), 1., iced::Color::BLACK)
}
#[test]
fn public_watermark_adds_visible_text() {
    let mut c = chart();
    prepare(&mut c);
    let before = pixels(c.frame());
    c.add_pane_primitive(
        0,
        TextWatermark::new(TextWatermarkOptions {
            lines: vec![TextWatermarkLine {
                text: "BTCUSD".into(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap(),
    )
    .unwrap();
    prepare(&mut c);
    assert!(
        before != pixels(c.frame()),
        "text watermark currently draws nothing"
    );
}

fn options(text: &str) -> TextWatermarkOptions {
    TextWatermarkOptions {
        lines: vec![TextWatermarkLine {
            text: text.into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}
#[test]
fn replacements_visibility_removal_and_retained_frames_preserve_chart_state() {
    let mut c = chart();
    prepare(&mut c);
    prepare(&mut c);
    let baseline = pixels(c.frame());
    let layout = c.frame().layout;
    let range = format!(
        "{:?}",
        c.model().panes()[0].default_price_scale().price_range()
    );
    let id = c
        .add_pane_primitive(0, TextWatermark::new(options("BTCUSD")).unwrap())
        .unwrap();
    prepare(&mut c);
    let retained = c.frame().clone();
    let first = pixels(&retained);
    assert!(!c.is_animating());
    assert_eq!(layout, c.frame().layout);
    assert_eq!(
        range,
        format!(
            "{:?}",
            c.model().panes()[0].default_price_scale().price_range()
        )
    );
    let mut changed = options("ETHUSD");
    changed.lines[0].font = iced::Font {
        family: iced::font::Family::Monospace,
        weight: iced::font::Weight::Bold,
        style: iced::font::Style::Italic,
        ..Default::default()
    };
    c.replace_primitive(id, TextWatermark::new(changed.clone()).unwrap())
        .unwrap();
    prepare(&mut c);
    assert!(first != pixels(c.frame()));
    assert_eq!(first, pixels(&retained));
    changed.visible = false;
    c.replace_primitive(id, TextWatermark::new(changed).unwrap())
        .unwrap();
    prepare(&mut c);
    assert_eq!(baseline, pixels(c.frame()));
    c.remove_primitive(id).unwrap();
    prepare(&mut c);
    assert_eq!(baseline, pixels(c.frame()));
    assert_eq!(first, pixels(&retained));
    assert!(c.remove_primitive(id).is_err());
}
#[test]
fn invalid_options_do_not_replace_a_working_attachment() {
    let mut c = chart();
    let id = c
        .add_pane_primitive(0, TextWatermark::new(options("BTCUSD")).unwrap())
        .unwrap();
    prepare(&mut c);
    let before = pixels(c.frame());
    for (size, height, color) in [
        (0., None, "red"),
        (-1., None, "red"),
        (f32::NAN, None, "red"),
        (48., Some(-1.), "red"),
        (48., Some(f32::INFINITY), "red"),
        (48., None, "invalid"),
    ] {
        let mut bad = options("bad");
        bad.lines[0].font_size = size;
        bad.lines[0].line_height = height;
        bad.lines[0].color = color.into();
        assert!(TextWatermark::new(bad).is_err());
    }
    assert!(c.primitive_owner(id).is_ok());
    prepare(&mut c);
    assert_eq!(before, pixels(c.frame()));
    assert!(
        c.add_pane_primitive(9, TextWatermark::new(options("bad pane")).unwrap())
            .is_err()
    );
}
#[test]
fn pane_identity_survives_index_shifts_and_preserved_empty_panes() {
    use feather_charts::api::PrimitiveOwner;
    let mut c = chart();
    let first = c.series_ids().next().unwrap();
    let second = c.add_pane();
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            second,
            PriceScalePosition::Right,
        )
        .unwrap();
    let id = c
        .add_pane_primitive(second, TextWatermark::new(options("Volume pane")).unwrap())
        .unwrap();
    let owner = c.primitive_owner(id).unwrap();
    assert!(matches!(owner, PrimitiveOwner::Pane(_)));
    c.remove_series(first).unwrap();
    prepare(&mut c);
    assert_eq!(c.primitive_owner(id).unwrap(), owner);
    assert!(c.frame().plot.primitives.iter().any(|a| a.id == id));
    c.set_preserve_empty_pane(0, true).unwrap();
    c.remove_series(peer).unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.primitives.iter().any(|a| a.id == id));
    assert!(pixels(c.frame()).iter().any(|v| *v != 255));
    c.set_preserve_empty_pane(0, false).unwrap();
    // The final pane remains by model policy; explicit primitive removal is still valid.
    let empty = pixels(c.frame());
    c.remove_primitive(id).unwrap();
    prepare(&mut c);
    assert!(
        empty != pixels(c.frame()),
        "preserved empty pane must actually draw its watermark"
    );
    assert!(c.primitive_owner(id).is_err());
}
#[derive(Debug)]
struct Cover;
impl feather_charts::api::DrawingPrimitive for Cover {
    fn prepare(
        &self,
        context: &feather_charts::api::PrimitiveContext<'_>,
    ) -> feather_charts::api::PreparedPrimitive {
        #[derive(Debug)]
        struct Rectangle(Size);
        impl feather_charts::api::PrimitiveRenderer for Rectangle {
            fn draw(&self, f: &mut iced::widget::canvas::Frame) {
                f.fill_rectangle(iced::Point::ORIGIN, self.0, iced::Color::WHITE);
            }
        }
        feather_charts::api::PreparedPrimitive {
            views: vec![feather_charts::api::PrimitiveView::new(
                feather_charts::api::PrimitiveLayer::Normal,
                Rectangle(context.size),
            )],
            ..Default::default()
        }
    }
}
#[test]
fn watermark_normal_layer_precedes_series_primitives_and_has_no_hit() {
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let cover = c.add_series_primitive(series, Cover).unwrap();
    prepare(&mut c);
    let reference = pixels(c.frame());
    let point = iced::Point::new(100., 100.);
    let hit = c.hit_test(0, point);
    let watermark = c
        .add_pane_primitive(0, TextWatermark::new(options("BTCUSD")).unwrap())
        .unwrap();
    prepare(&mut c);
    assert_eq!(reference, pixels(c.frame()));
    assert_eq!(hit, c.hit_test(0, point));
    c.remove_primitive(cover).unwrap();
    prepare(&mut c);
    assert!(reference != pixels(c.frame()));
    assert!(
        c.frame()
            .plot
            .primitives
            .iter()
            .find(|a| a.id == watermark)
            .unwrap()
            .output
            .hit_tester
            .is_none()
    );
}

#[test]
fn animation_only_ticks_do_not_prepare_the_watermark() {
    use feather_charts::{
        api::{ChartMessage, DrawingPrimitive, PreparedPrimitive, PrimitiveContext},
        model::series_options::{LastPriceAnimationMode, LineSeriesOptions},
    };
    use std::{cell::Cell, rc::Rc, time::Duration};
    #[derive(Debug)]
    struct Observed {
        value: TextWatermark,
        calls: Rc<Cell<usize>>,
    }
    impl DrawingPrimitive for Observed {
        fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
            self.calls.set(self.calls.get() + 1);
            self.value.prepare(c)
        }
    }
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let mut o = LineSeriesOptions::default();
    o.style.last_price_animation = LastPriceAnimationMode::Continuous;
    c.replace_series_options(series, SeriesOptionsMap::Line(o))
        .unwrap();
    let calls = Rc::new(Cell::new(0));
    c.add_pane_primitive(
        0,
        Observed {
            value: TextWatermark::new(options("BTCUSD")).unwrap(),
            calls: calls.clone(),
        },
    )
    .unwrap();
    prepare(&mut c);
    let now = Instant::now();
    c.update(ChartMessage::Tick(now)).unwrap();
    let before = calls.get();
    for i in 1..100 {
        c.update(ChartMessage::Tick(now + Duration::from_millis(i * 16)))
            .unwrap();
    }
    assert_eq!(calls.get(), before);
}
#[test]
fn actual_pane_removal_drops_its_watermark() {
    let mut c = chart();
    let pane = c.add_pane();
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    let watermark = c
        .add_pane_primitive(pane, TextWatermark::new(options("Remove me")).unwrap())
        .unwrap();
    c.remove_series(peer).unwrap();
    prepare(&mut c);
    assert!(c.primitive_owner(watermark).is_err());
}

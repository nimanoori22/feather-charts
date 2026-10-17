use feather_charts::{
    api::{Chart, ImageWatermark, ImageWatermarkOptions},
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
fn public_image_watermark_draws_visible_pixels() {
    let mut c = chart();
    prepare(&mut c);
    let before = pixels(c.frame());
    c.add_pane_primitive(
        0,
        ImageWatermark::from_rgba(
            2,
            1,
            vec![255, 0, 0, 255, 0, 0, 255, 128],
            ImageWatermarkOptions::default(),
        )
        .unwrap(),
    )
    .unwrap();
    prepare(&mut c);
    assert!(
        before != pixels(c.frame()),
        "image watermark must draw visible pixels"
    );
}

fn image(color: [u8; 4]) -> ImageWatermark {
    ImageWatermark::from_rgba(1, 1, color.to_vec(), ImageWatermarkOptions::default()).unwrap()
}
#[test]
fn replacements_opacity_and_removal_keep_old_frames_and_chart_state() {
    let mut c = chart();
    prepare(&mut c);
    let baseline = pixels(c.frame());
    let layout = c.frame().layout;
    let hit = c.hit_test(0, iced::Point::new(100., 100.));
    let original = image([255, 0, 0, 255]);
    let id = c.add_pane_primitive(0, original.clone()).unwrap();
    prepare(&mut c);
    let retained = c.frame().clone();
    let red = pixels(&retained);
    assert!(red != baseline);
    assert_eq!(c.frame().layout, layout);
    assert_eq!(c.hit_test(0, iced::Point::new(100., 100.)), hit);
    assert!(!c.is_animating());
    c.replace_primitive(
        id,
        original
            .with_options(ImageWatermarkOptions {
                alpha: 0.,
                ..Default::default()
            })
            .unwrap(),
    )
    .unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) == baseline);
    c.replace_primitive(id, image([0, 0, 255, 128])).unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) != red);
    assert!(pixels(&retained) == red);
    c.remove_primitive(id).unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) == baseline);
    assert!(pixels(&retained) == red);
    assert!(c.remove_primitive(id).is_err());
}
#[test]
fn invalid_inputs_cannot_replace_working_image() {
    use feather_charts::api::ImageWatermarkError as E;
    assert_eq!(
        ImageWatermark::from_rgba(0, 1, vec![], Default::default()).unwrap_err(),
        E::InvalidDimensions
    );
    assert!(ImageWatermark::from_rgba(u32::MAX, u32::MAX, vec![], Default::default()).is_err());
    assert_eq!(
        ImageWatermark::from_rgba(2, 2, vec![0; 15], Default::default()).unwrap_err(),
        E::InvalidBufferLength
    );
    let original = image([255, 0, 0, 255]);
    let mut c = chart();
    let id = c.add_pane_primitive(0, original.clone()).unwrap();
    prepare(&mut c);
    let before = pixels(c.frame());
    for o in [
        ImageWatermarkOptions {
            alpha: f32::NAN,
            ..Default::default()
        },
        ImageWatermarkOptions {
            alpha: 1.1,
            ..Default::default()
        },
        ImageWatermarkOptions {
            padding: -1.,
            ..Default::default()
        },
        ImageWatermarkOptions {
            max_width: Some(f32::INFINITY),
            ..Default::default()
        },
        ImageWatermarkOptions {
            max_height: Some(-1.),
            ..Default::default()
        },
    ] {
        assert!(original.with_options(o).is_err());
    }
    assert!(c.primitive_owner(id).is_ok());
    prepare(&mut c);
    assert!(before == pixels(c.frame()));
    assert!(c.add_pane_primitive(9, original).is_err());
}
#[test]
fn multiple_images_follow_pane_identity_and_empty_pane_lifecycle() {
    let mut c = chart();
    let first = c.series_ids().next().unwrap();
    let pane = c.add_pane();
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    let red = c.add_pane_primitive(pane, image([255, 0, 0, 255])).unwrap();
    let blue = c.add_pane_primitive(pane, image([0, 0, 255, 255])).unwrap();
    let owner = c.primitive_owner(blue).unwrap();
    c.remove_series(first).unwrap();
    c.set_preserve_empty_pane(0, true).unwrap();
    c.remove_series(peer).unwrap();
    prepare(&mut c);
    assert_eq!(c.primitive_owner(blue).unwrap(), owner);
    let retained = c.frame().clone();
    let blue_pixels = pixels(&retained);
    c.remove_primitive(blue).unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) != blue_pixels);
    c.remove_primitive(red).unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) != blue_pixels);
    assert!(pixels(&retained) == blue_pixels);
    let pane = c.add_pane();
    let series = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    let id = c.add_pane_primitive(pane, image([255, 0, 0, 255])).unwrap();
    c.remove_series(series).unwrap();
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
        struct Fill(Size);
        impl feather_charts::api::PrimitiveRenderer for Fill {
            fn draw(&self, f: &mut iced::widget::canvas::Frame) {
                f.fill_rectangle(iced::Point::ORIGIN, self.0, iced::Color::WHITE);
            }
        }
        feather_charts::api::PreparedPrimitive {
            views: vec![feather_charts::api::PrimitiveView::new(
                feather_charts::api::PrimitiveLayer::Normal,
                Fill(context.size),
            )],
            ..Default::default()
        }
    }
}
#[test]
fn bottom_image_does_not_cover_later_vectors_or_text_watermark() {
    use feather_charts::api::{TextWatermark, TextWatermarkLine, TextWatermarkOptions};
    let mut c = chart();
    c.add_pane_primitive(0, Cover).unwrap();
    prepare(&mut c);
    let covered = pixels(c.frame());
    c.add_pane_primitive(0, image([255, 0, 0, 255])).unwrap();
    prepare(&mut c);
    assert!(
        pixels(c.frame()) == covered,
        "normal cover must occlude bottom image"
    );
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
    assert!(pixels(c.frame()) != covered);
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
        value: ImageWatermark,
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
            value: image([255, 0, 0, 255]),
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

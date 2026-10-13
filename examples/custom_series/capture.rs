use super::{App, Message};
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
use iced::{Color, Size, widget::canvas::Frame};
use std::{io::Write, path::Path, time::Instant};

pub fn run(directory: &str, wgpu: bool) {
    std::fs::create_dir_all(directory).unwrap();
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    let states = [
        "normal",
        "hover",
        "append",
        "replace",
        "historical",
        "whitespace",
        "style",
        "empty",
        "reload",
        "removed",
        "readded",
        "narrow",
        "zoom",
        "inverted",
        "log",
        "percentage",
        "indexed",
        "hidden",
        "left",
        "multi-pane",
        "manual",
        "pan",
        "markers",
    ];
    for ratio in [1., 1.25, 1.5, 2.] {
        for state in states {
            let mut app = App {
                size: if state == "narrow" {
                    Size::new(420., 320.)
                } else {
                    Size::new(900., 600.)
                },
                ratio,
                ..Default::default()
            };
            app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
            match state {
                "left" | "multi-pane" => {
                    app.apply(Message::Remove);
                    let pane = if state == "multi-pane" {
                        app.chart.add_pane()
                    } else {
                        0
                    };
                    let side = if state == "left" {
                        feather_charts::model::pane::PriceScalePosition::Left
                    } else {
                        feather_charts::model::pane::PriceScalePosition::Right
                    };
                    let handle = app
                        .chart
                        .add_custom_series(
                            super::View::default(),
                            feather_charts::model::series_options::CustomSeriesOptions::default(),
                            pane,
                            side,
                        )
                        .unwrap();
                    app.chart
                        .set_custom_data(&handle, (0..120).map(super::sample).collect())
                        .unwrap();
                    app.custom = Some(handle);
                }
                "pan" => app.chart.set_right_offset(-30.),
                "manual" => {
                    use feather_charts::ui::chart_input::{ChartInteraction, Navigation};
                    for (start, position) in [
                        (
                            Some(iced::Point::new(300., 200.)),
                            iced::Point::new(300., 200.),
                        ),
                        (None, iced::Point::new(340., 240.)),
                    ] {
                        app.chart
                            .update(feather_charts::api::ChartMessage::Interaction(
                                ChartInteraction {
                                    position: Some(position),
                                    navigation: Some(Navigation::Pan { start, position }),
                                    capture: true,
                                },
                            ))
                            .unwrap();
                    }
                    assert!(
                        !app.chart.model().panes()[0]
                            .right_price_scale()
                            .options()
                            .auto_scale
                    );
                }
                "markers" => {
                    let feather_charts::model::icustom_series::CustomSeriesDataItem::Data(data) =
                        super::sample(60)
                    else {
                        unreachable!()
                    };
                    app.chart
                        .add_series_markers(
                            app.custom.as_ref().unwrap().id(),
                            vec![feather_charts::api::SeriesMarker::new(
                                data.time,
                                feather_charts::api::MarkerPosition::AboveBar,
                                feather_charts::api::MarkerShape::Circle,
                                "yellow",
                            )],
                            Default::default(),
                        )
                        .unwrap();
                }
                "append" => app.apply(Message::Append),
                "replace" => app.apply(Message::Replace),
                "historical" => app.apply(Message::Historical),
                "whitespace" => app.apply(Message::Whitespace),
                "style" => app.apply(Message::Style),
                "empty" => app.apply(Message::Clear),
                "reload" => {
                    app.apply(Message::Clear);
                    app.apply(Message::Reload);
                }
                "removed" => app.apply(Message::Remove),
                "readded" => {
                    app.apply(Message::Remove);
                    app.apply(Message::Reload);
                }
                "zoom" => {
                    app.chart
                        .set_logical_range(feather_charts::model::time_data::LogicalRange {
                            from: 35.0.into(),
                            to: 65.0.into(),
                        })
                }
                "inverted" | "log" | "percentage" | "indexed" => {
                    use feather_charts::model::price_scale::{
                        PriceScaleMode, PriceScaleOptionsPatch,
                    };
                    app.chart
                        .apply_price_axis_options(
                            0,
                            feather_charts::model::axis_snapshots::PriceAxisSide::Right,
                            PriceScaleOptionsPatch {
                                invert_scale: Some(state == "inverted"),
                                mode: Some(match state {
                                    "log" => PriceScaleMode::Logarithmic,
                                    "percentage" => PriceScaleMode::Percentage,
                                    "indexed" => PriceScaleMode::IndexedTo100,
                                    _ => PriceScaleMode::Normal,
                                }),
                                ..Default::default()
                            },
                        )
                        .unwrap();
                }
                "hidden" => {
                    let mut options =
                        feather_charts::model::series_options::CustomSeriesOptions::default();
                    options.common.visible = false;
                    app.chart
                        .replace_custom_series_options(app.custom.as_ref().unwrap(), options)
                        .unwrap();
                }
                _ => {}
            }
            app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
            if state == "hover" {
                app.chart
                    .update(feather_charts::api::ChartMessage::Click {
                        pane: 0,
                        position: iced::Point::new(400., 240.),
                    })
                    .unwrap();
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
            }
            let mut frame = Frame::new(&renderer, app.size);
            if let Some(multi) = app.chart.multi_frame() {
                multi.draw(&mut frame);
            } else {
                app.chart.frame().draw(&mut frame);
            }
            renderer.reset(iced::Rectangle::with_size(app.size));
            renderer.draw_geometry(frame.into_geometry());
            let physical = Size::new(
                (app.size.width * ratio) as u32,
                (app.size.height * ratio) as u32,
            );
            let bytes = renderer.screenshot(physical, ratio, Color::BLACK);
            assert_eq!(
                bytes,
                renderer.screenshot(physical, ratio, Color::BLACK),
                "{state}: unstable retained draw"
            );
            assert!(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[0] != pixel[1] || pixel[1] != pixel[2]),
                "{state}: scene is blank"
            );
            let has_boxes = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] == [233, 30, 99]);
            assert_eq!(
                has_boxes,
                !matches!(state, "empty" | "removed" | "hidden"),
                "{state}: custom boxes missing or stale"
            );
            let mut file =
                std::fs::File::create(Path::new(directory).join(format!("{state}-{ratio}.ppm")))
                    .unwrap();
            write!(file, "P6\n{} {}\n255\n", physical.width, physical.height).unwrap();
            let mut rgb = Vec::with_capacity(bytes.len() / 4 * 3);
            for pixel in bytes.as_chunks::<4>().0 {
                rgb.extend_from_slice(&pixel[..3]);
            }
            file.write_all(&rgb).unwrap();
        }
    }
    println!(
        "{} custom-series captures passed on {backend}",
        states.len() * 4
    );
}

//! Explicit managed updates and a frozen-clock native capture fixture.
use super::*;
use feather_charts::api::{MarkerSign, UpDownMarker, UpDownMarkersId, UpDownMarkersOptions};
use std::time::Duration;
fn time(index: usize) -> Time {
    UtcTimestamp::new(1_700_000_000. + index as f64 * 60.).into()
}
fn value(index: usize) -> f64 {
    100. + (index as f64 * 0.1).sin() * 8.
}
pub(super) struct Scene {
    pub id: UpDownMarkersId,
    kind: SeriesType,
    count: usize,
    step: usize,
    options: UpDownMarkersOptions,
}
impl Scene {
    pub(super) fn new(
        chart: &mut Chart,
        series: feather_charts::model::data_layer::SeriesId,
        kind: SeriesType,
        count: usize,
    ) -> Self {
        let options = UpDownMarkersOptions::default();
        let id = chart.add_up_down_markers(series, options.clone()).unwrap();
        chart
            .set_up_down_data(
                id,
                (0..count)
                    .map(|i| sample_row(kind, time(i), value(i)))
                    .collect(),
            )
            .unwrap();
        Self {
            id,
            kind,
            count,
            step: 0,
            options,
        }
    }
    pub(super) fn lifetime_label(&self) -> String {
        format!(
            "Lifetime: {} ms",
            self.options.update_visibility_duration.as_millis()
        )
    }
    pub(super) fn change(
        &mut self,
        chart: &mut Chart,
        message: &Message,
        latest: usize,
        now: Instant,
    ) -> Result<(), feather_charts::api::ChartError> {
        match message {
            Message::UpDownLatest | Message::UpDownHistorical => {
                let i = if matches!(message, Message::UpDownLatest) {
                    self.count - 1
                } else {
                    self.count - 12 - (self.step % 3) * 4
                };
                let delta = [2., 0., -2.][self.step % 3];
                self.step += 1;
                chart.update_up_down_data(
                    self.id,
                    sample_row(self.kind, time(i), value(i) + delta),
                    i != latest,
                    now,
                )?;
            }
            Message::UpDownManual => {
                self.manual(chart);
            }
            Message::UpDownClear => chart.clear_up_down_markers(self.id).unwrap(),
            Message::UpDownLifetime => {
                self.options.update_visibility_duration =
                    match self.options.update_visibility_duration.as_millis() {
                        5000 => Duration::from_millis(500),
                        500 => Duration::ZERO,
                        _ => Duration::from_secs(5),
                    };
                chart
                    .replace_up_down_marker_options(self.id, self.options.clone())
                    .unwrap();
            }
            Message::UpDownColors => {
                let alternate = self.options.positive_color == "#22AB94";
                self.options.positive_color = if alternate { "purple" } else { "#22AB94" }.into();
                self.options.negative_color = if alternate { "orange" } else { "#F7525F" }.into();
                chart
                    .replace_up_down_marker_options(self.id, self.options.clone())
                    .unwrap();
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    fn manual(&self, chart: &mut Chart) {
        chart
            .set_up_down_markers(
                self.id,
                [
                    (self.count - 20, MarkerSign::Positive),
                    (self.count - 10, MarkerSign::Neutral),
                    (self.count - 1, MarkerSign::Negative),
                ]
                .into_iter()
                .map(|(i, sign)| UpDownMarker {
                    time: time(i),
                    value: value(i),
                    sign,
                })
                .collect(),
            )
            .unwrap();
    }
}
pub(super) fn capture(directory: &std::path::Path, wgpu: bool) {
    use iced::advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    };
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    assert!(
        matches!(
            (&renderer, wgpu),
            (iced::Renderer::Primary(_), true) | (iced::Renderer::Secondary(_), false)
        ),
        "backend fallback"
    );
    std::fs::create_dir_all(directory).unwrap();
    let count = if std::env::args().any(|v| v == "--conflation") {
        8192
    } else {
        120
    };
    for (width, height) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            let size = Size::new(width, height);
            for scene in [
                "default",
                "directions",
                "historical",
                "refreshed",
                "mixed",
                "expired",
            ] {
                let kind = if scene == "historical" {
                    SeriesType::Area
                } else {
                    SeriesType::Line
                };
                let mut app = App {
                    size,
                    ratio,
                    ..App::new(kind)
                };
                let now = Instant::now();
                let mut at = now;
                let mut id = None;
                if scene != "default" {
                    let mut markers = Scene::new(&mut app.chart, app.price, kind, count);
                    id = Some(markers.id);
                    match scene {
                        "directions" => markers.manual(&mut app.chart),
                        "historical" => {
                            app.chart.apply_time_scale_options(
                                feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                                    enable_conflation: Some(false),
                                    ..Default::default()
                                },
                            );
                            for _ in 0..3 {
                                markers
                                    .change(
                                        &mut app.chart,
                                        &Message::UpDownHistorical,
                                        count - 1,
                                        now,
                                    )
                                    .unwrap();
                            }
                        }
                        "refreshed" => {
                            markers
                                .change(&mut app.chart, &Message::UpDownLatest, count - 1, now)
                                .unwrap();
                            at = now + Duration::from_secs(4);
                            markers
                                .change(&mut app.chart, &Message::UpDownLatest, count - 1, at)
                                .unwrap();
                            at = now + Duration::from_secs(5);
                        }
                        "mixed" => {
                            markers.manual(&mut app.chart);
                            hover_demo::add_overlays(&mut app.chart);
                            app.primitive_scene =
                                Some(primitive_demo::Scene::new(&mut app.chart, app.price));
                            app.watermark = Some(watermark_demo::Scene::new(&mut app.chart));
                            app.image_watermark =
                                Some(image_watermark_demo::Scene::new(&mut app.chart));
                            app.chart
                                .add_series_markers(
                                    app.price,
                                    sample_markers(false),
                                    SeriesMarkersOptions::default(),
                                )
                                .unwrap();
                        }
                        "expired" => {
                            markers
                                .change(&mut app.chart, &Message::UpDownLatest, count - 1, now)
                                .unwrap();
                            at = now + Duration::from_secs(5);
                        }
                        _ => unreachable!(),
                    }
                }
                if scene != "default" && scene != "mixed" {
                    app.chart
                        .set_logical_range(feather_charts::model::time_data::LogicalRange {
                            from: ((count - 35) as f64).into(),
                            to: ((count + 5) as f64).into(),
                        });
                }
                app.chart.prepare(size, ratio, at).unwrap();
                let retained = app.chart.multi_frame().unwrap().clone();
                let mut draw =
                    |snapshot: &feather_charts::ui::multi_pane_frame::IcedMultiPaneChartFrame| {
                        renderer.reset(iced::Rectangle::with_size(size));
                        let mut frame = iced::widget::canvas::Frame::new(&renderer, size);
                        snapshot.draw(&mut frame);
                        renderer.draw_geometry(frame.into_geometry());
                        renderer.screenshot(
                            Size::new((width * ratio) as u32, (height * ratio) as u32),
                            ratio,
                            iced::Color::BLACK,
                        )
                    };
                let rgba = draw(&retained);
                assert!(rgba == draw(&retained), "frozen frame changed pixels");
                if let Some(id) = id {
                    app.chart.clear_up_down_markers(id).unwrap();
                    app.chart.prepare(size, ratio, at).unwrap();
                    let clear = draw(app.chart.multi_frame().unwrap());
                    if scene == "expired" {
                        assert!(rgba == clear, "expired marker remained visible");
                    } else if retained.panes[0].plot.size.height > 30. {
                        assert!(
                            rgba != clear,
                            "markers drew nothing: {backend} {scene} {width} {height} {ratio}"
                        );
                    }
                    assert!(rgba == draw(&retained), "old frame changed after clear");
                }
                std::fs::write(
                    directory.join(format!("{scene}-{width}x{height}-{ratio}.rgba")),
                    rgba,
                )
                .unwrap();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_and_idle_expiration_work_on_line_and_area_after_short_height_recovery() {
        for kind in [SeriesType::Line, SeriesType::Area] {
            let mut app = App::new(kind);
            let mut scene = Scene::new(&mut app.chart, app.price, kind, 120);
            let now = Instant::now();
            for message in [
                Message::UpDownLatest,
                Message::UpDownHistorical,
                Message::UpDownManual,
                Message::UpDownColors,
                Message::UpDownLifetime,
                Message::UpDownClear,
            ] {
                scene.change(&mut app.chart, &message, 119, now).unwrap();
                for size in [Size::new(300., 90.), Size::new(800., 500.)] {
                    app.chart.prepare(size, 1.25, now).unwrap();
                }
            }
            scene
                .change(&mut app.chart, &Message::UpDownLatest, 119, now)
                .unwrap();
            app.chart
                .prepare(Size::new(800., 500.), 1., now + Duration::from_secs(10))
                .unwrap();
            assert!(
                app.chart
                    .up_down_markers(scene.id, now + Duration::from_secs(10))
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

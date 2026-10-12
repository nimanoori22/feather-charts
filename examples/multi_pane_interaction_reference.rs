use feather_charts::{
    model::{
        crosshair::{CrosshairOptions, CrosshairPosition},
        time_data::TimePointIndex,
    },
    renderers::{
        crosshair_renderer, grid_renderer::PixelRatio, line_renderer::PathOperation,
        price_axis_renderer::AxisBounds,
    },
    ui::{
        chart_input::{MultiPaneInput, MultiPaneInteraction},
        chart_layout::AxisSizeRequests,
        multi_pane_layout::allocate_multi_pane_layout,
    },
};
fn main() {
    let mut cases = vec![];
    for ratio in [1., 1.25, 1.5, 2.] {
        for active in 0..2 {
            for pane in 0..2 {
                let result = crosshair_renderer::draw_commands(
                    CrosshairPosition {
                        pane: active,
                        index: TimePointIndex::new(1.),
                        x: 20.2,
                        y: (active == pane).then_some(30.8),
                    },
                    &CrosshairOptions::default(),
                    AxisBounds {
                        width: 100.,
                        height: 60.,
                    },
                    PixelRatio {
                        horizontal: ratio,
                        vertical: ratio,
                    },
                );
                let paths = result
                    .iter()
                    .map(|s| match (&s.path[0], &s.path[1]) {
                        (PathOperation::MoveTo(a), PathOperation::LineTo(b)) => {
                            format!("[{},{},{},{},{}]", a.x, a.y, b.x, b.y, s.width)
                        }
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                cases.push(format!("{{\"kind\":\"crosshair\",\"ratio\":{ratio},\"active\":{active},\"pane\":{pane},\"result\":[{paths}]}}"));
            }
        }
        for height in [50., 100., 500.] {
            for delta in [-1000., -25., 0., 25., 1000.] {
                let layout = allocate_multi_pane_layout(
                    AxisBounds {
                        width: 800.,
                        height,
                    },
                    AxisSizeRequests::default(),
                    PixelRatio {
                        horizontal: ratio,
                        vertical: ratio,
                    },
                    &[2., 1.],
                )
                .unwrap();
                let mut input = MultiPaneInput::default();
                let point = iced::Point::new(100., layout.separators[0].origin.y as f32);
                input.update(
                    &iced::widget::canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(
                        iced::mouse::Button::Left,
                    )),
                    Some(point),
                    &layout,
                    &[2., 1.],
                    true,
                    true,
                );
                let action = input.update(
                    &iced::widget::canvas::Event::Mouse(iced::mouse::Event::CursorMoved {
                        position: point,
                    }),
                    Some(iced::Point::new(point.x, point.y + delta)),
                    &layout,
                    &[2., 1.],
                    true,
                    true,
                );
                let result = match action {
                    Some(MultiPaneInteraction::Resize {
                        upper_stretch,
                        lower_stretch,
                        ..
                    }) => format!("[{upper_stretch},{lower_stretch}]"),
                    _ => "[2,1]".into(),
                };
                let upper = layout.panes[0].plot.size.height;
                let lower = layout.panes[1].plot.size.height;
                // Record the actual f32 pointer delta supplied to the adapter.
                let delta = f64::from((point.y + delta) - point.y);
                cases.push(format!("{{\"kind\":\"divider\",\"upperHeight\":{upper},\"lowerHeight\":{lower},\"delta\":{delta},\"result\":{result}}}"));
            }
        }
    }
    for precision in [0, 2, 4] {
        for value in [
            0.,
            100.,
            994.4,
            995.,
            1000.,
            40_000.,
            -30_000_000.,
            999_995.,
            999_999_995.,
            0.0001,
            1_000_000_000.,
            10_000_000_000.,
        ] {
            let result =
                feather_charts::formatters::volume_formatter::VolumeFormatter::new(precision)
                    .format(value);
            cases.push(format!("{{\"kind\":\"volume\",\"precision\":{precision},\"value\":{value},\"result\":{result:?}}}"));
        }
    }
    println!("[{}]", cases.join(","));
}

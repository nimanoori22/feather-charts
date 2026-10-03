//! Emits deterministic command cases for scripts/check_line_reference.cjs.
use feather_charts::{
    model::time_data::ValueRange,
    renderers::{
        draw_line::{LineStyle, LineType, LineWidth},
        grid_renderer::PixelRatio,
        line_renderer::{LinePoint, LineRendererData, PathOperation, Point},
    },
    ui::line_chart::{FrameLayout, PlotSnapshot},
};
fn p(point: Point) -> String {
    format!("{},{}", point.x, point.y)
}
fn path_json(path: &[PathOperation]) -> String {
    path.iter()
        .map(|op| match *op {
            PathOperation::MoveTo(point) => format!("[\"M\",{}]", p(point)),
            PathOperation::LineTo(point) => format!("[\"L\",{}]", p(point)),
            PathOperation::CubicTo {
                control1,
                control2,
                end,
            } => format!("[\"C\",{},{},{}]", p(control1), p(control2), p(end)),
        })
        .collect::<Vec<_>>()
        .join(",")
}
fn main() {
    let points = [
        (10., 50., "red"),
        (30., 20., "blue"),
        (40., 60., "blue"),
        (70., 10., "red"),
        (90., 40., "green"),
    ];
    let mut cases = vec![];
    for (kind_index, kind) in [LineType::Simple, LineType::WithSteps, LineType::Curved]
        .into_iter()
        .enumerate()
    {
        for style in [
            LineStyle::Solid,
            LineStyle::Dotted,
            LineStyle::Dashed,
            LineStyle::LargeDashed,
            LineStyle::SparseDotted,
        ] {
            for ratio in [1., 1.25, 1.5, 2.] {
                for range in [
                    ValueRange { from: 0, to: 5 },
                    ValueRange { from: 1, to: 4 },
                    ValueRange { from: 2, to: 3 },
                ] {
                    let items = points
                        .iter()
                        .enumerate()
                        .map(|(i, &(x, y, color))| LinePoint {
                            index: (i as f64).into(),
                            x,
                            y,
                            color: color.into(),
                        })
                        .collect();
                    let data = LineRendererData {
                        items,
                        visible_range: range,
                        bar_width: 6.,
                        line_type: Some(kind),
                        line_width: LineWidth::Three,
                        line_style: style,
                        point_markers_radius: Some(3.5),
                    };
                    let commands = data.draw_commands(PixelRatio {
                        horizontal: ratio,
                        vertical: ratio,
                    });
                    let strokes=commands.strokes.iter().map(|stroke|{
                        let path=path_json(&stroke.path);
                        format!("{{\"color\":{:?},\"width\":{},\"dash\":{:?},\"offset\":{},\"path\":[{}]}}",stroke.color,stroke.width,stroke.dash_pattern,stroke.dash_offset,path)
                    }).collect::<Vec<_>>().join(",");
                    let markers = commands
                        .markers
                        .iter()
                        .map(|m| {
                            format!(
                                "{{\"color\":{:?},\"center\":[{}],\"radius\":{}}}",
                                m.color,
                                p(m.center),
                                m.radius
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let layout = FrameLayout {
                        pane: 0,
                        size: iced::Size::new(120., 80.),
                        pixel_ratio: PixelRatio {
                            horizontal: ratio,
                            vertical: ratio,
                        },
                        background: "white".into(),
                    };
                    let snapshot =
                        PlotSnapshot::from_commands(&layout, vec![], commands.clone()).unwrap();
                    let adapter = snapshot
                        .lines
                        .iter()
                        .zip(&commands.strokes)
                        .map(|(line, source)| {
                            format!(
                                "{{\"color\":{:?},\"width\":{},\"path\":[{}]}}",
                                source.color,
                                line.width,
                                path_json(&line.path)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    cases.push(format!("{{\"kind\":{kind_index},\"style\":{},\"ratio\":{ratio},\"from\":{},\"to\":{},\"strokes\":[{strokes}],\"markers\":[{markers}],\"adapter\":[{adapter}]}}",style as u8,range.from,range.to));
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}

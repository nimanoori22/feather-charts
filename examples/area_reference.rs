//! Source differential fixture for independent Area fill styles.
use feather_charts::{
    model::time_data::ValueRange,
    renderers::{
        area_renderer::AreaRendererData,
        draw_line::{LineStyle, LineType, LineWidth},
        grid_renderer::PixelRatio,
        line_renderer::{LinePoint, LineRendererData, PathOperation, Point},
    },
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
    let points = [(10., 50.), (30., 20.), (40., 60.), (70., 10.), (90., 40.)];
    let mut cases = vec![];
    for (kind_index, kind) in [LineType::Simple, LineType::WithSteps, LineType::Curved]
        .into_iter()
        .enumerate()
    {
        for ratio in [1., 1.25, 1.5, 2.] {
            for inverted in [false, true] {
                for top in [-20., 0., 20., 80.] {
                    for range in [
                        ValueRange { from: 0, to: 5 },
                        ValueRange { from: 1, to: 4 },
                        ValueRange { from: 2, to: 3 },
                    ] {
                        let data = AreaRendererData {
                            line: LineRendererData {
                                items: points
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (x, y))| LinePoint {
                                        index: (i as f64).into(),
                                        x: *x,
                                        y: *y,
                                        color: "black".into(),
                                    })
                                    .collect(),
                                visible_range: range,
                                bar_width: 6.,
                                line_type: Some(kind),
                                line_width: LineWidth::Three,
                                line_style: LineStyle::Dashed,
                                point_markers_radius: None,
                            },
                            fill_colors: [
                                ("red", "white"),
                                ("blue", "white"),
                                ("blue", "white"),
                                ("red", "black"),
                                ("green", "black"),
                            ]
                            .into_iter()
                            .map(|(a, b)| (a.into(), b.into()))
                            .collect(),
                            fill_type: kind,
                            height: 80.,
                            top,
                            inverted,
                        };
                        let fills = data
                            .draw_commands(PixelRatio {
                                horizontal: ratio,
                                vertical: ratio,
                            })
                            .0;
                        let commands=fills.into_iter().map(|f|format!("{{\"path\":[{}],\"gradient\":{{\"top\":{},\"bottom\":{},\"stops\":[{}]}}}}",path_json(&f.path),f.gradient.top,f.gradient.bottom,f.gradient.stops.iter().map(|(o,c)|format!("[{}, {:?}]",o,c)).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(",");
                        cases.push(format!("{{\"kind\":{},\"ratio\":{},\"inverted\":{},\"top\":{},\"from\":{},\"to\":{},\"fills\":[{}]}}",kind_index,ratio,inverted,top,range.from,range.to,commands));
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}

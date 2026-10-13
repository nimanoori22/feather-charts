use feather_charts::{
    model::time_data::ValueRange,
    renderers::{
        area_renderer::VerticalGradient,
        baseline_renderer::BaselineRendererData,
        draw_line::{LineStyle, LineType, LineWidth},
        grid_renderer::PixelRatio,
        line_renderer::{LinePoint, LineRendererData, PathOperation, Point},
    },
};
fn p(p: Point) -> String {
    format!("{},{}", p.x, p.y)
}
fn path(ops: &[PathOperation]) -> String {
    ops.iter()
        .map(|op| match *op {
            PathOperation::MoveTo(a) => format!("[\"M\",{}]", p(a)),
            PathOperation::LineTo(a) => format!("[\"L\",{}]", p(a)),
            PathOperation::CubicTo {
                control1,
                control2,
                end,
            } => format!("[\"C\",{},{},{}]", p(control1), p(control2), p(end)),
        })
        .collect::<Vec<_>>()
        .join(",")
}
fn gradient(g: &VerticalGradient) -> String {
    format!(
        "{{\"top\":{},\"bottom\":{},\"stops\":[{}]}}",
        g.top,
        g.bottom,
        g.stops
            .iter()
            .map(|(o, c)| format!("[{}, {:?}]", o, c))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn main() {
    let mut cases = vec![];
    for (k, kind) in [LineType::Simple, LineType::WithSteps, LineType::Curved]
        .into_iter()
        .enumerate()
    {
        for ratio in [1., 1.25, 1.5, 2.] {
            for base in [-50., 0., 20., 40., 80., 150.] {
                for (top, bottom) in [
                    (None, None),
                    (Some(-20.), Some(80.)),
                    (Some(10.), Some(60.)),
                    (Some(20.), Some(20.)),
                ] {
                    if top == bottom && top == Some(base) {
                        continue;
                    }
                    for range in [
                        ValueRange { from: 0, to: 5 },
                        ValueRange { from: 1, to: 4 },
                        ValueRange { from: 2, to: 3 },
                    ] {
                        for visible in [false, true] {
                            let fill_colors = [
                                ["green", "white", "white", "red"],
                                ["blue", "white", "white", "red"],
                                ["blue", "white", "white", "red"],
                                ["green", "black", "black", "red"],
                                ["green", "white", "white", "red"],
                            ]
                            .map(|a| a.map(String::from))
                            .to_vec();
                            let stroke_colors = [
                                ["green", "red"],
                                ["green", "red"],
                                ["blue", "yellow"],
                                ["blue", "yellow"],
                                ["green", "red"],
                            ]
                            .map(|a| a.map(String::from))
                            .to_vec();
                            let line = LineRendererData {
                                items: [(10., 50.), (30., 20.), (40., 60.), (70., 10.), (90., 40.)]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, (x, y))| LinePoint {
                                        index: (i as f64).into(),
                                        x,
                                        y,
                                        color: "black".into(),
                                    })
                                    .collect(),
                                visible_range: range,
                                bar_width: 6.,
                                line_type: visible.then_some(kind),
                                line_width: LineWidth::Three,
                                line_style: LineStyle::Dashed,
                                point_markers_radius: Some(3.),
                            };
                            let d = BaselineRendererData {
                                line,
                                fill_colors,
                                stroke_colors,
                                fill_type: kind,
                                base,
                                top,
                                bottom,
                                height: 80.,
                            };
                            let c = d
                                .draw_commands(PixelRatio {
                                    horizontal: ratio,
                                    vertical: ratio,
                                })
                                .unwrap();
                            let fills = c
                                .fills
                                .iter()
                                .map(|f| {
                                    format!(
                                        "{{\"path\":[{}],\"gradient\":{}}}",
                                        path(&f.path),
                                        gradient(&f.gradient)
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            let strokes=c.strokes.iter().map(|f|format!("{{\"path\":[{}],\"gradient\":{},\"width\":{},\"offset\":{},\"dash\":{:?}}}",path(&f.stroke.path),gradient(&f.gradient),f.stroke.width,f.stroke.dash_offset,f.stroke.dash_pattern)).collect::<Vec<_>>().join(",");
                            let markers = c
                                .markers
                                .iter()
                                .map(|g| {
                                    format!(
                                        "{{\"gradient\":{},\"markers\":[{}]}}",
                                        gradient(&g.gradient),
                                        g.markers
                                            .iter()
                                            .map(|m| format!(
                                                "[{},{},{}]",
                                                m.center.x, m.center.y, m.radius
                                            ))
                                            .collect::<Vec<_>>()
                                            .join(",")
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            cases.push(format!("{{\"kind\":{k},\"ratio\":{ratio},\"base\":{base},\"top\":{},\"bottom\":{},\"from\":{},\"to\":{},\"visible\":{visible},\"fills\":[{fills}],\"strokes\":[{strokes}],\"markers\":[{markers}]}}",top.map_or("null".into(),|x|x.to_string()),bottom.map_or("null".into(),|x|x.to_string()),range.from,range.to));
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}

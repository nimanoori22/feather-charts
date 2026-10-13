//! Actual-source geometry comparison records; no renderer or window required.
use feather_charts::{
    model::time_data::SeriesItemsIndexesRange,
    renderers::{
        draw_line::LineType,
        line_hit_test::line_hit,
        line_renderer::LinePoint,
        range_hit_test::{RangePoint, range_hit},
    },
};
#[path = "hover_selection/pane_cases.rs"]
mod pane_cases;
fn main() {
    let mut records = vec![];
    for spacing in [0.5, 6., 20.] {
        let points = (0..5)
            .map(|i| LinePoint {
                index: (i as f64).into(),
                x: i as f64 * spacing,
                y: if i % 2 == 0 { 10. } else { 30. },
                color: String::new(),
            })
            .collect::<Vec<_>>();
        let items = format!(
            "[{}]",
            points
                .iter()
                .map(|p| format!("{{\"x\":{},\"y\":{}}}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(",")
        );
        for (from, to) in [(0, 5), (1, 4), (2, 3), (0, 0)] {
            for (kind, code) in [
                (LineType::Simple, 0),
                (LineType::WithSteps, 1),
                (LineType::Curved, 2),
            ] {
                for width in [1., 4.] {
                    for radius in [None, Some(3.)] {
                        for tolerance in [0., 3.] {
                            for xi in -2..=12 {
                                for yi in -2..=12 {
                                    let x = xi as f64 * spacing / 2.;
                                    let y = yi as f64 * 4.;
                                    let result = line_hit(
                                        &points,
                                        SeriesItemsIndexesRange { from, to },
                                        x,
                                        y,
                                        kind,
                                        width,
                                        radius,
                                        spacing,
                                        tolerance,
                                    );
                                    let expected = result
                                        .map(|(d, p)| format!("[{d},{}]", p as u8))
                                        .unwrap_or("null".into());
                                    records.push(format!("{{\"kind\":\"line\",\"items\":{items},\"from\":{from},\"to\":{to},\"x\":{x},\"y\":{y},\"lineType\":{code},\"width\":{width},\"radius\":{},\"spacing\":{spacing},\"tolerance\":{tolerance},\"expected\":{expected}}}",radius.map(|r|r.to_string()).unwrap_or("null".into())));
                                }
                            }
                        }
                    }
                }
            }
        }
        for gap in [false, true] {
            let points = (0..5)
                .map(|i| RangePoint {
                    index: if gap { i as f64 * 2. } else { i as f64 },
                    x: i as f64 * spacing,
                    top: 10.,
                    bottom: if i % 2 == 0 { 30. } else { -10. },
                })
                .collect::<Vec<_>>();
            let items = format!(
                "[{}]",
                points
                    .iter()
                    .map(|p| format!(
                        "{{\"time\":{},\"x\":{},\"top\":{},\"bottom\":{}}}",
                        p.index, p.x, p.top, p.bottom
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            for tolerance in [0., 3.] {
                for xi in -2..=12 {
                    for yi in -3..=12 {
                        let x = xi as f64 * spacing / 2.;
                        let y = yi as f64 * 4.;
                        let expected = range_hit(&points, x, y, spacing, tolerance)
                            .map(|d| d.to_string())
                            .unwrap_or("null".into());
                        records.push(format!("{{\"kind\":\"range\",\"items\":{items},\"x\":{x},\"y\":{y},\"spacing\":{spacing},\"tolerance\":{tolerance},\"expected\":{expected}}}"));
                    }
                }
            }
        }
    }
    records.extend(pane_cases::records());
    println!("[{}]", records.join(","));
}

use super::*;
use crate::api::TextWatermarkLine;
#[test]
fn source_layout_cases() {
    for horizontal in [Horizontal::Left, Horizontal::Center, Horizontal::Right] {
        for vertical in [Vertical::Top, Vertical::Center, Vertical::Bottom] {
            for (width, height) in [
                (400., 200.),
                (80., 20.),
                (0., 1.),
                (1., 0.),
                (0., 0.),
                (160., 90.),
            ] {
                for scenario in 0..5 {
                    let (texts, widths, heights) = match scenario {
                        0 => (vec!["A"], vec![120.], vec![None]),
                        1 => (
                            vec!["A", "", "B"],
                            vec![120., 0., 240.],
                            vec![None, None, Some(18.)],
                        ),
                        2 => (
                            vec!["A", "B"],
                            vec![f64::from(width), 5.],
                            vec![Some(60.), Some(12.)],
                        ),
                        3 => (
                            vec!["A", "  ", "B"],
                            vec![500., 20., 60.],
                            vec![Some(0.), Some(13.5), None],
                        ),
                        _ => (
                            vec!["BTCUSD", "日本語"],
                            vec![0., 800.],
                            vec![Some(150.), Some(40.)],
                        ),
                    };
                    let options = TextWatermarkOptions {
                        horizontal_alignment: horizontal,
                        vertical_alignment: vertical,
                        lines: texts
                            .iter()
                            .zip(&heights)
                            .map(|(t, h)| TextWatermarkLine {
                                text: (*t).into(),
                                line_height: *h,
                                ..Default::default()
                            })
                            .collect(),
                        ..Default::default()
                    };
                    let placements = layout(&options, Size::new(width, height), &widths);
                    let actual = placements
                        .iter()
                        .map(|p| format!("[{},{},{}]", p.x, p.y, p.zoom))
                        .collect::<Vec<_>>()
                        .join(",");
                    println!(
                        "WATERMARK_LAYOUT {{\"horizontal\":\"{horizontal:?}\",\"vertical\":\"{vertical:?}\",\"size\":[{width},{height}],\"scenario\":{scenario},\"actual\":[{actual}]}}"
                    );
                    assert!(placements.iter().all(|p| p.x.is_finite()
                        && p.y.is_finite()
                        && p.zoom.is_finite()
                        && p.zoom >= 0.
                        && p.zoom <= 1.));
                }
            }
        }
    }
}

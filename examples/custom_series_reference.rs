#[path = "custom_series/box_whisker.rs"]
#[allow(dead_code)]
mod box_whisker;
use feather_charts::model::{
    horz_scale_behavior_time::types::UtcTimestamp,
    icustom_series::{CustomSeriesDataItem, CustomSeriesPaneView, CustomSeriesWhitespaceData},
};
fn main() {
    let view = box_whisker::View::default();
    assert!(view.is_whitespace(&CustomSeriesDataItem::Whitespace(
        CustomSeriesWhitespaceData {
            time: UtcTimestamp::new(1.).into(),
            custom_values: None
        }
    )));
    let mut cases = vec![];
    for ratio in [1., 1.25, 1.5, 2.] {
        for spacing in [0.25, 1., 2., 2.5, 3., 4., 4.01, 6., 10., 20.] {
            for x in [-0.5, 0., 0.5, 17.3] {
                for inverted in [false, true] {
                    let mut q = [-0.5, 10.2, 20., 32.4, 40.5];
                    if inverted {
                        q.reverse();
                    }
                    let shapes = box_whisker::geometry(x, q, &[-10., 50.5], spacing, ratio);
                    let prices = view
                        .price_value_builder(&box_whisker::Datum {
                            time: UtcTimestamp::new(1.).into(),
                            quartiles: q,
                            outliers: vec![-10., 50.5],
                        })
                        .into_iter()
                        .map(|price| price.value())
                        .collect::<Vec<_>>();
                    let result = shapes
                        .into_iter()
                        .map(|shape| {
                            format!(
                                "[{}, {}, {}, {}, {}, {}]",
                                shape.color,
                                shape.circle,
                                shape.coordinates[0],
                                shape.coordinates[1],
                                shape.coordinates[2],
                                shape.coordinates[3]
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    cases.push(format!("{{\"ratio\":{ratio},\"spacing\":{spacing},\"x\":{x},\"q\":{q:?},\"prices\":{prices:?},\"result\":[{result}]}}"));
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}

//! Source axis alignment: render positions may move; price coordinates never do.
use crate::model::axis_snapshots::SeriesAxisLabel;
pub fn align_labels(labels: &mut [SeriesAxisLabel], center: f64, height: f64, label_height: f64) {
    for label in labels.iter_mut() {
        label.render_coordinate = label.axis.coordinate;
    }
    let mut top = (0..labels.len())
        .filter(|i| labels[*i].axis.coordinate <= center)
        .collect::<Vec<_>>();
    let mut bottom = (0..labels.len())
        .filter(|i| labels[*i].axis.coordinate > center)
        .collect::<Vec<_>>();
    top.sort_by(|a, b| {
        labels[*b]
            .axis
            .coordinate
            .total_cmp(&labels[*a].axis.coordinate)
    });
    if !top.is_empty() && !bottom.is_empty() {
        bottom.push(top[0]);
    }
    bottom.sort_by(|a, b| {
        labels[*a]
            .axis
            .coordinate
            .total_cmp(&labels[*b].axis.coordinate)
    });
    let half = (label_height / 2.).floor();
    for label in labels.iter_mut() {
        let y = label.axis.coordinate;
        if y > -half && y < half {
            label.render_coordinate = half;
        }
        if y > height - half && y < height + half {
            label.render_coordinate = height - half;
        }
    }
    recalculate(labels, &top, 1., height, label_height);
    recalculate(labels, &bottom, -1., height, label_height);
}
fn recalculate(
    labels: &mut [SeriesAxisLabel],
    indices: &[usize],
    direction: f64,
    height: f64,
    h: f64,
) {
    let Some(first) = indices.first() else {
        return;
    };
    let mut start = 0;
    let mut space =
        (direction * (height / 2. - (labels[*first].render_coordinate - h / 2.))).max(0.);
    for i in 1..indices.len() {
        let y = labels[indices[i]].render_coordinate;
        let prev = labels[indices[i - 1]].render_coordinate;
        let overlaps = if direction == 1. {
            y > prev - h
        } else {
            y < prev + h
        };
        if overlaps {
            let render = prev - h * direction;
            labels[indices[i]].render_coordinate = render;
            let edge = render - direction * h / 2.;
            let outside = if direction == 1. {
                edge < 0.
            } else {
                edge > height
            };
            if outside && space > 0. {
                let desired = if direction == 1. {
                    -1. - edge
                } else {
                    edge - height
                };
                let shift = desired.min(space);
                for key in &indices[start..] {
                    labels[*key].render_coordinate += direction * shift;
                }
                space -= shift;
            }
        } else {
            start = i;
            space = if direction == 1. {
                prev - h - y
            } else {
                y - (prev + h)
            };
        }
    }
}

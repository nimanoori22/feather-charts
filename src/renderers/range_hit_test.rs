//! Source high/low and value/base interval hits in logical coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct RangePoint {
    pub index: f64,
    pub x: f64,
    pub top: f64,
    pub bottom: f64,
}
pub fn range_hit(
    items: &[RangePoint],
    x: f64,
    y: f64,
    spacing: f64,
    tolerance: f64,
) -> Option<f64> {
    let radius = spacing / 2. + tolerance;
    let from = items.partition_point(|p| p.x < x - radius);
    let to = items.partition_point(|p| p.x <= x + radius);
    let mut closest = f64::INFINITY;
    for i in from..to {
        let p = &items[i];
        let left = if i > 0 && items[i - 1].index == p.index - 1. {
            (items[i - 1].x + p.x) / 2.
        } else {
            p.x - spacing / 2.
        };
        let right = if i + 1 < items.len() && items[i + 1].index == p.index + 1. {
            (items[i + 1].x + p.x) / 2.
        } else {
            p.x + spacing / 2.
        };
        if x < left - tolerance || x > right + tolerance {
            continue;
        }
        let top = p.top.min(p.bottom);
        let bottom = p.top.max(p.bottom);
        if y >= top && y <= bottom {
            closest = 0.;
        } else if y >= top - tolerance && y <= bottom + tolerance {
            closest = closest.min((y - top).abs().min((y - bottom).abs()));
        }
    }
    closest.is_finite().then_some(closest)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gap_slots_and_inverted_vertical_ranges_follow_source() {
        let items = vec![
            RangePoint {
                index: 0.,
                x: 0.,
                top: 10.,
                bottom: 0.,
            },
            RangePoint {
                index: 3.,
                x: 30.,
                top: 0.,
                bottom: 10.,
            },
        ];
        assert_eq!(range_hit(&items, 5., 5., 10., 0.), Some(0.));
        assert_eq!(range_hit(&items, 15., 5., 10., 0.), None);
        assert_eq!(range_hit(&items, 0., 11., 10., 1.), Some(1.));
        assert_eq!(range_hit(&items, 0., 11.01, 10., 1.), None);
    }
}

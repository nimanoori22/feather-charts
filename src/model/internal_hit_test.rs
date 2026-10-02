#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HitTestPriority {
    #[default]
    Range,
    Line,
    Point,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InternalHitTestCandidate {
    pub distance: f64,
    pub priority: HitTestPriority,
    pub cursor_style: Option<String>,
    pub external_id: Option<String>,
}
pub fn is_better_hit(
    candidate: &InternalHitTestCandidate,
    current: Option<&InternalHitTestCandidate>,
) -> bool {
    let Some(current) = current else { return true };
    match (candidate.priority, current.priority) {
        (HitTestPriority::Point, HitTestPriority::Point) => candidate.distance < current.distance,
        (HitTestPriority::Point, _) => true,
        (_, HitTestPriority::Point) => false,
        _ => candidate.distance < current.distance,
    }
}

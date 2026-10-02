//! Backend-independent options for the horizontal scale.

/// Scheduling priority requested for optional precomputed data conflation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PrecomputeConflationPriority {
    #[default]
    Background,
    UserVisible,
    UserBlocking,
}

/// Options shared by all horizontal-scale behaviors.
#[derive(Clone, Debug, PartialEq)]
pub struct HorzScaleOptions {
    pub right_offset: f64,
    pub right_offset_pixels: Option<f64>,
    pub bar_spacing: f64,
    pub min_bar_spacing: f64,
    pub max_bar_spacing: f64,
    pub fix_left_edge: bool,
    pub fix_right_edge: bool,
    pub lock_visible_time_range_on_resize: bool,
    pub right_bar_stays_on_scroll: bool,
    pub border_visible: bool,
    pub border_color: String,
    pub visible: bool,
    pub time_visible: bool,
    pub seconds_visible: bool,
    pub shift_visible_range_on_new_bar: bool,
    pub allow_shift_visible_range_on_whitespace_replacement: bool,
    pub ticks_visible: bool,
    pub tick_mark_max_character_length: Option<usize>,
    pub uniform_distribution: bool,
    pub minimum_height: f64,
    pub allow_bold_labels: bool,
    pub ignore_whitespace_indices: bool,
    pub enable_conflation: bool,
    pub conflation_threshold_factor: Option<f64>,
    pub precompute_conflation_on_init: bool,
    pub precompute_conflation_priority: PrecomputeConflationPriority,
}
#[derive(Clone, Debug, Default)]
pub struct HorzScaleOptionsPatch {
    pub right_offset: Option<f64>,
    pub right_offset_pixels: Option<Option<f64>>,
    pub bar_spacing: Option<f64>,
    pub min_bar_spacing: Option<f64>,
    pub max_bar_spacing: Option<f64>,
    pub fix_left_edge: Option<bool>,
    pub fix_right_edge: Option<bool>,
    pub lock_visible_time_range_on_resize: Option<bool>,
    pub right_bar_stays_on_scroll: Option<bool>,
    pub border_visible: Option<bool>,
    pub border_color: Option<String>,
    pub visible: Option<bool>,
    pub time_visible: Option<bool>,
    pub seconds_visible: Option<bool>,
    pub shift_visible_range_on_new_bar: Option<bool>,
    pub allow_shift_visible_range_on_whitespace_replacement: Option<bool>,
    pub ticks_visible: Option<bool>,
    pub tick_mark_max_character_length: Option<Option<usize>>,
    pub uniform_distribution: Option<bool>,
    pub minimum_height: Option<f64>,
    pub allow_bold_labels: Option<bool>,
    pub ignore_whitespace_indices: Option<bool>,
    pub enable_conflation: Option<bool>,
    pub conflation_threshold_factor: Option<Option<f64>>,
    pub precompute_conflation_on_init: Option<bool>,
    pub precompute_conflation_priority: Option<PrecomputeConflationPriority>,
}

impl Default for HorzScaleOptions {
    fn default() -> Self {
        Self {
            right_offset: 0.0,
            right_offset_pixels: None,
            bar_spacing: 6.0,
            min_bar_spacing: 0.5,
            max_bar_spacing: 0.0,
            fix_left_edge: false,
            fix_right_edge: false,
            lock_visible_time_range_on_resize: false,
            right_bar_stays_on_scroll: false,
            border_visible: true,
            border_color: "#2B2B43".to_owned(),
            visible: true,
            time_visible: false,
            seconds_visible: true,
            shift_visible_range_on_new_bar: true,
            allow_shift_visible_range_on_whitespace_replacement: false,
            ticks_visible: false,
            tick_mark_max_character_length: None,
            uniform_distribution: false,
            minimum_height: 0.0,
            allow_bold_labels: true,
            ignore_whitespace_indices: false,
            enable_conflation: false,
            conflation_threshold_factor: None,
            precompute_conflation_on_init: false,
            precompute_conflation_priority: PrecomputeConflationPriority::Background,
        }
    }
}

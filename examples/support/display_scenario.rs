//! Explicit, deterministic input shared by the demo and display tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayScenario {
    #[default]
    Default,
    LargeNegative,
    Narrow,
    Gradient,
    Historical,
    Animation,
    Empty,
}
impl DisplayScenario {
    pub const ALL: [Self; 7] = [
        Self::Default,
        Self::LargeNegative,
        Self::Narrow,
        Self::Gradient,
        Self::Historical,
        Self::Animation,
        Self::Empty,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::LargeNegative => "large-negative",
            Self::Narrow => "narrow",
            Self::Gradient => "gradient",
            Self::Historical => "historical",
            Self::Animation => "animation",
            Self::Empty => "empty",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }
}
#[derive(Clone, Debug)]
pub struct DisplayConfig {
    pub scenario: DisplayScenario,
    pub width: f32,
    pub height: f32,
    pub multiplier: f64,
    pub negative: bool,
    pub gradient: bool,
    pub left_axis: bool,
    pub ticks: bool,
    pub fixed_edges: bool,
    pub font_size: f64,
    pub font_family: String,
}
impl Default for DisplayConfig {
    fn default() -> Self {
        Self::for_scenario(DisplayScenario::Default)
    }
}
impl DisplayConfig {
    pub fn for_scenario(scenario: DisplayScenario) -> Self {
        Self {
            scenario,
            width: if scenario == DisplayScenario::Narrow {
                260.
            } else {
                800.
            },
            height: 500.,
            multiplier: if scenario == DisplayScenario::LargeNegative {
                10000.
            } else {
                1.
            },
            negative: scenario == DisplayScenario::LargeNegative,
            gradient: scenario == DisplayScenario::Gradient,
            left_axis: matches!(
                scenario,
                DisplayScenario::Gradient | DisplayScenario::LargeNegative
            ),
            ticks: scenario != DisplayScenario::Default,
            fixed_edges: false,
            font_size: 12.,
            font_family: "sans-serif".into(),
        }
    }
    pub fn value(&self, index: usize, adjustment: f64) -> f64 {
        let value = (100. + (index as f64 * 0.2).sin() * 8. + index as f64 * 0.04 + adjustment)
            * self.multiplier;
        if self.negative { -value } else { value }
    }
}

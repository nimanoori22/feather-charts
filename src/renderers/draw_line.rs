/// A supported chart line width, in logical pixels.
///
/// Lightweight Charts limits this particular option to one through four.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LineWidth {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LineType {
    #[default]
    Simple,
    WithSteps,
    Curved,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dotted = 1,
    Dashed = 2,
    LargeDashed = 3,
    SparseDotted = 4,
}

pub fn dash_pattern(style: LineStyle, width: f32) -> Vec<f32> {
    match style {
        LineStyle::Solid => vec![],
        LineStyle::Dotted => vec![width, width],
        LineStyle::Dashed => vec![2.0 * width, 2.0 * width],
        LineStyle::LargeDashed => vec![6.0 * width, 6.0 * width],
        LineStyle::SparseDotted => vec![width, 4.0 * width],
    }
}

impl LineWidth {
    pub const fn pixels(self) -> f32 {
        self as u8 as f32
    }
}

impl TryFrom<u8> for LineWidth {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            _ => Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LineWidth;

    #[test]
    fn accepts_only_the_supported_widths() {
        assert_eq!(LineWidth::try_from(3), Ok(LineWidth::Three));
        assert_eq!(LineWidth::Three.pixels(), 3.0);
        assert_eq!(LineWidth::try_from(5), Err(5));
    }
}

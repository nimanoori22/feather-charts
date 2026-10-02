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

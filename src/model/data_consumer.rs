//! Common timed-data boundary shared by horizontal-scale behaviors.

/// Data carrying a horizontal-scale item.
///
/// Behavior implementations only need to inspect or normalize the time field;
/// value-specific series data remains independent of this contract.
pub trait TimedData {
    type Item;

    fn time(&self) -> &Self::Item;
    fn time_mut(&mut self) -> &mut Self::Item;
}

/// A data item with a horizontal item but no plotted value.
#[derive(Clone, Debug, PartialEq)]
pub struct WhitespaceData<T> {
    pub time: T,
}

impl<T> TimedData for WhitespaceData<T> {
    type Item = T;

    fn time(&self) -> &Self::Item {
        &self.time
    }

    fn time_mut(&mut self) -> &mut Self::Item {
        &mut self.time
    }
}

#[cfg(test)]
mod tests {
    use super::{TimedData, WhitespaceData};

    #[test]
    fn exposes_time_for_behavior_preprocessing() {
        let mut data = WhitespaceData { time: "2021-02-03" };

        *data.time_mut() = "2021-02-04";
        assert_eq!(data.time(), &"2021-02-04");
    }
}

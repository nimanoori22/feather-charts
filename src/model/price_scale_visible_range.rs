//! Shared, synchronous visible-range snapshot for price-scale source adapters.

use crate::model::{range_impl::RangeImpl, time_data::TimePointIndex};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
pub struct PriceScaleVisibleRange {
    value: Rc<RefCell<Option<RangeImpl<TimePointIndex>>>>,
}

impl PriceScaleVisibleRange {
    pub fn set(&self, range: Option<&RangeImpl<TimePointIndex>>) {
        *self.value.borrow_mut() = range.copied();
    }

    pub fn get(&self) -> Option<RangeImpl<TimePointIndex>> {
        *self.value.borrow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_the_current_snapshot() {
        let first = PriceScaleVisibleRange::default();
        let second = first.clone();
        let range = RangeImpl::new(2.0.into(), 4.0.into());
        first.set(Some(&range));
        assert_eq!(second.get(), Some(range));
        second.set(None);
        assert_eq!(first.get(), None);
    }
}

pub trait ZOrdered {
    fn z_order(&self) -> i32;
}
pub fn sort_sources<T: ZOrdered>(sources: &mut [T]) {
    sources.sort_by_key(ZOrdered::z_order);
}

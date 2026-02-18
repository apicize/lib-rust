/// Trait to describe oneself
pub trait Disabled {
    /// Returns whether the entity is disabled
    fn get_disabled(&self) -> bool;
}
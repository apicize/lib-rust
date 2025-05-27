/// Entity that has warnings that should be shown to user upon access
pub trait Warnings {
    /// Retrieve warnings
    fn get_warnings(&self) -> &Option<Vec<String>>;
}

/// Trait to allow appending warnings
pub trait EditableWarnings {
    /// Set warnings
    fn set_warnings(&mut self, warnings: Option<Vec<String>>);
}
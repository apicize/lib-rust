/// Entity that has warnings that should be shown to user upon access
pub trait Warnings {
    /// Retrieve warnings
    fn get_warnings(&self) -> &Option<Vec<String>>;

    /// Set warnings
    fn add_warning(&mut self, warning: String);
}


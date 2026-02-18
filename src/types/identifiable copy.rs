/// Trait to describe oneself
pub trait Disable {
    /// Return ID of self
    fn get_id(&self) -> &str;

    /// Return name of self
    fn get_name(&self) -> &str;

    /// Return a title to display in a list
    fn get_title(&self) -> String;
}

pub trait CloneIdentifiable {
    /// Create a copy with a new identifier
    fn clone_as_new(&self, new_name: String) -> Self;
}
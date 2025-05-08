/// Trait to describe oneself
pub trait Identifiable {
    /// Return ID of self
    fn get_id(&self) -> &String;

    /// Return name of self
    fn get_name(&self) -> &String;

    /// Return a title to display in a list
    fn get_title(&self) -> String;

    /// Create a copy with a new identifier
    fn clone_as_new(&self, new_name: String) -> Self;
}

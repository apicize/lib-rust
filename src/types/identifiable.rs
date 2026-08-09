/// Trait to describe oneself
pub trait Identifiable {
    /// Return ID of self
    fn get_id(&self) -> &str;

    /// Return name of self
    fn get_name(&self) -> &str;

    /// Return a title to display in a list
    fn get_title(&self) -> String;

    /// Whether this entity can contain child entities (i.e. accept an "Under" placement).
    /// Defaults to false; container types (e.g. request groups) override to true.
    fn can_have_children(&self) -> bool {
        false
    }
}

pub trait CloneIdentifiable {
    /// Create a copy with a new identifier
    fn clone_as_new(&self, new_name: String) -> Self;
}

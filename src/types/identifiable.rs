
/// Trait to describe oneself
pub trait Identifable {
    /// Return ID of self
    fn get_id(&self) -> &String;

    /// Return name of self
    fn get_name(&self) -> &String; 

    /// Return a title to display in a list
    fn get_title(&self) -> String;
}

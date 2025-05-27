use std::collections::HashMap;

/// Entity that has validation errors,
pub trait ValidationErrors {
    /// Retrieve validation errors by property name
    fn get_validation_errors(&self) -> &Option<HashMap<String, String>>;
}
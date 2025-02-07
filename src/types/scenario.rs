use serde::{Deserialize, Serialize};
use crate::Identifable;
use crate::NameValuePair;
use crate::utility::*;

/// A set of variables that can be injected into templated values
/// when submitting an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Scenario {
    /// Uniquely identifies scenario
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Name of variable to substitute (avoid using curly braces)
    pub name: String,
    /// Value of variable to substitute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<NameValuePair>>,
}

impl Identifable for Scenario {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            format!("{} (Unnamed)", self.id)
        } else {
            self.name.to_string()
        }
    }
}

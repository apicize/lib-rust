use serde::{Deserialize, Serialize};
use crate::Identifiable;
use crate::utility::*;

use super::Variable;

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
    pub variables: Option<Vec<Variable>>,
}

impl Default for Scenario {
    fn default() -> Self {
        Self { id: generate_uuid(), name: Default::default(), variables: Default::default() }
    }
}

impl Identifiable for Scenario {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            "(Unamed)".to_string()
        } else {
            self.name.to_string()
        }
    }

    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

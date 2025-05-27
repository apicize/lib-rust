use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use crate::Identifiable;
use crate::utility::*;

use super::identifiable::CloneIdentifiable;
use super::ValidationErrors;
use super::Variable;
use super::Warnings;

/// A set of variables that can be injected into templated values
/// when submitting an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    /// Uniquely identifies scenario
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Name of variable to substitute (avoid using curly braces)
    pub name: String,
    /// Value of variable to substitute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<Variable>>,
    /// Validation errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<HashMap<String, String>>,
}

impl Identifiable for Scenario {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            "(Unamed)".to_string()
        } else {
            self.name.to_string()
        }
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self { 
            id: generate_uuid(),
            name: Default::default(),
            variables: Default::default(),
            validation_errors: Default::default() 
        }
    }
}

impl CloneIdentifiable for Scenario {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

impl Warnings for Scenario {
    fn get_warnings(&self) -> &Option<Vec<String>> {
        &None
    }
}

impl ValidationErrors for Scenario {
    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        &self.validation_errors
    }
}
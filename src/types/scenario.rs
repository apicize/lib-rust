use std::collections::HashMap;

use crate::{Identifiable, Validated, ValidationState, add_validation_error, remove_validation_error, utility::*};
use serde::{Deserialize, Serialize};

use super::Variable;
use super::identifiable::CloneIdentifiable;

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
    /// Validation state
    #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
    pub validation_state: ValidationState,
    /// Warnings for invalid values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_warnings: Option<Vec<String>>,
    /// Validation errors
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
            "(Unnamed)".to_string()
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
            validation_state: Default::default(),
            validation_warnings: Default::default(),
            validation_errors: Default::default(),
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

impl Scenario {
    pub fn perform_validation(&mut self) {
        self.validate_name();
        self.validate_variables();
    }

    pub fn validate_name(&mut self) {
        let name_ok = ! self.name.trim().is_empty();
        if name_ok {
            remove_validation_error(&mut self.validation_errors, "name");
        } else {
            add_validation_error(&mut self.validation_errors, "name", "Name is required");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }

    pub fn validate_variables(&mut self) {
        let variables_ok = match &self.variables {
            Some(vars) => ! vars.iter().any(|v| v.name.trim().is_empty()),
            None => true
        };
        if variables_ok {
            remove_validation_error(&mut self.validation_errors, "variables");
        } else {
            add_validation_error(&mut self.validation_errors, "variables", "Variables must be named");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }    
}

impl Validated for Scenario {
    fn get_validation_state(&self) -> super::ValidationState {
        self.validation_state
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        &self.validation_warnings
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        self.validation_warnings = warnings;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        &self.validation_errors
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        self.validation_errors = errors;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }
}

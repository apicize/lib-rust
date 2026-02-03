use std::collections::HashMap;

use super::{Identifiable, identifiable::CloneIdentifiable};
use crate::{Validated, ValidationState, add_validation_error, remove_validation_error, utility::*};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum DataSourceType {
    #[default]
    JSON,
    #[serde(rename = "FILE-JSON")]
    FileJSON,
    #[serde(rename = "FILE-CSV")]
    FileCSV,
}

/// Data that may be sourced from text, JSON, a JSON File or a CSV file
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct DataSet {
    /// Uniquely identifies external data
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Names external data
    pub name: String,
    /// Source type of the data set
    #[serde(rename = "type")]
    pub source_type: DataSourceType,
    /// Source of the data set
    pub source: String,
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

impl Validated for DataSet {
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

impl Default for DataSet {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: String::default(),
            source_type: DataSourceType::JSON,
            source: String::default(),
            validation_state: ValidationState::default(),
            validation_warnings: None,
            validation_errors: None,
        }
    }
}

impl Identifiable for DataSet {
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

impl CloneIdentifiable for DataSet {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

impl DataSet {
    pub fn perform_validation(&mut self) {
        self.validate_name();
        self.validate_source();
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

    pub fn validate_source(&mut self) {
        let source_ok = self.source_type == DataSourceType::JSON || ! self.source.trim().is_empty();
        if source_ok {
            remove_validation_error(&mut self.validation_errors, "source");
        } else {
            add_validation_error(&mut self.validation_errors, "source", "Source file name is required");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }

}
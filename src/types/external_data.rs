use std::collections::HashMap;

use super::{identifiable::CloneIdentifiable, Identifiable};
use crate::utility::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum ExternalDataSourceType {
    JSON,
    #[serde(rename = "FILE-JSON")]
    FileJSON,
    #[serde(rename = "FILE-CSV")]
    FileCSV,
}

/// Data that may be sourced from text, JSON, a JSON File or a CSV file
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ExternalData {
    /// Uniquely identifies external data
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Names external data
    pub name: String,
    /// Source type of the external data
    #[serde(rename = "type")]
    pub source_type: ExternalDataSourceType,
    /// Source of the external data
    pub source: String,
    /// Validation errors
    validation_errors: Option<HashMap<String, String>>,
}

impl Default for ExternalData {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: String::default(),
            source_type: ExternalDataSourceType::FileJSON,
            source: String::default(),
            validation_errors: None,
        }
    }
}

impl Identifiable for ExternalData {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
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

impl CloneIdentifiable for ExternalData {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

use serde::{Deserialize, Serialize};
use crate::utility::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum VariableSourceType {
    #[default]
    #[serde(rename="TEXT")]
    Text,
    JSON,
    #[serde(rename="FILE-JSON")]
    FileJSON,
    #[serde(rename="FILE-CSV")]
    FileCSV,
    ExternalData,
}


impl VariableSourceType {
    fn is_default(&self) -> bool {
        *self == VariableSourceType::Text
    }
}

/// Data that may be sourced from text, JSON, a JSON File or a CSV file
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Variable {
    #[serde(default="generate_uuid")]
    pub name: String,
    #[serde(default, rename="type", skip_serializing_if = "VariableSourceType::is_default")]
    pub source_type: VariableSourceType,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>
}

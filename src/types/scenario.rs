use serde::{Deserialize, Serialize};
use crate::Identifable;
use crate::utility::*;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum ScenarioVariableType {
    #[serde(rename="TEXT")]
    Text,
    JSON,
    #[serde(rename="FILE-JSON")]
    FileJSON,
    #[serde(rename="FILE-CSV")]
    FileCSV
}

impl Default for ScenarioVariableType {
    fn default() -> Self {
        ScenarioVariableType::Text
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ScenarioVariable {
    pub name: String,
    #[serde(default, rename="type")]
    pub var_type: ScenarioVariableType,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>

}

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
    pub variables: Option<Vec<ScenarioVariable>>,
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

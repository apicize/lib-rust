use serde::{Deserialize, Serialize};
use crate::utility::*;
use super::Identifable;


#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum ExternalDataSourceType {
    JSON,
    #[serde(rename="FILE-JSON")]
    FileJSON,
    #[serde(rename="FILE-CSV")]
    FileCSV,
}

/// Data that may be sourced from text, JSON, a JSON File or a CSV file
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ExternalData {
    #[serde(default="generate_uuid")]
    pub id: String,
    pub name: String,
    #[serde(rename="type")]
    pub source_type: ExternalDataSourceType,
    pub source: String,
}


impl Identifable for ExternalData {
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

use std::collections::HashMap;
use std::fs::File;
use std::io::{self};
use std::path::PathBuf;

use crate::ApicizeError;
use crate::ScenarioVariableType;
use serde_json::{Map, Value};

use super::Scenario;

/// A set of variables that can be injected into templated values
/// when submitting an Apicize Request
pub struct ScenarioValueCache {
    allowed_path: Option<PathBuf>,
    cache: HashMap<String, HashMap<String, Result<Value, ApicizeError>>>,
}

impl ScenarioValueCache {
    pub fn new(allowed_path: Option<PathBuf>) -> Self {
        ScenarioValueCache {
            allowed_path,
            cache: HashMap::new(),
        }
    }

    pub fn get_scenario_values(
        &mut self,
        scenario: &Scenario,
    ) -> &HashMap<String, Result<Value, ApicizeError>> {
        self
            .cache
            .entry(scenario.id.clone())
            .or_insert_with(|| match &scenario.variables {
                Some(vars) => vars
                    .iter()
                    .map(|var| match var.var_type {
                        ScenarioVariableType::Text => {
                            (var.name.clone(), Ok(Value::String(var.value.clone())))
                        }
                        ScenarioVariableType::JSON => {
                            (var.name.clone(), convert_json(&var.name, &var.value))
                        }
                        ScenarioVariableType::FileJSON => (
                            var.name.clone(),
                            extract_json(&var.name, &var.value, &self.allowed_path),
                        ),
                        ScenarioVariableType::FileCSV => (
                            var.name.clone(),
                            extract_csv(&var.name, &var.value, &self.allowed_path),
                        ),
                    })
                    .collect::<HashMap<String, Result<Value, ApicizeError>>>(),
                None => HashMap::new(),
            })
    }
}

/// Convert a JSON value to an array of values for loop processing
fn convert_json(name: &str, value: &str) -> Result<Value, ApicizeError> {
    match serde_json::from_str::<Value>(value) {
        Ok(v) => Ok(v),
        Err(err) => Err(ApicizeError::from_serde(err, name)),
    }
}

/// Return serialized JSON after validating it
fn extract_json(
    name: &str,
    file_name: &str,
    allowed_path: &Option<PathBuf>,
) -> Result<Value, ApicizeError> {
    match get_absolute_file_name(file_name, allowed_path) {
        Ok(full_file_name) => match File::open(full_file_name) {
            Ok(file) => match serde_json::from_reader::<File, Value>(file) {
                Ok(v) => Ok(v),
                Err(err) => Err(ApicizeError::from_serde(err, name)),
            },
            Err(err) => Err(ApicizeError::from_io(err)),
        },
        Err(err) => Err(err),
    }
}

/// Return serialized CSV data rendered as JSON
fn extract_csv(
    name: &str,
    file_name: &str,
    allowed_path: &Option<PathBuf>,
) -> Result<Value, ApicizeError> {
    match get_absolute_file_name(file_name, allowed_path) {
        Ok(full_file_name) => match File::open(full_file_name) {
            Ok(file) => {
                let mut rdr = csv::Reader::from_reader(file);
                let mut data = Vec::<Value>::new();
                for record in rdr.deserialize::<Map<String, Value>>() {
                    match record {
                        Ok(r) => data.push(serde_json::Value::Object(r)),
                        Err(err) => return Err(ApicizeError::from_csv(err, name)),
                    }
                }
                Ok(serde_json::Value::Array(data))
            }
            Err(err) => Err(ApicizeError::from_io(err)),
        },
        Err(err) => Err(err),
    }
}

/// Return the absolute file name, ensuring it exists and that it is form the same directory as our workbook
fn get_absolute_file_name(
    file_name: &str,
    allowed_path: &Option<PathBuf>,
) -> Result<PathBuf, ApicizeError> {
    match allowed_path {
        Some(parent_data_path) => {
            let data_path = PathBuf::from(parent_data_path).join(file_name);
            if data_path.exists() {
                Ok(data_path)
            } else {
                Err(ApicizeError::from_io(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{} not found", file_name),
                )))
            }
        }
        None => Err(ApicizeError::Error {
            description: "External scenario variable files are unavailable in an unsaved workbook"
                .to_string(),
            source: None,
        }),
    }
}

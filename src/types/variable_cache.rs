use std::collections::HashMap;
use std::path::PathBuf;

use crate::VariableSourceType;
use crate::{convert_json, extract_csv, extract_json, ApicizeError};
use serde_json::Value;

use super::{ExternalData, ExternalDataSourceType, Scenario, Variable};

/// Cached storage of variables that have been deserialized from files or data
pub struct VariableCache {
    allowed_path: Option<PathBuf>,
    scenario_cache: HashMap<String, HashMap<String, Result<Value, ApicizeError>>>,
    data_cache: HashMap<String, Result<Value, ApicizeError>>,
}

impl VariableCache {
    pub fn new(allowed_path: &Option<PathBuf>) -> Self {
        VariableCache {
            allowed_path: allowed_path.clone(),
            scenario_cache: HashMap::new(),
            data_cache: HashMap::new(),
        }
    }

    pub fn get_scenario_values(
        &mut self,
        scenario: &Scenario,
    ) -> &HashMap<String, Result<Value, ApicizeError>> {
        self.scenario_cache
            .entry(scenario.id.clone())
            .or_insert_with(|| match &scenario.variables {
                Some(vars) => vars
                    .iter()
                    .filter(|v| Some(true) != v.disabled)
                    // .map(|var| (var.name.clone(), extract_value(var, &self.allowed_path)))
                    .map(|var| {
                        (var.name.clone(), {
                            match var.source_type {
                                VariableSourceType::Text => Ok(Value::String(var.value.clone())),
                                VariableSourceType::JSON => convert_json(&var.name, &var.value),
                                VariableSourceType::FileJSON => {
                                    extract_json(&var.name, &var.value, &self.allowed_path)
                                }
                                VariableSourceType::FileCSV => {
                                    extract_csv(&var.name, &var.value, &self.allowed_path)
                                }
                                VariableSourceType::ExternalData => {
                                    todo!("Add support for External Data")
                                }
                            }
                        })
                    })
                    .collect::<HashMap<String, Result<Value, ApicizeError>>>(),
                None => HashMap::new(),
            })
    }

    pub fn get_external_data(&mut self, data: &ExternalData) -> &Result<Value, ApicizeError> {
        self.data_cache
            .entry(data.name.clone())
            .or_insert_with(|| match data.source_type {
                ExternalDataSourceType::JSON => convert_json(&data.name, &data.source),
                ExternalDataSourceType::FileJSON => {
                    extract_json(&data.name, &data.source, &self.allowed_path)
                }
                ExternalDataSourceType::FileCSV => {
                    extract_csv(&data.name, &data.source, &self.allowed_path)
                }
            })
    }
}

pub fn extract_value(
    var: &Variable,
    allowed_path: &Option<PathBuf>,
) -> Result<Value, ApicizeError> {
    match var.source_type {
        VariableSourceType::Text => Ok(Value::String(var.value.clone())),
        VariableSourceType::JSON => convert_json(&var.name, &var.value),
        VariableSourceType::FileJSON => extract_json(&var.name, &var.value, allowed_path),
        VariableSourceType::FileCSV => extract_csv(&var.name, &var.value, allowed_path),
        VariableSourceType::ExternalData => {
            todo!("Add support for External Data")
        }
    }
}

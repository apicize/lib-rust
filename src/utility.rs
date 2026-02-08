//! Utility models submodule
//!
//! This submodule defines utility functions used for serialization and deserialization

use std::{fs::File, path::{Path, PathBuf}};

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{ApicizeError, ExecutionConcurrency};

/// Generate unique ID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Generate the value of Sequential execution for serde
pub fn sequential() -> ExecutionConcurrency {
    ExecutionConcurrency::Sequential
}

/// Convert a JSON value to an array of values for loop processing
pub fn convert_json(name: &str, value: &str) -> Result<Value, ApicizeError> {
    match serde_json::from_str::<Value>(value) {
        Ok(v) => Ok(v),
        Err(err) => Err(ApicizeError::from_serde(err, name.to_string())),
    }
}

/// Return serialized JSON after validating it
pub fn extract_json(
    name: &str,
    file_name: &str,
    allowed_path: &Option<PathBuf>,
) -> Result<Value, ApicizeError> {
    match get_existing_absolute_file_name(file_name, allowed_path) {
        Ok(full_file_name) => match File::open(full_file_name) {
            Ok(file) => match serde_json::from_reader::<File, Value>(file) {
                Ok(v) => Ok(v),
                Err(err) => Err(ApicizeError::from_serde(err, name.to_string())),
            },
            Err(err) => Err(ApicizeError::from_io(err, Some(file_name.to_string()))),
        },
        Err(err) => Err(err),
    }
}

/// Return serialized CSV data rendered as JSON
pub fn extract_csv(
    name: &str,
    file_name: &str,
    allowed_path: &Option<PathBuf>,
) -> Result<Value, ApicizeError> {
    match get_existing_absolute_file_name(file_name, allowed_path) {
        Ok(full_file_name) => match File::open(full_file_name.clone()) {
            Ok(file) => {
                let mut rdr = csv::Reader::from_reader(file);
                let mut data = Vec::<Value>::new();
                for record in rdr.deserialize::<Map<String, Value>>() {
                    match record {
                        Ok(r) => data.push(serde_json::Value::Object(r)),
                        Err(err) => return Err(ApicizeError::from_csv(err, name.to_string())),
                    }
                }
                Ok(serde_json::Value::Array(data))
            }
            Err(err) => Err(ApicizeError::from_io(
                err,
                Some(full_file_name.to_string_lossy().to_string()),
            )),
        },
        Err(err) => Err(err),
    }
}

/// Return the absolute file name, ensuring it exists and that it is form the same directory as our workbook
pub fn get_existing_absolute_file_name(
    file_name: &str,
    allowed_parent_directory: &Option<PathBuf>,
) -> Result<PathBuf, ApicizeError> {
    match allowed_parent_directory {
        Some(parent_data_path) => {
            let data_path = PathBuf::from(parent_data_path).join(file_name);
            if data_path.exists() {
                Ok(data_path)
            } else {
                Err(ApicizeError::FileAccess {
                    description: "Not found".to_string(),
                    file_name: Some(file_name.to_string()),
                })
            }
        }
        None => Err(ApicizeError::Error {
            description: "External data files are unavailable in an unsaved workbook".to_string(),
        }),
    }
}

/// Generate an absolute file name ensuring the directory exists
pub fn build_absolute_file_name(
    file_name: &str,
    directory: &Path,
) -> Result<PathBuf, ApicizeError> {
    let result = PathBuf::from(directory).join(file_name);
    if !directory.exists() {
        Err(ApicizeError::FileAccess {
            description: "Unable to create (invalid directory)".to_string(),
            file_name: Some(result.to_string_lossy().to_string()),
        })
    } else {
        Ok(result)
    }
}

/// Ensure that file_path is an absolute path and is a child of allowed_parent_path, returning the relative path
pub fn get_relative_file_name(
    file_path: &Path,
    parent_directory: &Path,
) -> Result<String, ApicizeError> {
    if !file_path.is_absolute() {
        return Err(ApicizeError::Error {
            description: format!(
                "File path '{}' is not an absolute path",
                file_path.display()
            ),
        });
    }

    match file_path.strip_prefix(parent_directory) {
        Ok(relative_path) => Ok(relative_path.to_string_lossy().to_string()),
        Err(_) => Err(ApicizeError::Error {
            description: format!(
                "File '{}' is not a child of '{}'",
                file_path.display(),
                parent_directory.display()
            ),
        }),
    }
}

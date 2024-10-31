//! Shared models submodule
//!
//! This submodule defines information used globally

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

use crate::{
    Persistence, Selection,
};

/// Trait to describe oneself
pub trait Identifable {
    /// Return ID of self
    fn get_id(&self) -> &String;

    /// Return name of self
    fn get_name(&self) -> &String; 

    /// Return a title to display in a list
    fn get_title(&self) -> String;
}

/// Trait to describe how an entity will be persisted
pub trait Persistable {
    /// Get persistence
    fn get_persistence(&self) -> Persistence;

    /// Set persistence
    fn set_persistence(&self, persistence_to_set: Persistence);
}

/// Types of selectable request/group options
pub enum SelectableOptionType {
    /// Selectable scenario
    Scenario,
    /// Selectdable authorization
    Authorization,
    /// Selectable certificate
    Certificate,
    /// Selectable proxy
    Proxy,
}

impl SelectableOptionType {
    /// Convert to readable string
    pub fn as_str(&self) -> &'static str {
        match self {
            SelectableOptionType::Scenario => "scenario",
            SelectableOptionType::Authorization => "authorization",
            SelectableOptionType::Certificate => "certificate",
            SelectableOptionType::Proxy => "proxy",
        }
    }
}

/// Whether a missing selectable option defaults to the parent or to None
pub enum SelectableOptionDefaultType {
    /// The request/group parent will be used as a default if no value is provided
    Parent,
    /// No default will be used if no value is provided
    None,
}

impl SelectableOptionDefaultType {
    /// Render default type as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            SelectableOptionDefaultType::Parent => "parent",
            SelectableOptionDefaultType::None => "none",
        }
    }
}

/// Trait indicating scenarios, authorizations, etc. can be
pub trait SelectableOptions {
    /// Get selected scenario, if any
    fn get_selected_scenario(&self) -> &Option<Selection>;

    /// Get selected authorization, if any
    fn get_selected_authorization(&self) -> &Option<Selection>;

    /// Get selected certificate, if any
    fn get_selected_certificate(&self) -> &Option<Selection>;

    /// Get selected proxy, if any
    fn get_selected_proxy(&self) -> &Option<Selection>;

    /// Set selected scenario, if any
    fn set_selected_scenario(&mut self, value: Option<Selection>);

    /// Set selected authorization, if any
    fn set_selected_authorization(&mut self, value: Option<Selection>);

    /// Set selected certificate, if any
    fn set_selected_certificate(&mut self, value: Option<Selection>);

    /// Set selected proxy, if any
    fn set_selected_proxy(&mut self, value: Option<Selection>);
}

/// Implement helpers for setting serde default values
impl Persistence {
    /// Return the private enum value for setting persistence defaults
    pub fn private() -> Self {
        Persistence::Private
    }
    /// Return the Shared enum value for setting persistence defaults
    pub fn global() -> Self {
        Persistence::Global
    }
}

/// Open the specified data file
pub fn open_data_file<T: DeserializeOwned>(
    input_file_name: &PathBuf,
) -> Result<SerializationOpenSuccess<T>, SerializationFailure> {
    let file_name = String::from(input_file_name.to_string_lossy());
    match std::fs::File::open(input_file_name) {
        Ok(mut f) => open_data_stream(file_name, &mut f),
        Err(err) => Err(SerializationFailure {
            file_name,
            error: SerializationError::IO(err),
        }),
    }
}

/// Open the specified data stream
pub fn open_data_stream<T: DeserializeOwned>(
    file_name: String,
    reader: &mut dyn Read,
) -> Result<SerializationOpenSuccess<T>, SerializationFailure> {
    let mut text = String::new();
    match reader.read_to_string(&mut text) {
        Ok(_) => match serde_json::from_str::<T>(&text) {
            Ok(data) => Ok(SerializationOpenSuccess { file_name, data }),
            Err(err) => Err(SerializationFailure {
                file_name,
                error: SerializationError::JSON(err),
            }),
        },
        Err(err) => Err(SerializationFailure {
            file_name,
            error: SerializationError::IO(err),
        }),
    }
}

/// Save the specified data file
pub fn save_data_file<T: Serialize>(
    output_file_name: &PathBuf,
    data: &T,
) -> Result<SerializationSaveSuccess, SerializationFailure> {
    let file_name = String::from(output_file_name.to_string_lossy());
    match serde_json::to_string(data) {
        Ok(text) => match fs::write(output_file_name, text) {
            Ok(()) => Ok(SerializationSaveSuccess {
                file_name,
                operation: SerializationOperation::Save,
            }),
            Err(err) => Err(SerializationFailure {
                file_name,
                error: SerializationError::IO(err),
            }),
        },
        Err(err) => Err(SerializationFailure {
            file_name,
            error: SerializationError::JSON(err),
        }),
    }
}

/// Delete the specified file, if it exists
pub fn delete_data_file(
    delete_file_name: &PathBuf,
) -> Result<SerializationSaveSuccess, SerializationFailure> {
    let file_name = String::from(delete_file_name.to_string_lossy());
    if Path::new(&delete_file_name).is_file() {
        match fs::remove_file(delete_file_name) {
            Ok(()) => Ok(SerializationSaveSuccess {
                file_name,
                operation: SerializationOperation::Delete,
            }),
            Err(err) => Err(SerializationFailure {
                file_name,
                error: SerializationError::IO(err),
            }),
        }
    } else {
        Ok(SerializationSaveSuccess {
            file_name,
            operation: SerializationOperation::None,
        })
    }
}

/// File operation
pub enum SerializationOperation {
    /// File saved
    Save,
    /// File deleted
    Delete,
    /// No operation taken
    None,
}

/// Information on open success, including data
pub struct SerializationOpenSuccess<T> {
    /// Name of file that was opened or saved
    pub file_name: String,
    /// Data
    pub data: T,
}

/// Information on save success
pub struct SerializationSaveSuccess {
    /// Name of file that was opened or saved
    pub file_name: String,
    /// File operation
    pub operation: SerializationOperation,
}

/// Information about I/O failure
pub struct SerializationFailure {
    /// Name of file that was opened or saved
    pub file_name: String,
    /// Error on serialization/deserialization
    pub error: SerializationError,
}

/// Represents errors occurring during Workbook serialization and deserialization
#[derive(Error, Debug)]
pub enum SerializationError {
    /// File system error
    #[error(transparent)]
    IO(#[from] io::Error),
    /// JSON parsing error
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
}

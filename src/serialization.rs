use serde::{de::DeserializeOwned, Serialize};
use serde_json::ser::PrettyFormatter;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use crate::ApicizeError;

/// Information on save success
/// Information on open success, including data
pub struct SerializationOpenSuccess<T> {
    /// Name of file that was opened or saved
    pub file_name: String,
    /// Data
    pub data: T,
}

pub struct SerializationSaveSuccess {
    /// Name of file that was opened or saved
    pub file_name: String,
    /// File operation
    pub operation: SerializationOperation,
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

/// Open the specified data file
pub fn open_data_file<T: DeserializeOwned>(
    input_file_name: &PathBuf,
) -> Result<SerializationOpenSuccess<T>, ApicizeError> {
    let file_name = String::from(input_file_name.to_string_lossy());
    let mut f = fs::File::open(input_file_name)
        .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
    open_data_stream(file_name, &mut f)
}

/// Open the specified data stream
pub fn open_data_stream<T: DeserializeOwned>(
    file_name: String,
    reader: &mut dyn Read,
) -> Result<SerializationOpenSuccess<T>, ApicizeError> {
    let mut text = String::new();
    reader.read_to_string(&mut text).map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
    match serde_json::from_str::<T>(&text) {
        Ok(data) => Ok(SerializationOpenSuccess { file_name, data }),
        Err(err) => Err(ApicizeError::from_serde(err, file_name))
    }
}

/// Save the specified data file
pub fn save_data_file<T: Serialize>(
    output_file_name: &PathBuf,
    data: &T,
) -> Result<SerializationSaveSuccess, ApicizeError> {
    let file_name = String::from(output_file_name.to_string_lossy());
    let formatter = PrettyFormatter::with_indent(b"    ");

    let writer = File::create(output_file_name)
        .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
    let mut ser = serde_json::Serializer::with_formatter(writer, formatter);
    data.serialize(&mut ser).map_err(|err| ApicizeError::from_serde(err, file_name.clone()))?;
    Ok(SerializationSaveSuccess {
        file_name,
        operation: SerializationOperation::Save,
    })
}

/// Delete the specified file, if it exists
pub fn delete_data_file(
    delete_file_name: &PathBuf,
) -> Result<SerializationSaveSuccess, ApicizeError> {
    let file_name = String::from(delete_file_name.to_string_lossy());
    if Path::new(&delete_file_name).is_file() {
        match fs::remove_file(delete_file_name) {
            Ok(()) => Ok(SerializationSaveSuccess {
                file_name,
                operation: SerializationOperation::Delete,
            }),
            Err(err) => Err(ApicizeError::FileAccess {
                file_name: Some(file_name),
                description: err.to_string(),
            }),
        }
    } else {
        Ok(SerializationSaveSuccess {
            file_name,
            operation: SerializationOperation::None,
        })
    }
}

pub trait PersistedIndex<T> {
    /// Retrieve parameters from workbook (if existing)
    fn get_workbook(&self) -> Option<Vec<T>>;

    /// Retrieve parameters from private workbook parameters file (if existing)
    fn get_private(&self) -> Option<Vec<T>>;

    /// Retrieve parameters from global vault (if existing)
    fn get_vault(&self) -> Option<Vec<T>>
    where
        Self: Sized;

    // Generate parameters from stored files
    fn new(workbook: Option<Vec<T>>, private: Option<Vec<T>>, vault: Option<Vec<T>>) -> Self
    where
        Self: Sized;
}

pub const PERSIST_WORKBOOK: &str = "W";
pub const PERSIST_PRIVATE: &str = "P";
pub const PERSIST_VAULT: &str = "V";

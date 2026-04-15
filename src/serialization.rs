use crate::ApicizeError;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::ser::PrettyFormatter;
use std::{
    fs,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

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
    reader
        .read_to_string(&mut text)
        .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
    match serde_json::from_str::<T>(&text) {
        Ok(data) => Ok(SerializationOpenSuccess { file_name, data }),
        Err(err) => Err(ApicizeError::from_serde(err, file_name)),
    }
}

/// Save the specified data file atomically.
///
/// Writes to a temporary file in the same directory, flushes, fsyncs, then
/// atomically renames over the target. If any step fails, the original file
/// is left untouched.
pub fn save_data_file<T: Serialize>(
    output_file_name: &PathBuf,
    data: &T,
) -> Result<SerializationSaveSuccess, ApicizeError> {
    let file_name = String::from(output_file_name.to_string_lossy());
    let dir = output_file_name
        .parent()
        .ok_or_else(|| ApicizeError::FileAccess {
            file_name: Some(file_name.clone()),
            description: "Unable to determine parent directory".to_string(),
        })?;

    let tmp = NamedTempFile::new_in(dir)
        .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;

    let formatter = PrettyFormatter::with_indent(b"    ");
    {
        let mut writer = BufWriter::new(tmp.as_file());
        let mut ser = serde_json::Serializer::with_formatter(&mut writer, formatter);
        data.serialize(&mut ser)
            .map_err(|err| ApicizeError::from_serde(err, file_name.clone()))?;
        writer
            .flush()
            .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
    }

    tmp.persist(output_file_name)
        .map_err(|err| ApicizeError::FileAccess {
            file_name: Some(file_name.clone()),
            description: format!("Failed to persist temp file: {}", err),
        })?;

    Ok(SerializationSaveSuccess {
        file_name,
        operation: SerializationOperation::Save,
    })
}

/// Save data to a file atomically.
///
/// Writes content to a temporary file, flushes, fsyncs, then atomically
/// renames over the target.
pub fn save_file_atomically(output_file_name: &PathBuf, content: &str) -> Result<(), ApicizeError> {
    let file_name = String::from(output_file_name.to_string_lossy());
    let dir = output_file_name
        .parent()
        .ok_or_else(|| ApicizeError::FileAccess {
            file_name: Some(file_name.clone()),
            description: "Unable to determine parent directory".to_string(),
        })?;

    let tmp = NamedTempFile::new_in(dir)
        .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;

    {
        let mut writer = BufWriter::new(tmp.as_file());
        writer
            .write_all(content.as_bytes())
            .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
        writer
            .flush()
            .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|err| ApicizeError::from_io(err, Some(file_name.clone())))?;
    }

    tmp.persist(output_file_name)
        .map_err(|err| ApicizeError::FileAccess {
            file_name: Some(file_name.clone()),
            description: format!("Failed to persist temp file: {}", err),
        })?;

    Ok(())
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

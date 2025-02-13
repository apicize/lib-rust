use serde::{de::DeserializeOwned, Serialize};
use serde_json::ser::PrettyFormatter;
use thiserror::Error;
use std::{fs::{self, File}, io::{self, Read}, path::{Path, PathBuf}};

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

/// Information about I/O failure
pub struct SerializationFailure {
    /// Name of file that was opened or saved
    pub file_name: String,
    /// Error on serialization/deserialization
    pub error: SerializationError,
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
    let formatter = PrettyFormatter::with_indent(b"    ");

    match File::create(output_file_name) {
        Ok(writer) => {
            let mut ser = serde_json::Serializer::with_formatter(writer, formatter);
            match data.serialize(&mut ser) {
                Ok(()) => Ok(SerializationSaveSuccess {
                    file_name,
                    operation: SerializationOperation::Save,
                }),
                Err(err) => Err(SerializationFailure {
                    file_name,
                    error: SerializationError::JSON(err),
                }),
            }
        },
        Err(err) => Err(SerializationFailure {
            file_name,
            error: SerializationError::IO(err),
        })
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

pub trait PersistedIndex<T> /*where T: Sized*/ {
    /// Retrieve parameters from workbook (if existing)
    fn get_workbook(&self) -> Option<Vec<T>>;

    /// Retrieve parameters from private workbook parameters file (if existing)
    fn get_private(&self) -> Option<Vec<T>>;
    
    /// Retrieve parameters from global vault (if existing)
    fn get_vault(&self) -> Option<Vec<T>> where Self: Sized;
    
    // Generate parameters from stored files
    fn new(
        workbook: Option<&[T]>,
        private: Option<&[T]>,
        vault: Option<&[T]>,
    ) -> Self where Self: Sized;
}

pub const PERSIST_WORKBOOK: &str = "W";
pub const PERSIST_PRIVATE: &str = "P";
pub const PERSIST_VAULT: &str = "V";
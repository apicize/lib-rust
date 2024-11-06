//! Apicize execution error types
use std::error::Error;

use oauth2::basic::BasicErrorResponseType;
use oauth2::{RequestTokenError, StandardErrorResponse};
use serde::ser::SerializeMap;
use serde::Serialize;
use thiserror::Error;
use tokio::task::JoinError;

/// Represents errors occuring during Workbook running, dispatching and testing
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// HTTP errors
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// Join/async errors
    #[error(transparent)]
    Join(#[from] JoinError),
    /// OAuth2 authentication errors
    #[error(transparent)]
    OAuth2(
        #[from]
        RequestTokenError<
            oauth2::HttpClientError<oauth2::reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),
    /// Failed test execution
    #[error("{0}")]
    FailedTest(String),
}

impl Serialize for ExecutionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut map = serializer.serialize_map(None)?;
        match &self {
            ExecutionError::Reqwest(error) => {
                map.serialize_entry("errorType", "HttpError")?;
                map.serialize_entry("errorDescription", error.to_string().as_str())?;
                if let Some(source) = error.source() {
                    map.serialize_entry("errorSource", source.to_string().as_str())?;
                }
            },
            ExecutionError::Join(error) => {
                map.serialize_entry("errorType", "AsyncError")?;
                map.serialize_entry("errorDescription", error.to_string().as_str())?;
                if let Some(source) = error.source() {
                    map.serialize_entry("errorSource", source.to_string().as_str())?;
                }
            },
            ExecutionError::OAuth2(error) => {
                map.serialize_entry("errorType", "OAuth2Error")?;
                map.serialize_entry("errorDescription", error.to_string().as_str())?;
                if let Some(source) = error.source() {
                    map.serialize_entry("errorSource", source.to_string().as_str())?;
                }
            },
            ExecutionError::FailedTest(error) => {
                map.serialize_entry("errorType", "TestError")?;
                map.serialize_entry("errorDescription", error.to_string().as_str())?;
            },
        }
        map.end()
    }
}

/// Represents errors occuring during Workbook running, dispatching and testing
#[derive(Error, Debug)]
pub enum RunError {
    /// Other error
    #[error("Other")]
    Other(String),
    /// Join error
    #[error("JoinError")]
    JoinError(JoinError),
    /// Execution cancelled
    #[error("Cancelled")]
    Cancelled,
}

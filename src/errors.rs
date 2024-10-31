//! Apicize execution error types

use oauth2::basic::BasicErrorResponseType;
use oauth2::{RequestTokenError, StandardErrorResponse};
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

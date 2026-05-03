//! Apicize execution error types
use oauth2::basic::BasicErrorResponseType;
use oauth2::{RequestTokenError, StandardErrorResponse};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::io;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Serialize, Deserialize, Clone, PartialEq, Error, Debug)]
#[serde(tag = "type")]
/// Errors that can result from Apicize operations
pub enum ApicizeError {
    Error {
        description: String,
    },

    Encryption {
        description: String,
    },

    Http {
        context: Option<String>,
        description: String,
        url: Option<String>,
    },

    FileAccess {
        description: String,
        file_name: Option<String>,
    },

    Timeout {
        url: Option<String>,
    },

    Serialization {
        description: String,
        name: String,
    },

    Cancelled,

    OAuth2Client {
        description: String,
        context: Option<String>,
    },

    Async {
        description: String,
        id: String,
    },

    InvalidId {
        description: String,
    },

    FailedTest {
        description: String,
    },
    Csv {
        description: String,
    },
}

impl Display for ApicizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApicizeError::Error { description } => {
                write!(f, "{description}")
            }
            ApicizeError::Encryption { description } => {
                write!(f, "{description}")
            }
            ApicizeError::Http {
                context,
                description,
                url,
            } => {
                let suffix = match url {
                    Some(u) => format!(" ({u})"),
                    None => String::default(),
                };

                match context {
                    Some(s) => {
                        write!(f, "{s} - {description}{suffix}")
                    }
                    None => {
                        write!(f, "{description}{suffix}")
                    }
                }
            }
            ApicizeError::FileAccess {
                description,
                file_name,
            } => match file_name {
                Some(s) => {
                    write!(f, "{s} - {description}")
                }
                None => {
                    write!(f, "{description}")
                }
            },
            ApicizeError::Timeout { url } => match url {
                Some(s) => {
                    write!(f, "Timeout - {s}")
                }
                None => {
                    write!(f, "Timeout")
                }
            },
            ApicizeError::Serialization {
                description,
                name: file_name,
            } => {
                write!(f, "{description} - {file_name}")
            }
            ApicizeError::Cancelled => write!(f, "Cancelled"),
            ApicizeError::OAuth2Client {
                description,
                context,
            } => match context {
                Some(s) => {
                    write!(f, "OAuth2 error {s} - {description}")
                }
                None => {
                    write!(f, "OAuth2 error - {description}")
                }
            },
            ApicizeError::Async { description, id } => {
                write!(f, "Async error - {description} ({id})")
            }
            ApicizeError::InvalidId { description } => write!(f, "Invalid ID - {description}"),
            ApicizeError::FailedTest { description } => write!(f, "Failed test - {description}"),
            ApicizeError::Csv { description } => write!(f, "CSV Error - {description}"),
        }
    }
}

impl From<reqwest::Error> for ApicizeError {
    fn from(error: reqwest::Error) -> Self {
        Self::from_reqwest(error, None)
    }
}

impl
    From<
        RequestTokenError<
            oauth2::HttpClientError<oauth2::reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    > for ApicizeError
{
    fn from(
        error: RequestTokenError<
            oauth2::HttpClientError<oauth2::reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ) -> Self {
        Self::from_oauth2(error, None)
    }
}

impl From<csv::Error> for ApicizeError {
    fn from(err: csv::Error) -> Self {
        Self::Csv {
            description: format!("{}", &err),
        }
    }
}

impl ApicizeError {
    pub fn from_reqwest(error: reqwest::Error, context: Option<String>) -> ApicizeError {
        if error.is_timeout() {
            ApicizeError::Timeout {
                url: error.url().map(|url| url.to_string()),
            }
        } else {
            ApicizeError::Http {
                description: error.to_string(),
                url: error.url().map(|url| url.to_string()),
                context,
            }
        }
    }

    pub fn from_oauth2(
        error: RequestTokenError<
            oauth2::HttpClientError<oauth2::reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
        context: Option<String>,
    ) -> ApicizeError {
        ApicizeError::OAuth2Client {
            description: error.to_string(),
            context,
        }
    }

    pub fn from_async(error: JoinError) -> ApicizeError {
        ApicizeError::Async {
            id: error.id().to_string(),
            description: error.to_string(),
        }
    }

    pub fn from_serde(error: serde_json::Error, name: String) -> ApicizeError {
        ApicizeError::Serialization {
            description: format!("{error}"),
            name,
        }
    }

    pub fn from_csv(error: csv::Error, name: String) -> ApicizeError {
        ApicizeError::Serialization {
            description: format!("{error}"),
            name,
        }
    }

    pub fn from_io(error: io::Error, file_name: Option<String>) -> ApicizeError {
        ApicizeError::FileAccess {
            description: error.to_string(),
            file_name,
        }
    }

    pub fn from_failed_test(description: String) -> ApicizeError {
        ApicizeError::FailedTest { description }
    }

    pub fn get_label(&self) -> &str {
        match &self {
            ApicizeError::Error { .. } => "Error",
            ApicizeError::Encryption { .. } => "Encryption",
            ApicizeError::Http { .. } => "HTTP Error",
            ApicizeError::Timeout { .. } => "HTTP Timeout",
            ApicizeError::Cancelled => "Cancelled",
            ApicizeError::OAuth2Client { .. } => "OAuth2 Token Error",
            ApicizeError::Async { .. } => "Task Error",
            ApicizeError::FailedTest { .. } => "Failed Test",
            ApicizeError::FileAccess { .. } => "File IO",
            ApicizeError::Serialization { .. } => "Failed Serialization/Deserialization",
            ApicizeError::InvalidId { .. } => "Invalid ID",
            ApicizeError::Csv { .. } => "CSV Error",
        }
    }
}

//! Apicize execution error types
use std::error::Error;
use std::fmt::Display;
use std::io;

use oauth2::basic::BasicErrorResponseType;
use oauth2::{RequestTokenError, StandardErrorResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
/// Errors that can result from Apicize operations
pub enum ApicizeError {
    Error {
        description: String,
        source: Option<Box<ApicizeError>>,
    },
    Http {
        description: String,
        source: Option<Box<ApicizeError>>,
        url: Option<String>,
    },
    Timeout {
        description: String,
        source: Option<Box<ApicizeError>>,
        url: Option<String>,
    },
    Cancelled {
        description: String,
        source: Option<Box<ApicizeError>>,
    },
    OAuth2Client {
        description: String,
        source: Option<Box<ApicizeError>>,
    },
    Async {
        id: String,
        description: String,
        source: Option<Box<ApicizeError>>,
    },
    IO {
        description: String,
    },
    Parse {
        description: String,
        name: String,
    },
    FailedTest {
        description: String,
    },
}

impl ApicizeError {
    fn from_error(error: &dyn Error) -> ApicizeError {
        ApicizeError::Error {
            description: error.to_string(),
            source: error
                .source()
                .map(|src| Box::new(ApicizeError::from_error(src))),
        }
    }

    pub fn from_reqwest(error: reqwest::Error) -> ApicizeError {
        if error.is_timeout() {
            ApicizeError::Timeout {
                description: String::from("Timeout"),
                source: error
                    .source()
                    .map(|src| Box::new(ApicizeError::from_error(src))),
                url: error.url().map(|url| url.to_string()),
            }
        } else if error.is_timeout() {
            ApicizeError::Cancelled {
                description: String::from("Cancelled"),
                source: error
                    .source()
                    .map(|src| Box::new(ApicizeError::from_error(src))),
            }
        } else {
            ApicizeError::Http {
                description: error.to_string(),
                source: error
                    .source()
                    .map(|src| Box::new(ApicizeError::from_error(src))),
                url: error.url().map(|url| url.to_string()),
            }
        }
    }

    pub fn from_oauth2(
        error: RequestTokenError<
            oauth2::HttpClientError<oauth2::reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ) -> ApicizeError {
        ApicizeError::Error {
            description: error.to_string(),
            source: error
                .source()
                .map(|src| Box::new(ApicizeError::from_error(src))),
        }
    }

    pub fn from_async(error: JoinError) -> ApicizeError {
        ApicizeError::Async {
            id: error.id().to_string(),
            description: error.to_string(),
            source: error
                .source()
                .map(|src| Box::new(ApicizeError::from_error(src))),
        }
    }

    pub fn from_serde(error: serde_json::Error, name: &str) -> ApicizeError {
        ApicizeError::Parse {
            description: format!("{}", error),
            name: name.to_string(),
        }
    }

    pub fn from_csv(error: csv::Error, name: &str) -> ApicizeError {
        ApicizeError::Parse {
            description: format!("{}", error),
            name: name.to_string(),
        }
    }

    pub fn from_io(error: io::Error) -> ApicizeError {
        ApicizeError::IO { 
            description: format!("{}", error)
        }
    }

    pub fn from_failed_test(description: String) -> ApicizeError {
        ApicizeError::FailedTest { description }
    }

    pub fn get_label(&self) -> &str {
        match &self {
            ApicizeError::Error { .. } => "Error",
            ApicizeError::Http { .. } => "HTTP Error",
            ApicizeError::Timeout { .. } => "HTTP Timeout",
            ApicizeError::Cancelled { .. } => "Cancelled",
            ApicizeError::OAuth2Client { .. } => "OAuth2 Token Error",
            ApicizeError::Async { .. } => "Task Error",
            ApicizeError::FailedTest { .. } => "Failed Test",
            ApicizeError::IO { .. } => "I/O",
            ApicizeError::Parse { .. } => "Failed Parsing",
        }
    }
}

fn format_child_description(
    parent_description: &str,
    child: Option<&dyn Error>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match child {
        Some(c) => {
            let child_desc = c.to_string();
            if parent_description.ends_with(&child_desc) {
                Ok(())
            } else {
                f.write_str(format!(", {}", &child_desc).as_str())
                    .and_then(|()| format_child_description(&child_desc, c.source(), f))
            }
        }
        None => Ok(()),
    }
}

impl Display for ApicizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc: &String;
        let suffix: Option<String>;
        match &self {
            ApicizeError::Error { description, .. } => {
                suffix = None;
                desc = description;
            }
            ApicizeError::Http { description, .. } => {
                suffix = None;
                desc = description;
            }
            ApicizeError::Timeout {
                description, url, ..
            } => {
                suffix = url
                    .as_ref()
                    .map_or_else(|| None, |u| Some(format!("calling {}", u)));
                desc = description;
            }
            ApicizeError::Cancelled { description, .. } => {
                suffix = None;
                desc = description;
            }
            ApicizeError::OAuth2Client { description, .. } => {
                suffix = None;
                desc = description;
            }
            ApicizeError::Async {
                description, id, ..
            } => {
                suffix = Some(format!("(task {})", id));
                desc = description;
            }
            ApicizeError::IO { description, .. } => {
                suffix = None;
                desc = description;
            }
            ApicizeError::Parse { description, name } => {
                suffix = Some(format!("(value \"{}\"", name));
                desc = description;
            }
            ApicizeError::FailedTest { description, .. } => {
                suffix = None;
                desc = description;
            }
        }

        let result = if let Some(sfx) = suffix {
            f.write_str(format!("{}, {}", desc, sfx,).as_str())
        } else {
            f.write_str(desc)
        };

        result.and_then(|()| format_child_description(desc, self.source(), f))
    }
}

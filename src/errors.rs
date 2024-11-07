//! Apicize execution error types
use std::error::Error;
use std::fmt::Display;

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
        }
    }
}

fn format_child_description(parent_description: &str, child: Option<&dyn Error>) -> Option<String> {
    match child {
        Some(c) => {
            let child_desc = c.to_string();
            parent_description.ends_with(&child_desc).then(|| {
                if let Some(grandchild_desc) = format_child_description(&child_desc, c.source()) {
                    format!(", {}{}", child_desc, grandchild_desc)
                } else {
                    format!(", {}", child_desc)
                }
            })
        }
        None => None,
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
            ApicizeError::Http {
                description, url, ..
            } => {
                suffix = url
                    .as_ref()
                    .map_or_else(|| None, |u| Some(format!("calling {}", u)));
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
            ApicizeError::FailedTest { description, .. } => {
                suffix = None;
                desc = description;
            }
        }

        let source = self.source();
        let mut s = if let Some(sfx) = suffix {
            format!("{}, {}", desc, sfx,)
        } else {
            desc.to_owned()
        };
        if let Some(s1) = format_child_description(desc, source) {
            s = s + &s1;
        }
        f.write_str(s.as_str())
    }
}

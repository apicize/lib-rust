//! Apicize execution error types
use std::fmt::Display;
use std::io;

use oauth2::basic::BasicErrorResponseType;
use oauth2::{RequestTokenError, StandardErrorResponse};
use serde::{Deserialize, Serialize};
use tokio::task::JoinError;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
/// Errors that can result from Apicize operations
pub enum ApicizeError {
    Error {
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
}

impl Display for ApicizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApicizeError::Error { description } => {
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
                    write!(f, "OAuth 2 error - {description}")
                }
            },
            ApicizeError::Async { description, id } => {
                write!(f, "Async error - {description} ({id})")
            }
            ApicizeError::InvalidId { description } => write!(f, "Invalid ID - {description}"),
            ApicizeError::FailedTest { description } => write!(f, "Failed test - {description}"),
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
            ApicizeError::Http { .. } => "HTTP Error",
            ApicizeError::Timeout { .. } => "HTTP Timeout",
            ApicizeError::Cancelled => "Cancelled",
            ApicizeError::OAuth2Client { .. } => "OAuth2 Token Error",
            ApicizeError::Async { .. } => "Task Error",
            ApicizeError::FailedTest { .. } => "Failed Test",
            ApicizeError::FileAccess { .. } => "File IO",
            ApicizeError::Serialization { .. } => "Failed Serialization/Deserialization",
            ApicizeError::InvalidId { .. } => "Invalid ID",
        }
    }
}

// fn format_child_description(
//     parent_description: &str,
//     child: Option<&dyn Error>,
//     f: &mut std::fmt::Formatter<'_>,
// ) -> std::fmt::Result {
//     match child {
//         Some(c) => {
//             let child_desc = c.to_string();
//             if parent_description.ends_with(&child_desc) {
//                 Ok(())
//             } else {
//                 f.write_str(format!(", {}", &child_desc).as_str())
//                     .and_then(|()| format_child_description(&child_desc, c.source(), f))
//             }
//         }
//         None => Ok(()),
//     }
// }

// impl Display for ApicizeError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let desc: &String;
//         let suffix: Option<String>;
//         match &self {
//             ApicizeError::Error { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//             ApicizeError::Http { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//             ApicizeError::Timeout {
//                 description, url, ..
//             } => {
//                 suffix = url
//                     .as_ref()
//                     .map_or_else(|| None, |u| Some(format!("calling {}", u)));
//                 desc = description;
//             }
//             ApicizeError::Cancelled { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//             ApicizeError::OAuth2Client { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//             ApicizeError::Async {
//                 description, id, ..
//             } => {
//                 suffix = Some(format!("(task {})", id));
//                 desc = description;
//             }
//             ApicizeError::IO { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//             ApicizeError::Parse { description, name } => {
//                 suffix = Some(format!("(value \"{}\"", name));
//                 desc = description;
//             }
//             ApicizeError::FailedTest { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//             ApicizeError::InvalidId { description, .. } => {
//                 suffix = None;
//                 desc = description;
//             }
//         }

//         let result = if let Some(sfx) = suffix {
//             f.write_str(format!("{}, {}", desc, sfx,).as_str())
//         } else {
//             f.write_str(desc)
//         };

//         result.and_then(|()| format_child_description(desc, self.source(), f))
//     }
// }

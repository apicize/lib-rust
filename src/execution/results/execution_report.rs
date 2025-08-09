use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{ApicizeError, ApicizeTestBehavior};

use super::ExecutionResultSuccess;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExecutionReportFormat {
    #[default]
    JSON,
    CSV,
}

impl Display for ExecutionReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionReportFormat::JSON => f.write_str("JSON"),
            ExecutionReportFormat::CSV => f.write_str("CSV"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReportJson {
    /// Fully qualified request name
    pub name: String,

    /// Optional referential key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Associative tag name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,

    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Ordinal run number, if mult-run result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_number: Option<usize>,

    /// Ordinal run count, if mult-run result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_count: Option<usize>,

    /// Ordinal row number, if mult-row result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_number: Option<usize>,

    /// Ordinal row count, if multi-row result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<usize>,

    /// Method for request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// URL for request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Whether
    pub success: ExecutionResultSuccess,

    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,

    /// HTTP status text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,

    /// Error on dispatch or error execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApicizeError>,

    /// Request test results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_results: Option<Vec<ApicizeTestBehavior>>,

    /// Child groups and requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ExecutionReportJson>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReportCsv {
    // /// Set to run number if a multi-run execution from CLI tool
    // #[serde(rename = "Run #")]
    // pub run_number: Option<usize>,

    // /// Set to run number if a multi-run execution from CLI tool
    // #[serde(rename = "Row #")]
    // pub row_number: Option<usize>,
    /// Fully qualified request name
    #[serde(rename = "Name")]
    pub name: String,

    /// Optional referential key
    #[serde(rename = "Key")]
    pub key: Option<String>,

    /// Execution start (millisecond offset from start)
    #[serde(rename = "Executed At")]
    pub executed_at: u128,

    /// Duration of execution (milliseconds)
    #[serde(rename = "Duration")]
    pub duration: u128,

    /// Method for request
    #[serde(rename = "Method")]
    pub method: Option<String>,

    /// URL for request
    #[serde(rename = "URL")]
    pub url: Option<String>,

    /// Whether the request executed and tests succeeded
    #[serde(rename = "Success")]
    pub success: ExecutionResultSuccess,

    /// HTTP status code
    #[serde(rename = "Status")]
    pub status: Option<u16>,

    /// HTTP status text
    #[serde(rename = "Status Text")]
    pub status_text: Option<String>,

    /// Human readable name of the test
    #[serde(rename = "Test Name")]
    pub test_name: Option<String>,

    /// Associative tag name
    #[serde(rename = "Test Tag")]
    pub test_tag: Option<String>,

    /// Whether or not the test executed and passed successful
    #[serde(rename = "Test Success")]
    pub test_success: Option<bool>,

    /// Console I/O generated during the test
    #[serde(rename = "Test Logs")]
    pub test_logs: Option<String>,

    /// Error on dispatch or error execution
    #[serde(rename = "Error")]
    pub error: Option<ApicizeError>,

    /// Error generated during the test
    #[serde(rename = "Test Error")]
    pub test_error: Option<String>,
}

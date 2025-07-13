use serde::{Deserialize, Serialize};

use crate::{ApicizeError, ApicizeTestBehavior};

use super::execution_result_success::ExecutionResultSuccess;

/// Summary information about a request or group execution used for menus and summaries
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResultSummary {
    /// Request or group ID
    pub request_or_group_id: String,

    /// Ordinal position of this result in the response
    pub index: usize,

    /// Index of parent result, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_index: Option<usize>,

    /// Indexes of child results, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_indexes: Option<Vec<usize>>,

    /// Indentation level
    pub level: usize,

    /// Name of request or group
    pub name: String,

    /// Associative tag name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// Method for request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// URL requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,

    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,

    /// HTTP status text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,

    /// If true, this is for a request and headers were returned
    pub has_response_headers: bool,

    /// Used to indicate the length of a response body, if any
    pub response_body_length: Option<usize>,

    /// Indicates level of call success
    pub success: ExecutionResultSuccess,

    // Indicates an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApicizeError>,

    /// Executed test results, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_results: Option<Vec<ApicizeTestBehavior>>,

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
}

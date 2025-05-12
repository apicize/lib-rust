use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{ApicizeError, ApicizeHttpRequest, ApicizeHttpResponse, ApicizeTestResult};

use super::execution_result_success::ExecutionResultSuccess;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", tag = "entityType" )]
pub enum ExecutionResultDetail {
    Request(ExecutionResultDetailRequest),
    Grouped(Box<ExecutionResultDetailGroup>),
}

/// Represents detailed execution information of a request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResultDetailRequest {
    /// Request ID
    pub id: String,

    /// Request name
    pub name: String,

    /// Row number (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_number: Option<usize>,

    // Run number (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_number: Option<usize>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,

    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Variables assigned to the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Map<String, Value>>,

    /// Row data assigned to the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<String, Value>>,

    /// Variables to update at the end of the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_variables: Option<Map<String, Value>>,

    /// Request sent to server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<ApicizeHttpRequest>,

    /// Response received from server (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ApicizeHttpResponse>,

    /// Test results (if executed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<ApicizeTestResult>>,

    /// Error on dispatch or error execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApicizeError>,

    /// Indicates level of call success
    pub success: ExecutionResultSuccess,

    /// Number of child requests/groups with successful requests and all tests passed
    pub request_success_count: usize,

    /// Number of child requests/groups with successful requests and some tests failed
    pub request_failure_count: usize,

    /// Number of child requests/groups with successful requests and some tests failed
    pub request_error_count: usize,

    /// Number of passed tests, if request and tests are succesfully run
    pub test_pass_count: usize,

    /// Number of failed tests, if request and tests are succesfully run
    pub test_fail_count: usize,
}

/// Represents detailed execution information of a request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResultDetailGroup {
    /// Request ID
    pub id: String,

    /// Request name
    pub name: String,

    /// Row number (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_number: Option<usize>,

    // Run number (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_number: Option<usize>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,

    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Variables to update at the end of the grou's requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_variables: Option<Map<String, Value>>,

    /// Success is true if all runs are successful
    pub success: ExecutionResultSuccess,

    /// Number of child requests/groups with successful requests and all tests passed
    pub request_success_count: usize,

    /// Number of child requests/groups with successful requests and some tests failed
    pub request_failure_count: usize,

    /// Number of child requests/groups with successful requests and some tests failed
    pub request_error_count: usize,

    /// Number of passed tests, if request and tests are succesfully run
    pub test_pass_count: usize,

    /// Number of failed tests, if request and tests are succesfully run
    pub test_fail_count: usize,
}

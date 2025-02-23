//! Apicize models submodule
//!
//! This submodule defines models used to execute Apicize tests and report their results

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

use crate::ApicizeError;

use super::oauth2_client_tokens::TokenResult;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeItem {
    Group(ApicizeSummary),
    Request(ApicizeSummary),
    Items(Vec<Box<ApicizeItem>>),
    ExecutionSummaries(Vec<Box<ApicizeExecutionSummary>>),
    ExecutedRequest(Box<ApicizeRequestWithExecution>),
    Execution(Box<ApicizeExecution>),
}

impl ApicizeItem {
    pub fn get_output_variables(&self) -> Option<Map<String, Value>> {
        match self {
            ApicizeItem::Group(g) => g.output_variables.clone(),
            ApicizeItem::Request(r) => r.output_variables.clone(),
            ApicizeItem::ExecutedRequest(e) => e.output_variables.clone(),
            ApicizeItem::Execution(e) => match e {
                ApicizeExecution::Rows(items) => {
                    items.last().map_or(None, |i| i.output_variables.clone())
                }
                ApicizeExecution::Runs(items) => {
                    items.last().map_or(None, |d| d.output_variables.clone())
                }
                ApicizeExecution::Details(apicize_items) => todo!(),
            },
            ApicizeItem::Items(apicize_items) => todo!(),
            ApicizeItem::ExecutionSummaries(items) => todo!(),
        }
    }
}

/// A summary of a request, group or executions
#[derive(Serialize, Deserialize, PartialEq, Clone)]

#[serde(rename_all = "camelCase")]
pub struct ApicizeSummary {
    /// Request ID
    pub id: String,
    /// Request name
    pub name: String,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    // /// Variables assigned to the group
    // pub input_variables: Option<HashMap<String, Value>>,
    // /// Row data assigned to the group
    // pub data: Option<HashMap<String, Value>>,
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    pub children: Option<ApicizeItem>,

    /// Success is true if all runs are successful
    pub success: bool,
    /// Number of child requests/groups with successful requests and all tests passed
    pub requests_with_passed_tests_count: usize,
    /// Number of child requests/groups with successful requests and some tests failed
    pub requests_with_failed_tests_count: usize,
    /// Number of child requests/groups with errors executing requests and/or tests
    pub requests_with_errors: usize,
    /// Number of passed tests, if request and tests are succesfully run
    pub passed_test_count: usize,
    /// Number of failed tests, if request and tests are succesfully run
    pub failed_test_count: usize,
}

/// Information regarding a request that was executed once
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRequestWithExecution {
    /// Request ID
    pub id: String,
    /// Request name
    pub name: String,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)/// Information regarding execution of an Apicize request
    pub duration: u128,

    /// Index of data row (if applicable)
    pub row_number: Option<usize>,

    /// Variables included from scenario or previous call
    pub input_variables: Option<Map<String, Value>>,
    /// Row data assigned to the group
    pub data: Option<Map<String, Value>>,
    /// Variables to set on the next request
    pub output_variables: Option<Map<String, Value>>,

    /// URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTTP Method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ApicizeBody>,

    /// Response received from server (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ApicizeDispatchResponse>,

    /// Test results (if executed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<ApicizeTestResult>>,

    /// Error on dispatch or error execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApicizeError>,

    /// Success is true if request and tests were successful
    pub success: bool,
    /// Number of passed tests, if request and tests are succesfully run
    pub passed_test_count: usize,
    /// Number of failed tests, if request and tests are succesfully run
    pub failed_test_count: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum ApicizeExecution {
    Details(Vec<ApicizeItem>),
    Rows(Vec<ApicizeExecutionSummary>),
    Runs(Vec<ApicizeExecutionDetail>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// Request execution summary with multiple rows or runs
pub struct ApicizeExecutionSummary {
    /// Index of the run number
    pub run_number: Option<usize>,
    /// Index of the row when executing with a external data set
    pub row_number: Option<usize>,

    /// Child executions (multiple runs of a row)
    pub children: Option<ApicizeExecution>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    // /// Variables assigned to the group
    // pub input_variables: Option<Map<String, Value>>,
    // /// Row data assigned to the group
    // pub data: Option<Map<String, Value>>,
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    /// Success is true if all runs are successful
    pub success: bool,
    /// Number of child requests/groups with successful requests and all tests passed
    pub requests_with_passed_tests_count: usize,
    /// Number of child requests/groups with successful requests and some tests failed
    pub requests_with_failed_tests_count: usize,
    /// Number of child requests/groups with errors executing requests and/or tests
    pub requests_with_errors: usize,
    /// Number of passed tests, if request and tests are succesfully run
    pub passed_test_count: usize,
    /// Number of failed tests, if request and tests are succesfully run
    pub failed_test_count: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// Request execution results for a request
pub struct ApicizeExecutionDetail {
    /// Index of run for a mult-run execution
    pub run_number: Option<usize>,
    /// Index of the row when executing with a external data set
    pub row_number: Option<usize>,
    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    /// Variables assigned to the group
    pub input_variables: Option<Map<String, Value>>,
    /// Row data assigned to the group
    pub data: Option<Map<String, Value>>,
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    /// URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTTP Method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ApicizeBody>,

    /// Response received from server (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ApicizeDispatchResponse>,

    /// Test results (if executed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<ApicizeTestResult>>,

    /// Error on dispatch or error execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApicizeError>,

    /// Success is true if all runs are successful
    pub success: bool,
    /// Number of passed tests, if request and tests are succesfully run
    pub passed_test_count: usize,
    /// Number of failed tests, if request and tests are succesfully run
    pub failed_test_count: usize,
}

/// Information used to dispatch an Apicize request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeDispatchRequest {
    /// URL
    pub url: String,
    /// HTTP Method
    pub method: String,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ApicizeBody>,
    /// Variables passed into request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Map<String, Value>>,
}

/// Information about the response to a dispatched Apicize request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeDispatchResponse {
    /// HTTP status code
    pub status: u16,
    /// HTTP status text
    pub status_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Response headers
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Response body
    pub body: Option<ApicizeBody>,
    /// Set to OAuth2 token result information
    pub oauth2_token: Option<TokenResult>,
}

/// Body information used when dispatching an Apicize Request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeBody {
    /// Body as data (UTF-8 bytes)
    #[serde_as(as = "Option<Base64<Standard, Unpadded>>")]
    pub data: Option<Vec<u8>>,
    /// Reprsents body as text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Response from V8 when executing a request's tests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeTestResponse {
    /// Results of test
    pub results: Option<Vec<ApicizeTestResult>>,
    /// Scenario values (if any)
    pub variables: Map<String, Value>,
}

/// Test execution results
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeTestResult {
    /// Human readable name of the test
    pub test_name: Vec<String>,
    /// Whether or not the test was successful
    pub success: bool,
    /// Error generated during the test
    pub error: Option<String>,
    /// Console I/O generated during the test
    pub logs: Option<Vec<String>>,
}

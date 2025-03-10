//! Apicize models submodule
//!
//! This submodule defines models used to execute Apicize tests and report their results

use super::ApicizeExecution;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeResult {
    Items(ApicizeList<ApicizeGroupItem>),
    Rows(ApicizeRowSummary),
}

/// Summary of rows executed for an external data set
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRowSummary {
    /// Rows executed
    pub rows: Vec<ApicizeRow>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Success is true if all runs are successful
    pub success: bool,
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

/// Summary of rows executed for an external data set
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRow {
    /// Row number (if multi-row result)
    pub row_number: usize,

    /// Groups or requests that were executed
    pub items: Vec<ApicizeGroupItem>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Success is true if all runs are successful
    pub success: bool,
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

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeGroupItem {
    Group(Box<ApicizeGroup>),
    Request(Box<ApicizeRequest>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeGroupChildren {
    Items(ApicizeList<ApicizeGroupItem>),
    Runs(ApicizeList<ApicizeGroupRun>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeExecutionType {
    None,
    Single(ApicizeExecution),
    Runs(ApicizeList<ApicizeExecution>),
    // Rows(ApicizeList<ApicizeExecution>),
    // MultiRunRows(ApicizeList<ApicizeRowRuns>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ApicizeList<T> {
    pub items: Vec<T>,
}

/// A summary of a group
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeGroup {
    /// Request group ID
    pub id: String,
    /// Request group name
    pub name: String,

    /// Row number (if applicable)
    pub row_number: Option<usize>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    // Child requests, groups and/or runs
    pub children: Option<ApicizeGroupChildren>,
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    /// Success is true if all runs are successful
    pub success: bool,
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

/// Represents executions of a multi-run group
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeGroupRun {
    // Run number
    pub run_number: usize,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Each group run
    pub children: Vec<ApicizeGroupItem>,
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    /// Success is true if all runs are successful
    pub success: bool,
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

/// Represents executions of a multi-run row
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]

pub struct ApicizeRowRuns {
    // Row number
    pub row_number: usize,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Each row run
    pub runs: Vec<ApicizeExecution>,
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    /// Success is true if all runs are successful
    pub success: bool,
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

/// A summary of a request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRequest {
    /// Request group ID
    pub id: String,
    /// Request group name
    pub name: String,

    /// Row number (if applicable)
    pub row_number: Option<usize>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    // /// Variables assigned to the group
    /// Variables to update at the end of the group
    pub output_variables: Option<Map<String, Value>>,

    pub execution: ApicizeExecutionType,

    /// Success is true if all runs are successful
    pub success: bool,
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

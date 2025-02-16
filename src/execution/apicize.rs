//! Apicize models submodule
//!
//! This submodule defines models used to execute Apicize tests and report their results

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

use crate::oauth2_client_tokens::TokenResult;
use crate::ApicizeError;

/// Trait to expose functionality for retrieving totals and variables
pub trait ExecutionTotalsSource {
    /// Retrieve totals
    fn get_totals(&self) -> ApicizeExecutionTotals;

    /// Retrieve variables
    fn get_variables(&self) -> &Option<HashMap<String, Value>>;
}

/// Trait to expose functionality for incrementing totals
pub trait ExecutionTotals {
    /// Add totals
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource);
}

/// Information used to dispatch an Apicize request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRequest {
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
    pub variables: Option<HashMap<String, Value>>,
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

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// An Apicize execution of orne or more requests/groups
pub struct ApicizeExecution {
    /// Duration of execution (milliseconds)
    pub duration: u128,
    /// Requests or groups that are executed
    pub items: Vec<ApicizeExecutionItem>,
    /// True if all requests and tests are successful
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
#[serde(rename_all = "camelCase", tag = "type")]
/// Execution reuslts for a group or request
pub enum ApicizeExecutionItem {
    /// Request group that is executed
    Group(Box<ApicizeExecutionGroup>),
    /// Request that is executed
    Request(Box<ApicizeExecutionRequest>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// Request execution results for all runs
pub struct ApicizeExecutionRequest {
    /// Request ID
    pub id: String,
    /// Request name
    pub name: String,
    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    /// Executed request and test executions
    pub runs: Vec<ApicizeExecutionRequestRun>,
    /// Variables to update at the end of the request
    pub variables: Option<HashMap<String, Value>>,
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
    /// If set, indicates a fatal error
    pub error: Option<ApicizeError>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// Request execution results for a specific run
pub struct ApicizeExecutionRequestRun {
    /// Run number (index)
    pub run_number: usize,
    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    /// Request sent as HTTP call
    pub request: Option<ApicizeRequest>,
    /// Response received during exexecuted run
    pub response: Option<ApicizeResponse>,
    /// Set to true if HTTP call succeeded (regardless of status code)
    pub success: bool,
    /// Set if there was an error in execution
    pub error: Option<ApicizeError>,
    /// Tests executed during run
    pub tests: Option<Vec<ApicizeTestResult>>,
    // Input variables, if any, passed into the execution
    pub input_variables: Option<HashMap<String, Value>>,
    /// If set, active variables should be updated to this value after execution
    pub variables: Option<HashMap<String, Value>>,
    // True if run is successful
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

/// Information about the response to a dispatched Apicize request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeResponse {
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

/// Response from V8 when executing a request's tests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeTestResponse {
    /// Results of test
    pub results: Option<Vec<ApicizeTestResult>>,
    /// Scenario values (if any)
    pub variables: HashMap<String, Value>,
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

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// Group execution results for all runs
pub struct ApicizeExecutionGroup {
    /// Request group ID
    pub id: String,
    /// Request group name
    pub name: String,
    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    /// Child requests/groups
    pub runs: Vec<ApicizeExecutionGroupRun>,
    /// True if all requests and tests in group passed for all runs
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
    /// If set, indicates a fatal error
    pub error: Option<ApicizeError>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
/// Group execution results for a specific run
pub struct ApicizeExecutionGroupRun {
    /// Run number (index)
    pub run_number: usize,
    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,
    /// Child requests/groups
    pub items: Vec<ApicizeExecutionItem>,
    /// Variables to update at the end of the group
    pub variables: Option<HashMap<String, Value>>,
    /// Success if requests in run are successful
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

/// Tallies of execution results
pub struct ApicizeExecutionTotals {
    /// True if tests pass and (when applicable) child tests pass
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

impl ExecutionTotalsSource for ApicizeExecutionItem {
    /// Retrieve totals for an execution item
    fn get_totals(&self) -> ApicizeExecutionTotals {
        match self {
            ApicizeExecutionItem::Group(group) => ApicizeExecutionTotals {
                success: group.success,
                requests_with_passed_tests_count: group.requests_with_passed_tests_count,
                requests_with_failed_tests_count: group.requests_with_failed_tests_count,
                requests_with_errors: group.requests_with_errors,
                passed_test_count: group.passed_test_count,
                failed_test_count: group.failed_test_count,
            },
            ApicizeExecutionItem::Request(request) => ApicizeExecutionTotals {
                success: request.success,
                requests_with_passed_tests_count: if request.passed_test_count > 0 { 1 } else { 0 },
                requests_with_failed_tests_count: if request.failed_test_count > 0 { 1 } else { 0 },
                requests_with_errors: if request.success { 0 } else { 1 },
                passed_test_count: request.passed_test_count,
                failed_test_count: request.failed_test_count,
            },
        }
    }

    /// Retrieve variables for item execution
    fn get_variables(&self) -> &Option<HashMap<String, Value>> {
        match self {
            ApicizeExecutionItem::Group(group) => {
                if let Some(last_run) = group.runs.last() {
                    if let Some(last_item) = last_run.items.last() {
                        last_item.get_variables()
                    } else {
                        &None
                    }
                } else {
                    &None
                }
            }
            ApicizeExecutionItem::Request(request) => {
                if let Some(last_run) = request.runs.last() {
                    &last_run.variables
                } else {
                    &None
                }
            }
        }
    }
}

impl ExecutionTotals for ApicizeExecutionItem {
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource) {
        let other_totals = other.get_totals();

        match self {
            ApicizeExecutionItem::Group(group) => {
                group.requests_with_passed_tests_count +=
                    other_totals.requests_with_passed_tests_count;
                group.requests_with_failed_tests_count +=
                    other_totals.requests_with_failed_tests_count;
                group.requests_with_errors += other_totals.requests_with_errors;
                group.passed_test_count += other_totals.passed_test_count;
                group.failed_test_count += other_totals.failed_test_count;
            }
            ApicizeExecutionItem::Request(request) => {
                request.requests_with_passed_tests_count +=
                    other_totals.requests_with_passed_tests_count;
                request.requests_with_failed_tests_count +=
                    other_totals.requests_with_failed_tests_count;
                request.requests_with_errors += other_totals.requests_with_errors;
                request.passed_test_count += other_totals.passed_test_count;
                request.failed_test_count += other_totals.failed_test_count;
            }
        }
    }
}

impl ExecutionTotals for ApicizeExecutionGroup {
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource) {
        let other_totals = other.get_totals();

        self.requests_with_passed_tests_count += other_totals.requests_with_passed_tests_count;
        self.requests_with_failed_tests_count += other_totals.requests_with_failed_tests_count;
        self.requests_with_errors += other_totals.requests_with_errors;
        self.passed_test_count += other_totals.passed_test_count;
        self.failed_test_count += other_totals.failed_test_count;
    }
}

impl ExecutionTotals for ApicizeExecutionRequest {
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource) {
        let other_totals = other.get_totals();
        self.success = self.success && other_totals.success;
        self.requests_with_passed_tests_count += other_totals.requests_with_passed_tests_count;
        self.requests_with_failed_tests_count += other_totals.requests_with_failed_tests_count;
        self.requests_with_errors += other_totals.requests_with_errors;
        self.passed_test_count += other_totals.passed_test_count;
        self.failed_test_count += other_totals.failed_test_count;
    }
}

impl ExecutionTotalsSource for ApicizeExecutionRequestRun {
    /// Retrieve totals for an execution run
    fn get_totals(&self) -> ApicizeExecutionTotals {
        ApicizeExecutionTotals {
            success: self.success,
            requests_with_passed_tests_count: self.requests_with_passed_tests_count,
            requests_with_failed_tests_count: self.requests_with_failed_tests_count,
            requests_with_errors: self.requests_with_errors,
            passed_test_count: self.passed_test_count,
            failed_test_count: self.failed_test_count,
        }
    }

    fn get_variables(&self) -> &Option<HashMap<String, Value>> {
        &self.variables
    }
}

impl ExecutionTotals for ApicizeExecutionRequestRun {
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource) {
        let other_totals = other.get_totals();
        self.success = self.success && other_totals.success;
        self.requests_with_passed_tests_count += other_totals.requests_with_passed_tests_count;
        self.requests_with_failed_tests_count += other_totals.requests_with_failed_tests_count;
        self.requests_with_errors += other_totals.requests_with_errors;
        self.passed_test_count += other_totals.passed_test_count;
        self.failed_test_count += other_totals.failed_test_count;
    }
}

impl ExecutionTotalsSource for ApicizeExecutionGroupRun {
    /// Retrieve totals for an execution run
    fn get_totals(&self) -> ApicizeExecutionTotals {
        ApicizeExecutionTotals {
            success: self.success,
            requests_with_passed_tests_count: self.requests_with_passed_tests_count,
            requests_with_failed_tests_count: self.requests_with_failed_tests_count,
            requests_with_errors: self.requests_with_errors,
            passed_test_count: self.passed_test_count,
            failed_test_count: self.failed_test_count,
        }
    }

    fn get_variables(&self) -> &Option<HashMap<String, Value>> {
        &self.variables
    }
}

impl ExecutionTotals for ApicizeExecutionGroupRun {
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource) {
        let other_totals = other.get_totals();
        self.success = self.success && other_totals.success;
        self.requests_with_passed_tests_count += other_totals.requests_with_passed_tests_count;
        self.requests_with_failed_tests_count += other_totals.requests_with_failed_tests_count;
        self.requests_with_errors += other_totals.requests_with_errors;
        self.passed_test_count += other_totals.passed_test_count;
        self.failed_test_count += other_totals.failed_test_count;
    }
}

impl ExecutionTotals for ApicizeExecution {
    fn add_totals(&mut self, other: &dyn ExecutionTotalsSource) {
        let other_totals = other.get_totals();
        self.success = self.success && other_totals.success;
        self.requests_with_passed_tests_count += other_totals.requests_with_passed_tests_count;
        self.requests_with_failed_tests_count += other_totals.requests_with_failed_tests_count;
        self.requests_with_errors += other_totals.requests_with_errors;
        self.passed_test_count += other_totals.passed_test_count;
        self.failed_test_count += other_totals.failed_test_count;
    }
}

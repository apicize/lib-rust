//! Apicize models submodule
//!
//! This submodule defines models used to execute Apicize tests and report their results

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use std::collections::HashMap;
use std::slice::{Iter, IterMut};

use crate::ApicizeError;

use super::oauth2_client_tokens::TokenResult;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeItem {
    Group(ApicizeSummary),
    Request(ApicizeSummary),
    Items(ApicizeList<Box<ApicizeItem>>),
    ExecutionSummaries(ApicizeList<ApicizeExecutionSummary>),
    ExecutedRequest(ApicizeRequestWithExecution),
    Execution(ApicizeExecution),
}

impl ApicizeItem {
    pub fn get_name(&self) -> &str {
        match self {
            ApicizeItem::Group(group) => group.name.as_str(),
            ApicizeItem::Request(request) => request.name.as_str(),
            ApicizeItem::Items(_) => "",
            ApicizeItem::ExecutionSummaries(_) => "",
            ApicizeItem::ExecutedRequest(request) => request.name.as_str(),
            ApicizeItem::Execution(_) => "",
        }
    }
    pub fn get_success(&self) -> bool {
        match self {
            ApicizeItem::Group(group) => group.success,
            ApicizeItem::Request(request) => request.success,
            ApicizeItem::Items(list) => !list.items.iter().any(|i| !i.get_success()),
            ApicizeItem::ExecutionSummaries(summaries) => {
                !summaries.items.iter().any(|summary| !summary.success)
            }
            ApicizeItem::ExecutedRequest(request) => request.success,
            ApicizeItem::Execution(execution) => match execution {
                ApicizeExecution::Details(list) => !list.items.iter().any(|item| !item.get_success()),
                ApicizeExecution::Rows(summaries) => {
                    !summaries.iter().any(|summary| !summary.success)
                }
                ApicizeExecution::Runs(summaries) => {
                    !summaries.iter().any(|summary| !summary.success)
                }
            },
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
/// This class exists so that we can have a serializable vector
pub struct ApicizeList<T> {
    pub items: Vec<T>
}

impl <T> ApicizeList<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity)
        }
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }


    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.items.iter_mut()
    }

    #[inline]
    pub fn push(&mut self, item: T) {
        self.items.push(item)
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.items.last()
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

    pub children: Option<Box<ApicizeItem>>,

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
#[serde(tag="type")]
pub enum ApicizeExecution {
    Details(ApicizeList<Box<ApicizeItem>>),
    Rows(ApicizeList<ApicizeExecutionSummary>),
    Runs(ApicizeList<ApicizeExecutionDetail>),
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
    pub execution: Option<ApicizeExecution>,

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

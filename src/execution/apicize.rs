//! Apicize models submodule
//!
//! This submodule defines models used to execute Apicize tests and report their results

use std::collections::HashMap;

use crate::{Identifiable, identifiable::CloneIdentifiable};

use super::{ApicizeExecution, DataContext};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeResult {
    Request(Box<ApicizeRequestResult>),
    Group(Box<ApicizeGroupResult>),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeRequestResultContent {
    Rows {
        /// Rows processed for request
        rows: Vec<ApicizeRequestResultRow>,
    },
    Runs {
        /// Runs processed for request
        runs: Vec<ApicizeRequestResultRun>,
    },
    Execution {
        execution: Box<ApicizeExecution>,
    },
}

/// Result for a request has nested runs or rows
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRequestResult {
    /// Result ID
    pub id: String,

    /// Result name
    pub name: String,

    /// Result request/group key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Associative tag name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// URL requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Values applicable to tests
    pub data_context: DataContext,

    /// Request content (rows, runs or an execution)
    pub content: ApicizeRequestResultContent,

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
pub enum ApicizeRequestResultRowContent {
    Runs(Vec<ApicizeRequestResultRun>),
    Execution(Box<ApicizeExecution>),
}

/// Result for a request row
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRequestResultRow {
    /// Row number
    pub row_number: usize,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Values applicable to tests
    pub data_context: DataContext,

    /// Execution result
    pub results: ApicizeRequestResultRowContent,

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

/// Result for a request run
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeRequestResultRun {
    /// Run number
    pub run_number: usize,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Execution result
    pub execution: ApicizeExecution,

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
pub enum ApicizeGroupResultContent {
    Rows { rows: Vec<ApicizeGroupResultRow> },
    Runs { runs: Vec<ApicizeGroupResultRun> },
    Results { results: Vec<ApicizeResult> },
}

/// Result for a request group
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeGroupResult {
    /// Group ID
    pub id: String,
    /// Group name
    pub name: String,
    /// Group key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Associative tag name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Values applicable to tests
    pub data_context: DataContext,

    /// Request group rows, runs or executions
    pub content: ApicizeGroupResultContent,

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
pub enum ApicizeGroupResultRowContent {
    Runs { runs: Vec<ApicizeGroupResultRun> },
    Results { results: Vec<ApicizeResult> },
}

/// Result for a request row
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeGroupResultRow {
    /// Row number
    pub row_number: usize,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Values applicable to tests
    pub data_context: DataContext,

    /// Execution result
    pub content: ApicizeGroupResultRowContent,

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

/// Result for a request run
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeGroupResultRun {
    /// Run number
    pub run_number: usize,

    /// Execution start (millisecond offset from start)
    pub executed_at: u128,
    /// Duration of execution (milliseconds)
    pub duration: u128,

    /// Values applicable to tests
    pub data_context: DataContext,

    /// Execution results
    pub results: Vec<ApicizeResult>,

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
pub struct ApicizeList<T> {
    pub items: Vec<T>,
}

/// Body information used when dispatching an Apicize Request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeBody {
    Text {
        text: String,
    },
    JSON {
        text: String,
        data: Value,
    },
    XML {
        text: String,
        data: Value,
    },
    Form {
        text: String,
        data: HashMap<String, String>,
    },
    Binary {
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        data: Vec<u8>,
    },
}

/// Response from V8 when executing a request's tests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeTestResponse {
    /// Results of test
    pub results: Option<Vec<ApicizeTestResult>>,
    /// Output values to send to next test
    pub output: Map<String, Value>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ApicizeTestResult {
    /// Describe block of a test scenario
    Scenario(ApicizeTestScenario),
    /// Behavior test result
    Behavior(ApicizeTestBehavior),
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeTestScenario {
    /// Human readable name of test scenario
    pub name: String,

    /// Whether or not all child tests were successful
    pub success: bool,

    /// Child scenarios or behaviors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ApicizeTestResult>>,

    pub test_count: usize,

    pub test_fail_count: usize,

    /// Console I/O generated during the test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<String>>,
}

/// Test execution results
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeTestBehavior {
    /// Human readable name of the test
    pub name: String,
    /// Tagged reference of the test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Whether or not the test executed and passed successful
    pub success: bool,
    /// Error generated during the test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Console I/O generated during the test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<String>>,
}

impl Identifiable for ApicizeResult {
    fn get_id(&self) -> &str {
        match self {
            ApicizeResult::Request(request) => request.get_id(),
            ApicizeResult::Group(group) => group.get_id(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            ApicizeResult::Request(request) => request.get_name(),
            ApicizeResult::Group(group) => group.get_name(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            ApicizeResult::Request(request) => request.get_title(),
            ApicizeResult::Group(group) => group.get_title(),
        }
    }
}

impl CloneIdentifiable for ApicizeResult {
    fn clone_as_new(&self, new_name: String) -> Self {
        match self {
            ApicizeResult::Request(request) => {
                ApicizeResult::Request(Box::new(request.clone_as_new(new_name)))
            }
            ApicizeResult::Group(group) => {
                ApicizeResult::Group(Box::new(group.clone_as_new(new_name)))
            }
        }
    }
}

impl Identifiable for ApicizeRequestResult {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            "Unnamed".to_string()
        } else {
            self.name.clone()
        }
    }
}

impl CloneIdentifiable for ApicizeRequestResult {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.name = new_name;
        cloned
    }
}

impl Identifiable for ApicizeGroupResult {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            "Unnamed".to_string()
        } else {
            self.name.clone()
        }
    }
}

impl CloneIdentifiable for ApicizeGroupResult {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.name = new_name;
        cloned
    }
}

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::serde_as;
use std::collections::HashMap;

use crate::ApicizeError;

use super::{oauth2_client_tokens::TokenResult, ApicizeBody, ApicizeTestBehavior};

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeExecution {
    /// Name of the request being executed
    pub name: String,

    /// Values available to use in the test context
    pub test_context: ApicizeExecutionTestContext,

    /// Variables returned at the end of the test for use in next request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_variables: Option<Map<String, Value>>,   

    /// Test results (if executed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<ApicizeTestBehavior>>,

    /// Error on dispatch or error execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApicizeError>,

    /// Success is rrequest executes and tests all pass
    pub success: bool,

    /// Number of passed tests, if request and tests are succesfully run
    pub test_pass_count: usize,
    
    /// Number of failed tests, if request and tests are succesfully run
    pub test_fail_count: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeExecutionTestContext {

    /// Merged variables and data
    #[serde(skip_serializing_if = "Option::is_none", rename="$")]
    pub merged: Option<Map<String, Value>>,

    /// Variables available from scenario
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<Map<String, Value>>,

    /// Variables output from previous test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Map<String, Value>>,

    /// Row data assigned to the groups' requests (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<String, Value>>,

    /// Request sent to server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<ApicizeHttpRequest>,

    /// Response received from server (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ApicizeHttpResponse>,
}


#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeHttpRequest {
    /// URL sent
    pub url: String,
    /// HTTP Method
    pub method: String,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ApicizeBody>,
}

/// Information about the response to a dispatched Apicize request
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeHttpResponse {
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

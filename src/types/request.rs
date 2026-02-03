use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use super::identifiable::CloneIdentifiable;
use super::{NameValuePair, Selection};
use crate::{
    Identifiable, SelectedParameters, Validated, ValidationState, add_validation_error, remove_validation_error, utility::*
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use xmltojson::to_json;

pub fn default_runs() -> usize {
    1
}

fn default_redirects() -> usize {
    10
}

/// Enumeration of HTTP methods
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RequestMethod {
    /// HTTP GET
    Get,
    /// HTTP POST
    Post,
    /// HTTP PUT
    Put,
    /// HTTP DELETE
    Delete,
    /// HTTP PATCH
    Patch,
    /// HTTP HEAD
    Head,
    /// HTTP OPTIONS
    Options,
}

/// Apicize Request body
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum RequestBody {
    /// Text (UTF-8) body data
    Text {
        /// Text
        data: String,
    },
    /// JSON body data
    #[serde(rename = "JSON")]
    JSON {
        /// Text
        data: String,
    },
    /// XML body data
    #[serde(rename = "XML")]
    XML {
        /// Text
        data: String,
    },
    /// Form (not multipart) body data
    Form {
        /// Name/value pairs of form data
        data: Vec<NameValuePair>,
    },
    /// Binary body data serialized as Base64
    Raw {
        /// Base-64 encoded binary data
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        data: Vec<u8>,
    },
}

/// Indicator on  request execution order
#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExecutionConcurrency {
    /// Requests are executed sequentially
    #[default]
    Sequential,
    /// Requests are executed concurrently
    Concurrent,
}

/// Information required to dispatch and test an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// Unique identifier (required to keep track of dispatches and test executions)
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Human-readable name describing the Apicize Request
    pub name: String,
    // /// Current execution state of request
    // #[serde(default, skip_serializing_if = "ExecutionState::is_empty")]
    // pub execution_state: ExecutionState,
    /// Optional identifier for the Apicize Requset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// URL to dispatch the HTTP request to
    pub url: String,
    /// HTTP method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<RequestMethod>,
    /// Timeout, in milliseconds, to wait for a response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<NameValuePair>>,
    /// HTTP query string parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_string_params: Option<Vec<NameValuePair>>,
    /// HTTP body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<RequestBody>,
    /// Keep HTTP connection alive
    #[serde(default = "bool::default", skip_serializing_if = "std::ops::Not::not")]
    pub keep_alive: bool,
    /// Allow invalid certificates (default is false)
    #[serde(default = "bool::default", skip_serializing_if = "std::ops::Not::not")]
    pub accept_invalid_certs: bool,
    /// Number redirects (default = 10)
    #[serde(default = "default_redirects")]
    pub number_of_redirects: usize,
    /// Number of runs for the request to execute
    #[serde(default = "default_runs")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default)]
    pub multi_run_execution: ExecutionConcurrency,
    /// Test to execute after dispatching request and receiving response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// Selected scenario, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_scenario: Option<Selection>,
    /// Selected authorization, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_authorization: Option<Selection>,
    /// Selected certificate, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_certificate: Option<Selection>,
    /// Selected proxy, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_proxy: Option<Selection>,
    /// Selected external data, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_data: Option<Selection>,
    /// Validation state
    #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
    pub validation_state: ValidationState,
    /// Warnings for invalid values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_warnings: Option<Vec<String>>,
    /// Validation errors
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<HashMap<String, String>>,
}

/// A group of Apicize Requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestGroup {
    /// Uniquely identifies group of Apicize requests
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Human-readable name of the Apicize Group
    pub name: String,
    // /// Current execution state of request
    // #[serde(default, skip_serializing_if = "ExecutionState::is_empty")]
    // pub execution_state: ExecutionState,
    /// Optional identifier for the Apicize Group,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Child items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<RequestEntry>>,
    /// Execution of children
    #[serde(default)]
    pub execution: ExecutionConcurrency,
    /// Number of runs for the group to execute
    #[serde(default = "default_runs")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default)]
    pub multi_run_execution: ExecutionConcurrency,
    /// Selected scenario, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_scenario: Option<Selection>,
    /// Selected authorization, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_authorization: Option<Selection>,
    /// Selected certificate, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_certificate: Option<Selection>,
    /// Selected proxy, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_proxy: Option<Selection>,
    /// Selected external data, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_data: Option<Selection>,
    /// Validation state
    #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
    pub validation_state: ValidationState,
    /// Warnings for invalid values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_warnings: Option<Vec<String>>,
    /// Validation errors
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<HashMap<String, String>>,
}

/// Apcize Request that is either a specific request to run (Info)
/// or a group of requests (Group)
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum RequestEntry {
    /// Request to run
    Request(Request),
    /// Group of Apicize Requests
    Group(RequestGroup),
}

impl Identifiable for RequestEntry {
    fn get_id(&self) -> &str {
        match self {
            RequestEntry::Request(request) => request.get_id(),
            RequestEntry::Group(group) => group.get_id(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            RequestEntry::Request(request) => request.get_name(),
            RequestEntry::Group(group) => group.get_name(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            RequestEntry::Request(request) => request.get_title(),
            RequestEntry::Group(group) => group.get_title(),
        }
    }
}

impl Validated for RequestEntry {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            RequestEntry::Request(request) => request.validation_state,
            RequestEntry::Group(group) => group.validation_state,
        }
    }

    /// Retrieve warnings
    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            RequestEntry::Request(request) => &request.validation_warnings,
            RequestEntry::Group(group) => &group.validation_warnings,
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        match self {
            RequestEntry::Request(request) => request.set_validation_warnings(warnings),
            RequestEntry::Group(group) => group.set_validation_warnings(warnings),
        }
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            RequestEntry::Request(request) => &request.validation_errors,
            RequestEntry::Group(group) => &group.validation_errors,
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        match self {
            RequestEntry::Request(request) => request.set_validation_errors(errors),
            RequestEntry::Group(group) => group.set_validation_errors(errors),
        }
    }
}

// impl Executable for RequestEntry {
//     fn get_execution_state(&self) -> &ExecutionState {
//         match self {
//             RequestEntry::Request(request) => &request.execution_state,
//             RequestEntry::Group(group) => &group.execution_state,
//         }
//     }
// }

impl CloneIdentifiable for RequestEntry {
    fn clone_as_new(&self, new_name: String) -> Self {
        match self {
            RequestEntry::Request(request) => RequestEntry::Request(request.clone_as_new(new_name)),
            RequestEntry::Group(group) => RequestEntry::Group(group.clone_as_new(new_name)),
        }
    }
}

// xxx
// impl ValidatedSelectedParameters for RequestEntry {
//     fn validate_scenario(&mut self, valid_values: &HashMap<String, String>) {
//         match self {
//             RequestEntry::Request(request) => {
//                 if let Some(warning) = validate_selection(
//                     &request.get_title(),
//                     &mut request.selected_scenario,
//                     "scenario",
//                     valid_values,
//                 ) {
//                     request.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//             RequestEntry::Group(group) => {
//                 if let Some(warning) = validate_selection(
//                     &group.get_title(),
//                     &mut group.selected_scenario,
//                     "scenario",
//                     valid_values,
//                 ) {
//                     group.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//         }
//     }

//     fn validate_authorization(&mut self, valid_values: &HashMap<String, String>) {
//         match self {
//             RequestEntry::Request(request) => {
//                 if let Some(warning) = validate_selection(
//                     &request.get_title(),
//                     &mut request.selected_authorization,
//                     "authorization",
//                     valid_values,
//                 ) {
//                     request.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//             RequestEntry::Group(group) => {
//                 if let Some(warning) = validate_selection(
//                     &group.get_title(),
//                     &mut group.selected_authorization,
//                     "authorization",
//                     valid_values,
//                 ) {
//                     group.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//         }
//     }

//     fn validate_certificate(&mut self, valid_values: &HashMap<String, String>) {
//         match self {
//             RequestEntry::Request(request) => {
//                 if let Some(warning) = validate_selection(
//                     &request.get_title(),
//                     &mut request.selected_certificate,
//                     "certificate",
//                     valid_values,
//                 ) {
//                     request.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//             RequestEntry::Group(group) => {
//                 if let Some(warning) = validate_selection(
//                     &group.get_title(),
//                     &mut group.selected_certificate,
//                     "certificate",
//                     valid_values,
//                 ) {
//                     group.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//         }
//     }

//     fn validate_proxy(&mut self, valid_values: &HashMap<String, String>) {
//         match self {
//             RequestEntry::Request(request) => {
//                 if let Some(warning) = validate_selection(
//                     &request.get_title(),
//                     &mut request.selected_proxy,
//                     "proxy",
//                     valid_values,
//                 ) {
//                     request.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//             RequestEntry::Group(group) => {
//                 if let Some(warning) = validate_selection(
//                     &group.get_title(),
//                     &mut group.selected_proxy,
//                     "proxy",
//                     valid_values,
//                 ) {
//                     group.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//         }
//     }

//     fn validate_data(&mut self, valid_values: &HashMap<String, String>) {
//         match self {
//             RequestEntry::Request(request) => {
//                 if let Some(warning) = validate_selection(
//                     &request.get_title(),
//                     &mut request.selected_data,
//                     "data",
//                     valid_values,
//                 ) {
//                     request.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//             RequestEntry::Group(group) => {
//                 if let Some(warning) = validate_selection(
//                     &group.get_title(),
//                     &mut group.selected_data,
//                     "data",
//                     valid_values,
//                 ) {
//                     group.set_validation_warnings(Some(vec![warning]));
//                 }
//             }
//         }
//     }
// }

/// HTTP methods for Apicize Requests
impl RequestMethod {
    /// Returns Apicize Request method as string
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestMethod::Get => "GET",
            RequestMethod::Post => "POST",
            RequestMethod::Put => "PUT",
            RequestMethod::Delete => "DELETE",
            RequestMethod::Patch => "PATCH",
            RequestMethod::Head => "HEAD",
            RequestMethod::Options => "OPTIONS",
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Display for RequestGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: Default::default(),
            key: Default::default(),
            validation_state: Default::default(),
            // execution_state: Default::default(),
            test: Some(
                r#"describe('status', () => {
    it('equals 200', () => {
        expect(response.status).to.equal(200)
    })
})"#
                .to_string(),
            ),
            url: Default::default(),
            method: Default::default(),
            timeout: Default::default(),
            headers: Default::default(),
            query_string_params: Default::default(),
            body: Default::default(),
            keep_alive: Default::default(),
            runs: 1,
            multi_run_execution: ExecutionConcurrency::Sequential,
            selected_scenario: Default::default(),
            selected_authorization: Default::default(),
            selected_certificate: Default::default(),
            selected_proxy: Default::default(),
            selected_data: Default::default(),
            validation_warnings: Default::default(),
            validation_errors: None,
            accept_invalid_certs: false,
            number_of_redirects: default_redirects(),
        }
    }
}

impl Identifiable for Request {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_title(&self) -> String {
        let name = self.get_name();
        if name.is_empty() {
            "(Unnamed)".to_string()
        } else {
            name.to_string()
        }
    }
}

impl CloneIdentifiable for Request {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

impl Validated for Request {
    fn get_validation_state(&self) -> ValidationState {
        self.validation_state
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        &self.validation_warnings
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        self.validation_warnings = warnings;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        &self.validation_errors
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        self.validation_errors = errors;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }
}

impl Request {
    pub fn perform_validation(&mut self) {
        self.validate_name();
        self.validate_url();
    }

    pub fn validate_name(&mut self) {
        let name_ok = ! self.name.trim().is_empty();
        if name_ok {
            remove_validation_error(&mut self.validation_errors, "name");
        } else {
            add_validation_error(&mut self.validation_errors, "name", "Name is required");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }

    pub fn validate_url(&mut self) {
        let url_ok = ! self.url.trim().is_empty();
        if url_ok {
            remove_validation_error(&mut self.validation_errors, "url");
        } else {
            add_validation_error(&mut self.validation_errors, "url", "URL is required");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }
}

impl Identifiable for RequestGroup {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_title(&self) -> String {
        let name = self.get_name();
        if name.is_empty() {
            "(Unnamed)".to_string()
        } else {
            name.to_string()
        }
    }
}

impl CloneIdentifiable for RequestGroup {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

impl Default for RequestGroup {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: Default::default(),
            key: Default::default(),
            // execution_state: Default::default(),
            children: Default::default(),
            execution: ExecutionConcurrency::Sequential,
            runs: 1,
            multi_run_execution: ExecutionConcurrency::Sequential,
            selected_scenario: Default::default(),
            selected_authorization: Default::default(),
            selected_certificate: Default::default(),
            selected_proxy: Default::default(),
            selected_data: Default::default(),
            validation_state: Default::default(),
            validation_warnings: None,
            validation_errors: None,
        }
    }
}

impl Validated for RequestGroup {
    fn get_validation_state(&self) -> ValidationState {
        self.validation_state
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        &self.validation_warnings
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        self.validation_warnings = warnings;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        &self.validation_errors
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        self.validation_errors = errors;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }
}

impl RequestGroup {
    pub fn perform_validation(&mut self) {
        self.validate_name();
    }

    pub fn validate_name(&mut self) {
        let name_ok = ! self.name.trim().is_empty();
        if name_ok {
            remove_validation_error(&mut self.validation_errors, "name");
        } else {
            add_validation_error(&mut self.validation_errors, "name", "Name is required");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }
}

impl RequestEntry {
    pub fn perform_validation(&mut self) {
        match self {
            RequestEntry::Request(request) => request.perform_validation(),
            RequestEntry::Group(group) => group.perform_validation(),
        }
    }

    /// Utility function to perform string substitution based upon search/replace values in "subs"
    pub fn clone_and_sub(text: &str, subs: &HashMap<String, String>) -> String {
        if subs.is_empty() {
            text.to_string()
        } else {
            let mut clone = text.to_string();
            for (find, value) in subs.iter() {
                clone = str::replace(&clone, find, value)
            }
            clone
        }
    }

    /// Retrieve request entry number of runs
    pub fn get_runs(&self) -> usize {
        match self {
            RequestEntry::Request(info) => info.runs,
            RequestEntry::Group(group) => group.runs,
        }
    }

    /// Set number of runs
    pub fn set_runs(&mut self, runs: usize) {
        match self {
            RequestEntry::Request(info) => {
                info.runs = runs;
            }
            RequestEntry::Group(group) => {
                group.runs = runs;
            }
        }
    }
}

impl Display for RequestEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestEntry::Request(i) => write!(f, "{}", i.name),
            RequestEntry::Group(g) => write!(f, "{}", g.name),
        }
    }
}

impl SelectedParameters for RequestEntry {
    fn selected_scenario(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Request(info) => &info.selected_scenario,
            RequestEntry::Group(group) => &group.selected_scenario,
        }
    }

    fn selected_authorization(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Request(info) => &info.selected_authorization,
            RequestEntry::Group(group) => &group.selected_authorization,
        }
    }

    fn selected_certificate(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Request(info) => &info.selected_certificate,
            RequestEntry::Group(group) => &group.selected_certificate,
        }
    }

    fn selected_proxy(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Request(info) => &info.selected_proxy,
            RequestEntry::Group(group) => &group.selected_proxy,
        }
    }

    fn selected_data(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Request(info) => &info.selected_data,
            RequestEntry::Group(group) => &group.selected_data,
        }
    }

    fn selected_scenario_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Request(info) => &mut info.selected_scenario,
            RequestEntry::Group(group) => &mut group.selected_scenario,
        }
    }

    fn selected_authorization_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Request(info) => &mut info.selected_authorization,
            RequestEntry::Group(group) => &mut group.selected_authorization,
        }
    }

    fn selected_certificate_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Request(info) => &mut info.selected_certificate,
            RequestEntry::Group(group) => &mut group.selected_certificate,
        }
    }

    fn selected_proxy_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Request(info) => &mut info.selected_proxy,
            RequestEntry::Group(group) => &mut group.selected_proxy,
        }
    }

    fn selected_data_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Request(info) => &mut info.selected_data,
            RequestEntry::Group(group) => &mut group.selected_data,
        }
    }
}

/// Apicize Request body
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum StoredRequestBody {
    /// Text (UTF-8) body data
    Text {
        /// Text
        data: String,
    },
    /// JSON body data
    #[serde(rename = "JSON")]
    JSON {
        /// Parsed data (if formatted is valid)
        data: Option<Value>,
        /// Formatted text
        formatted: Option<String>,
    },
    /// XML body data
    #[serde(rename = "XML")]
    XML {
        /// Formatted text
        formatted: Option<String>,
    },
    /// Form (not multipart) body data
    Form {
        /// Name/value pairs of form data
        data: Vec<NameValuePair>,
    },
    /// Binary body data serialized as Base64
    Raw {
        /// Base-64 encoded binary data
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        data: Vec<u8>,
    },
}

/// Information required to dispatch and test an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredRequest {
    /// Unique identifier (required to keep track of dispatches and test executions)
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Human-readable name describing the Apicize Request
    pub name: String,
    /// Optional identifier for the Apicize Request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Test to execute after dispatching request and receiving response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// URL to dispatch the HTTP request to
    pub url: String,
    /// HTTP method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<RequestMethod>,
    /// Timeout, in milliseconds, to wait for a response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<NameValuePair>>,
    /// HTTP query string parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_string_params: Option<Vec<NameValuePair>>,
    /// HTTP body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<StoredRequestBody>,
    /// Keep HTTP connection alive
    #[serde(default = "bool::default", skip_serializing_if = "std::ops::Not::not")]
    pub keep_alive: bool,
    /// Allow invalid certificates (default is false)
    #[serde(default = "bool::default", skip_serializing_if = "std::ops::Not::not")]
    pub accept_invalid_certs: bool,
    /// Number redirects (default = 10)
    #[serde(default = "default_redirects")]
    pub number_of_redirects: usize,
    /// Number of runs for the request to execute
    #[serde(default = "default_runs")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default)]
    pub multi_run_execution: ExecutionConcurrency,
    /// Selected scenario, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_scenario: Option<Selection>,
    /// Selected authorization, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_authorization: Option<Selection>,
    /// Selected certificate, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_certificate: Option<Selection>,
    /// Selected proxy, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_proxy: Option<Selection>,
    /// Selected external data, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_data: Option<Selection>,
    /// Populated with any warnings regarding how the request is set up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// A group of Apicize Requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredRequestGroup {
    /// Uniquely identifies group of Apicize requests
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Human-readable name of the Apicize Group
    pub name: String,
    /// Optional identifier for the Apicize Group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Child items
    pub children: Option<Vec<StoredRequestEntry>>,
    /// Execution of children
    #[serde(default)]
    pub execution: ExecutionConcurrency,
    /// Number of runs for the group to execute
    #[serde(default = "default_runs")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default)]
    pub multi_run_execution: ExecutionConcurrency,
    /// Selected scenario, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_scenario: Option<Selection>,
    /// Selected authorization, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_authorization: Option<Selection>,
    /// Selected certificate, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_certificate: Option<Selection>,
    /// Selected proxy, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_proxy: Option<Selection>,
    /// Selected external data, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_data: Option<Selection>,
    /// Populated with any warnings regarding how the group is set up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// Apcize Request that is either a specific request to run (Info)
/// or a group of requests (Group)
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum StoredRequestEntry {
    /// Request to run
    Request(StoredRequest),
    /// Group of Apicize Requests
    Group(StoredRequestGroup),
}

impl StoredRequestEntry {
    pub fn from_workspace(entry: RequestEntry) -> StoredRequestEntry {
        match entry {
            RequestEntry::Request(request) => StoredRequestEntry::Request(StoredRequest {
                id: request.id,
                name: request.name,
                key: request.key,
                test: request.test,
                url: request.url,
                method: request.method,
                timeout: request.timeout,
                headers: request.headers,
                query_string_params: request.query_string_params,
                body: match request.body {
                    Some(body) => match body {
                        RequestBody::Text { data } => Some(StoredRequestBody::Text { data }),
                        RequestBody::JSON { data } => {
                            // If the data from the workspace is serializable, then store the serialized version,
                            // as well as writing the raw data
                            let data_to_save = Value::from_str(&data).ok();
                            Some(StoredRequestBody::JSON {
                                data: data_to_save,
                                formatted: Some(data),
                            })
                        }
                        RequestBody::XML { data } => {
                            // If the data from the workspace is serializable, then store the serialized version,
                            // as well as writing the raw data
                            let data_to_save = to_json(&data).ok();
                            Some(StoredRequestBody::JSON {
                                data: data_to_save,
                                formatted: Some(data),
                            })
                        }
                        RequestBody::Form { data } => Some(StoredRequestBody::Form { data }),
                        RequestBody::Raw { data } => Some(StoredRequestBody::Raw { data }),
                    },
                    None => None,
                },
                keep_alive: request.keep_alive,
                accept_invalid_certs: request.accept_invalid_certs,
                number_of_redirects: request.number_of_redirects,
                runs: request.runs,
                multi_run_execution: request.multi_run_execution,
                selected_scenario: request.selected_scenario,
                selected_authorization: request.selected_authorization,
                selected_certificate: request.selected_certificate,
                selected_proxy: request.selected_proxy,
                selected_data: request.selected_data,
                warnings: request.validation_warnings,
            }),
            RequestEntry::Group(group) => StoredRequestEntry::Group(StoredRequestGroup {
                id: group.id,
                name: group.name,
                key: group.key,
                children: if let Some(children) = group.children && ! children.is_empty() {
                    Some(
                        children
                            .into_iter()
                            .map(StoredRequestEntry::from_workspace)
                            .collect()
                    )
                } else {
                    None
                },
                execution: group.execution,
                runs: group.runs,
                multi_run_execution: group.multi_run_execution,
                selected_scenario: group.selected_scenario,
                selected_authorization: group.selected_authorization,
                selected_certificate: group.selected_certificate,
                selected_proxy: group.selected_proxy,
                selected_data: group.selected_data,
                warnings: group.validation_warnings,
            }),
        }
    }

    pub fn to_workspace(self) -> RequestEntry {
        match self {
            StoredRequestEntry::Request(stored_request) => RequestEntry::Request(Request {
                id: stored_request.id,
                name: stored_request.name,
                key: stored_request.key,
                validation_state: Default::default(),
                // execution_state: Default::default(),
                test: stored_request.test,
                url: stored_request.url,
                method: stored_request.method,
                timeout: stored_request.timeout,
                headers: stored_request.headers,
                query_string_params: stored_request.query_string_params,
                body: match stored_request.body {
                    Some(body) => match body {
                        StoredRequestBody::Text { data } => Some(RequestBody::Text { data }),
                        StoredRequestBody::JSON { formatted, data } => {
                            let result_data: Option<String>;
                            if let Some(s) = formatted {
                                result_data = Some(s);
                            } else if let Some(v) = data {
                                if let Ok(s) = serde_json::to_string_pretty(&v) {
                                    result_data = Some(s);
                                } else {
                                    result_data = None;
                                }
                            } else {
                                result_data = None;
                            }

                            result_data.map(|d| RequestBody::JSON { data: d })
                        }
                        StoredRequestBody::XML { formatted } => Some(RequestBody::XML {
                            data: match formatted {
                                Some(text) => text,
                                None => "".to_string(),
                            },
                        }),
                        StoredRequestBody::Form { data } => Some(RequestBody::Form { data }),
                        StoredRequestBody::Raw { data } => Some(RequestBody::Raw { data }),
                    },
                    None => None,
                },
                keep_alive: stored_request.keep_alive,
                accept_invalid_certs: stored_request.accept_invalid_certs,
                number_of_redirects: stored_request.number_of_redirects,
                runs: stored_request.runs,
                multi_run_execution: stored_request.multi_run_execution,
                selected_scenario: stored_request.selected_scenario,
                selected_authorization: stored_request.selected_authorization,
                selected_certificate: stored_request.selected_certificate,
                selected_proxy: stored_request.selected_proxy,
                selected_data: stored_request.selected_data,
                validation_warnings: stored_request.warnings,
                validation_errors: None,
            }),
            StoredRequestEntry::Group(group) => RequestEntry::Group(RequestGroup {
                id: group.id,
                name: group.name,
                key: group.key,
                validation_state: Default::default(),
                // execution_state: Default::default(),
                children: if let Some(children) = group.children && ! children.is_empty() {
                    Some(
                        children
                            .into_iter()
                            .map(StoredRequestEntry::to_workspace)
                            .collect()
                    )
                } else {
                    None
                },
                execution: group.execution,
                runs: group.runs,
                multi_run_execution: group.multi_run_execution,
                selected_scenario: group.selected_scenario,
                selected_authorization: group.selected_authorization,
                selected_certificate: group.selected_certificate,
                selected_proxy: group.selected_proxy,
                selected_data: group.selected_data,
                validation_warnings: group.warnings,
                validation_errors: None,
            }),
        }
    }
}


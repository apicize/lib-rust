use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use super::{NameValuePair, Selection, Warnings};
use crate::{utility::*, Identifable, SelectedParameters};

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
        data: Value,
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
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExecutionConcurrency {
    /// Requests are executed sequentially
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
    pub body: Option<RequestBody>,
    /// Keep HTTP connection alive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<bool>,
    /// Number of runs for the request to execute
    #[serde(default = "one")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default = "sequential")]
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
pub struct RequestGroup {
    /// Uniquely identifies group of Apicize requests
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Human-readable name of group
    pub name: String,
    /// Child items
    pub children: Option<Vec<RequestEntry>>,
    /// Execution of children
    #[serde(default = "sequential")]
    pub execution: ExecutionConcurrency,
    /// Number of runs for the group to execute
    #[serde(default = "one")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default = "sequential")]
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
pub enum RequestEntry {
    /// Request to run
    Info(Request),
    /// Group of Apicize Requests
    Group(RequestGroup),
}


impl Identifable for RequestEntry {
    fn get_id(&self) -> &String {
        return self.get_id();
    }

    fn get_name(&self) -> &String {
        return self.get_name()
    }

    fn get_title(&self) -> String {
        let (id, name) = self.get_id_and_name();
        if name.is_empty() {
            format!("{} (Unnamed)", id)
        } else {
            name.to_string()
        }
    }
}

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

impl RequestEntry {
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

    /// Retrieve request entry ID
    pub fn get_id(&self) -> &String {
        match self {
            RequestEntry::Info(info) => &info.id,
            RequestEntry::Group(group) => &group.id,
        }
    }

    /// Retrieve request entry name
    pub fn get_name(&self) -> &String {
        match self {
            RequestEntry::Info(info) => &info.name,
            RequestEntry::Group(group) => &group.name,
        }
    }

    /// Retrieve ID and name
    pub fn get_id_and_name(&self) -> (&String, &String) {
        match self {
            RequestEntry::Info(info) => (&info.id, &info.name),
            RequestEntry::Group(group) => (&group.id, &group.name),
        }
    }

    /// Retrieve ID and name
    pub fn get_title(&self) -> String {
        let (id, name) = self.get_id_and_name();
        format!("{} ({})", name, id)
    }    

    /// Retrieve request entry number of runs
    pub fn get_runs(&self) -> usize {
        match self {
            RequestEntry::Info(info) => info.runs,
            RequestEntry::Group(group) => group.runs,
        }
    }

}

impl Display for RequestEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestEntry::Info(i) => write!(f, "{}", i.name),
            RequestEntry::Group(g) => write!(f, "{}", g.name),
        }
    }
}

impl SelectedParameters for RequestEntry {
    fn selected_scenario(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Info(info) => &info.selected_scenario,
            RequestEntry::Group(group) => &group.selected_scenario,
        }
    }

    fn selected_authorization(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Info(info) => &info.selected_authorization,
            RequestEntry::Group(group) => &group.selected_authorization,
        }
    }

    fn selected_certificate(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Info(info) => &info.selected_certificate,
            RequestEntry::Group(group) => &group.selected_certificate,
        }
    }

    fn selected_proxy(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Info(info) => &info.selected_proxy,
            RequestEntry::Group(group) => &group.selected_proxy,
        }
    }

    fn selected_data(&self) -> &Option<Selection> {
        match self {
            RequestEntry::Info(info) => &info.selected_data,
            RequestEntry::Group(group) => &group.selected_data,
        }
    }

    fn selected_scenario_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Info(info) => &mut info.selected_scenario,
            RequestEntry::Group(group) => &mut group.selected_scenario,
        }
    }

    fn selected_authorization_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Info(info) => &mut info.selected_authorization,
            RequestEntry::Group(group) => &mut group.selected_authorization,
        }
    }

    fn selected_certificate_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Info(info) => &mut info.selected_certificate,
            RequestEntry::Group(group) => &mut group.selected_certificate,
        }
    }

    fn selected_proxy_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Info(info) => &mut info.selected_proxy,
            RequestEntry::Group(group) => &mut group.selected_proxy,
        }
    }

    fn selected_data_as_mut(&mut self) -> &mut Option<Selection> {
        match self {
            RequestEntry::Info(info) => &mut info.selected_data,
            RequestEntry::Group(group) => &mut group.selected_data,
        }
    }    
}

// Implement warnings trait for requests and groups
impl Warnings for RequestEntry {
    /// Retrieve warnings
    fn get_warnings(&self) -> &Option<Vec<String>> {
        match self {
            RequestEntry::Info(request) => &request.warnings,
            RequestEntry::Group(group) => &group.warnings,
        }
    }

    fn add_warning(&mut self, warning: String) {
        match self {
            RequestEntry::Info(request) => {
                match &mut request.warnings {
                    Some(warnings) => warnings.push(warning),
                    None => request.warnings = Some(vec![warning])
                }
            }
            RequestEntry::Group(group) => {
                match &mut group.warnings {
                    Some(warnings) => warnings.push(warning),
                    None => group.warnings = Some(vec![warning])
                }
            }
        }
    }
}

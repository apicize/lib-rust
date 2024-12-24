//! Workbook models submodule
//!
//! This submodule defines modules used to retrive and store Workbooks and access workbook entities
//! (requests, scenarios, authorizations, certificates and proxies)
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;

use crate::ApicizeError;

use super::{
    save_data_file, utility::*, Identifable, IndexedEntities, SelectableOptions, SerializationFailure, SerializationSaveSuccess, Warnings, WorkspaceParameter
};
use reqwest::{ClientBuilder, Error, Identity, Proxy};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

/// State of a selected option
pub enum WorkbookSelectedOption<T> {
    /// Use default parent selection (if available)
    UseDefault,
    /// Do not send a value for this selection
    Off,
    /// Use this value
    Some(T),
}

/// Enumeration of HTTP methods
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkbookRequestMethod {
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

/// String name/value pairs used to store values like Apicize headers, query string parameters, etc.
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkbookNameValuePair {
    /// Name of value
    pub name: String,
    /// Value
    pub value: String,
    /// If set to true, name/value pair should be ignored when dispatching Apicize Requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

/// Apicize Request body
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum WorkbookRequestBody {
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
        data: Vec<WorkbookNameValuePair>,
    },
    /// Binary body data serialized as Base64
    Raw {
        /// Base-64 encoded binary data
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        data: Vec<u8>,
    },
}

/// Specifies persistence options for non-request entities
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum Persistence {
    /// Shared configuration file
    Global,
    /// Workbook private information file
    Private,
    /// Workbook file
    Workbook,
}

/// Information about a selected entity, include both ID and name
/// to give the maximum chance of finding a match
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    /// ID of selected entity
    pub id: String,
    /// Name of selected entity
    pub name: String,
}

/// Indicator on workbook request execution order
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkbookExecution {
    /// Requests are executed sequentially
    Sequential,
    /// Requests are executed concurrently
    Concurrent,
}

/// Information required to dispatch and test an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookRequest {
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
    pub method: Option<WorkbookRequestMethod>,
    /// Timeout, in milliseconds, to wait for a response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<WorkbookNameValuePair>>,
    /// HTTP query string parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_string_params: Option<Vec<WorkbookNameValuePair>>,
    /// HTTP body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<WorkbookRequestBody>,
    /// Keep HTTP connection alive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<bool>,
    /// Number of runs for the request to execute
    #[serde(default = "one")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default = "sequential")]
    pub multi_run_execution: WorkbookExecution,
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
    /// Populated with any warnings regarding how the request is set up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// A group of Apicize Requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookRequestGroup {
    /// Uniquely identifies group of Apicize requests
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Human-readable name of group
    pub name: String,
    /// Child items
    pub children: Option<Vec<WorkbookRequestEntry>>,
    /// Execution of children
    #[serde(default = "sequential")]
    pub execution: WorkbookExecution,
    /// Number of runs for the group to execute
    #[serde(default = "one")]
    pub runs: usize,
    /// Execution of multiple runs
    #[serde(default = "sequential")]
    pub multi_run_execution: WorkbookExecution,
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
    /// Populated with any warnings regarding how the group is set up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// Apcize Request that is either a specific request to run (Info)
/// or a group of requests (Group)
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum WorkbookRequestEntry {
    /// Request to run
    Info(WorkbookRequest),
    /// Group of Apicize Requests
    Group(WorkbookRequestGroup),
}

/// A set of variables that can be injected into templated values
/// when submitting an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookScenario {
    /// Uniquely identifies scenario
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Name of variable to substitute (avoid using curly braces)
    pub name: String,
    /// Specifies how authorization will be saved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<Persistence>,
    /// Value of variable to substitute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<WorkbookNameValuePair>>,
}

/// Authorization information used when dispatching an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum WorkbookAuthorization {
    /// Basic authentication (basic authorization header)
    #[serde(rename_all = "camelCase")]
    Basic {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of authorization configuration
        name: String,
        /// Specifies how authorization will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// User name
        username: String,
        /// Password
        password: String,
    },
    /// OAuth2 client flow (bearer authorization header)
    #[serde(rename_all = "camelCase")]
    OAuth2Client {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// Specifies how authorization will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        /// Client secret (allowed to be blank)
        client_secret: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: Option<String>,
        /// Selected certificate, if applicable
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_certificate: Option<Selection>,
        /// Selected proxy, if applicable
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_proxy: Option<Selection>,
        // #[serde(skip_serializing_if="Option::is_none")]
        // send_credentials_in_body: Option<bool>,
    },
    /// OAuth2 PKCE flow (note, this can only be used interactively)
    #[serde(rename_all = "camelCase")]
    OAuth2Pkce {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// Specifies how authorization will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// URL for authorization
        authorization_url: String,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: Option<String>,
        /// Currently active token (needs to be set before usage)
        token: Option<String>,
        /// Currently active refresh token if available (needs to be set before usage)
        refresh_token: Option<String>,
        /// Expiration of currently active token in seconds past Unix epoch (needs to be set before usage)
        expiration: Option<u64>,
        // #[serde(skip_serializing_if="Option::is_none")]
        // send_credentials_in_body: Option<bool>,
    },    
    /// API key authentication (sent in HTTP header)
    #[serde(rename_all = "camelCase")]
    ApiKey {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// Specifies how authorization will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// Name of header (ex. "x-api-key")
        header: String,
        /// Value of key to include as header value
        value: String,
    },
}

/// Client certificate used to identify caller
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum WorkbookCertificate {
    /// PKCS 12 certificate and and password (.p12 or .pfx)
    #[serde(rename = "PKCS12")]
    PKCS12 {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Specifies how cetificate will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// Certificate
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pfx: Vec<u8>,
        /// Password
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
    /// PEM-encoded certificate and PKCS8 encoded private key files
    #[serde(rename = "PKCS8_PEM")]
    PKCS8PEM {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Specifies how cetificate will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
        /// Optional key file, if not combining in PKCS8 format
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        key: Vec<u8>,
    },
    /// PEM encoded certificate and key file
    #[serde(rename = "PEM")]
    PEM {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Specifies how cetificate will be saved
        #[serde(skip_serializing_if = "Option::is_none")]
        persistence: Option<Persistence>,
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
    },
}

/// An HTTP or SOCKS5 proxy that can be used to tunnel requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkbookProxy {
    /// Uniquely identify proxy
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Name of proxy
    pub name: String,
    /// Specifies how proxy will be saved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<Persistence>,
    /// Location of proxy (URL for HTTP proxy, IP for SOCKS)
    pub url: String,
}

/// Defaults for the workbook
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookDefaults {
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
}

/// Persisted Apcizize requests and scenario definitions
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Workbook {
    /// Version of workbook format (should not be changed manually)
    pub version: f32,
    /// List of requests/request groups
    pub requests: Vec<WorkbookRequestEntry>,
    /// List of scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenarios: Option<Vec<WorkbookScenario>>,
    /// Workbook Authorizations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizations: Option<Vec<WorkbookAuthorization>>,
    /// Workbook certificates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificates: Option<Vec<WorkbookCertificate>>,
    /// Workbook proxy servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<WorkbookProxy>>,
    /// Workbook defaults
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<WorkbookDefaults>,
}

impl Identifable for Selection {
    fn get_id(&self) -> &String {
        &self.id
    }
    
    fn get_name(&self) -> &String {
        &self.name
    }
    
    fn get_title(&self) -> String {
        if self.name.is_empty() {
            format!("{} (Unnamed)", self.id)
        } else {
            self.name.to_string()
        }
    }
}

impl Workbook {
    /// Save workbook information to the specified file
    pub fn save_workbook(
        file_name: PathBuf,
        requests: Vec<WorkbookRequestEntry>,
        scenarios: Vec<WorkbookScenario>,
        authorizations: Vec<WorkbookAuthorization>,
        certificates: Vec<WorkbookCertificate>,
        proxies: Vec<WorkbookProxy>,
        defaults: Option<WorkbookDefaults>,
    ) -> Result<SerializationSaveSuccess, SerializationFailure> {
        let save_scenarios = if scenarios.is_empty() {
            None
        } else {
            Some(scenarios.clone())
        };
        let save_authorizations = if authorizations.is_empty() {
            None
        } else {
            Some(authorizations.clone())
        };
        let save_certiificates = if certificates.is_empty() {
            None
        } else {
            Some(certificates.clone())
        };
        let save_proxies = if proxies.is_empty() {
            None
        } else {
            Some(proxies.clone())
        };

        let workbook = Workbook {
            version: 1.0,
            requests,
            scenarios: save_scenarios,
            authorizations: save_authorizations,
            certificates: save_certiificates,
            proxies: save_proxies,
            defaults,
        };

        save_data_file(&file_name, &workbook)
    }
}

impl Identifable for WorkbookRequestEntry {
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
impl WorkbookRequestMethod {
    /// Returns Apicize Request method as string
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkbookRequestMethod::Get => "GET",
            WorkbookRequestMethod::Post => "POST",
            WorkbookRequestMethod::Put => "PUT",
            WorkbookRequestMethod::Delete => "DELETE",
            WorkbookRequestMethod::Patch => "PATCH",
            WorkbookRequestMethod::Head => "HEAD",
            WorkbookRequestMethod::Options => "OPTIONS",
        }
    }
}

impl Display for WorkbookRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Display for WorkbookRequestGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl WorkbookRequestEntry {
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
            WorkbookRequestEntry::Info(info) => &info.id,
            WorkbookRequestEntry::Group(group) => &group.id,
        }
    }

    /// Retrieve request entry name
    pub fn get_name(&self) -> &String {
        match self {
            WorkbookRequestEntry::Info(info) => &info.name,
            WorkbookRequestEntry::Group(group) => &group.name,
        }
    }

    /// Retrieve ID and name
    pub fn get_id_and_name(&self) -> (&String, &String) {
        match self {
            WorkbookRequestEntry::Info(info) => (&info.id, &info.name),
            WorkbookRequestEntry::Group(group) => (&group.id, &group.name),
        }
    }

    /// Retrieve request entry number of runs
    pub fn get_runs(&self) -> usize {
        match self {
            WorkbookRequestEntry::Info(info) => info.runs,
            WorkbookRequestEntry::Group(group) => group.runs,
        }
    }

    /// Validate parameters to ensure they either have valid IDs and/or names
    pub fn validate_parameters(
        &mut self,
        scenarios: &IndexedEntities<WorkbookScenario>,
        authorizations: &IndexedEntities<WorkbookAuthorization>,
        certificates: &IndexedEntities<WorkbookCertificate>,
        proxies: &IndexedEntities<WorkbookProxy>,
    ) {
        if let Some(selection) = self.get_selected_scenario() {
            let ok = scenarios.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Request {} scenario {} not found, defaulting to Parent",
                    self.get_title(),
                    selection.get_title(),
                ));
                self.set_selected_scenario(None);
            }
        }

        if let Some(selection) = self.get_selected_authorization() {
            let ok = authorizations.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Request {} authorization {} not found, defaulting to Parent",
                    self.get_title(),
                    selection.get_title(),
                ));
                self.set_selected_authorization(None);
            }
        }

        if let Some(selection) = self.get_selected_certificate() {
            let ok = certificates.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Request {} certificate {} not found, defaulting to Parent",
                    self.get_title(),
                    selection.get_title(),
                ));
                self.set_selected_certificate(None);
            }
        }

        if let Some(selection) = self.get_selected_proxy() {
            let ok = proxies.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Request {} selected proxy {} not found, defaulting to Parent",
                    self.get_title(),
                    selection.get_title(),
                ));
                self.set_selected_proxy(None);
            }
        }
    }
}

impl Display for WorkbookRequestEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkbookRequestEntry::Info(i) => write!(f, "{}", i.name),
            WorkbookRequestEntry::Group(g) => write!(f, "{}", g.name),
        }
    }
}

impl SelectableOptions for WorkbookRequestEntry {
    fn get_selected_scenario(&self) -> &Option<Selection> {
        match self {
            WorkbookRequestEntry::Info(info) => &info.selected_scenario,
            WorkbookRequestEntry::Group(group) => &group.selected_scenario,
        }
    }

    fn get_selected_authorization(&self) -> &Option<Selection> {
        match self {
            WorkbookRequestEntry::Info(info) => &info.selected_authorization,
            WorkbookRequestEntry::Group(group) => &group.selected_authorization,
        }
    }

    fn get_selected_certificate(&self) -> &Option<Selection> {
        match self {
            WorkbookRequestEntry::Info(info) => &info.selected_certificate,
            WorkbookRequestEntry::Group(group) => &group.selected_certificate,
        }
    }

    fn get_selected_proxy(&self) -> &Option<Selection> {
        match self {
            WorkbookRequestEntry::Info(info) => &info.selected_proxy,
            WorkbookRequestEntry::Group(group) => &group.selected_proxy,
        }
    }

    fn set_selected_scenario(&mut self, value: Option<Selection>) {
        match self {
            WorkbookRequestEntry::Info(info) => info.selected_scenario = value,
            WorkbookRequestEntry::Group(group) => group.selected_scenario = value,
        }
    }

    fn set_selected_authorization(&mut self, value: Option<Selection>) {
        match self {
            WorkbookRequestEntry::Info(info) => info.selected_authorization = value,
            WorkbookRequestEntry::Group(group) => group.selected_authorization = value,
        }
    }

    fn set_selected_certificate(&mut self, value: Option<Selection>) {
        match self {
            WorkbookRequestEntry::Info(info) => info.selected_certificate = value,
            WorkbookRequestEntry::Group(group) => group.selected_certificate = value,
        }
    }

    fn set_selected_proxy(&mut self, value: Option<Selection>) {
        match self {
            WorkbookRequestEntry::Info(info) => info.selected_proxy = value,
            WorkbookRequestEntry::Group(group) => group.selected_proxy = value,
        }
    }
}

/// Implement warnings trait for requests and groups
impl Warnings for WorkbookRequestEntry {
    /// Retrieve warnings
    fn get_warnings(&self) -> &Option<Vec<String>> {
        match self {
            WorkbookRequestEntry::Info(request) => &request.warnings,
            WorkbookRequestEntry::Group(group) => &group.warnings,
        }
    }

    fn add_warning(&mut self, warning: String) {
        match self {
            WorkbookRequestEntry::Info(request) => {
                match &mut request.warnings {
                    Some(warnings) => warnings.push(warning),
                    None => request.warnings = Some(vec![warning])
                }
            }
            WorkbookRequestEntry::Group(group) => {
                match &mut group.warnings {
                    Some(warnings) => warnings.push(warning),
                    None => group.warnings = Some(vec![warning])
                }
            }
        }
    }
}

impl WorkbookAuthorization {
    fn get_id_and_name(&self) -> (&String, &String) {
        match self {
            WorkbookAuthorization::Basic { id, name, .. } => (id, name),
            WorkbookAuthorization::OAuth2Client { id, name, .. } => (id, name),
            WorkbookAuthorization::OAuth2Pkce { id, name, .. } => (id, name),
            WorkbookAuthorization::ApiKey { id, name, .. } => (id, name),
        }
    }
}

impl Identifable for WorkbookAuthorization {
    fn get_id(&self) -> &String {
        let (id, _) = self.get_id_and_name();
        id
    }

    fn get_name(&self) -> &String {
        let (_, name) = self.get_id_and_name();
        name
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

impl WorkspaceParameter<WorkbookAuthorization> for WorkbookAuthorization {
    fn get_persistence(&self) -> Option<Persistence> {
        match self {
            WorkbookAuthorization::Basic { persistence, .. } => *persistence,
            WorkbookAuthorization::OAuth2Client { persistence, .. } => *persistence,
            WorkbookAuthorization::OAuth2Pkce { persistence, .. } => *persistence,
            WorkbookAuthorization::ApiKey { persistence, .. } => *persistence,
        }
    }

    fn set_persistence(&mut self, persistence_to_set: Persistence) {
        match self {
            WorkbookAuthorization::Basic { persistence, .. } => {
                *persistence = Some(persistence_to_set)
            }
            WorkbookAuthorization::OAuth2Client { persistence, .. } => {
                *persistence = Some(persistence_to_set)
            }
            WorkbookAuthorization::OAuth2Pkce { persistence, .. } => {
                *persistence = Some(persistence_to_set)
            }
            WorkbookAuthorization::ApiKey { persistence, .. } => {
                *persistence = Some(persistence_to_set)
            }
        }
    }

    fn clear_persistence(&mut self) {
        match self {
            WorkbookAuthorization::Basic { persistence, .. } => *persistence = None,
            WorkbookAuthorization::OAuth2Client { persistence, .. } => *persistence = None,
            WorkbookAuthorization::OAuth2Pkce { persistence, .. } => *persistence = None,
            WorkbookAuthorization::ApiKey { persistence, .. } => *persistence = None,
        }
    }
}

impl Identifable for WorkbookScenario {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            format!("{} (Unnamed)", self.id)
        } else {
            self.name.to_string()
        }
    }
}

impl WorkspaceParameter<WorkbookScenario> for WorkbookScenario {
    fn get_persistence(&self) -> Option<Persistence> {
        self.persistence
    }

    fn set_persistence(&mut self, persistence_to_set: Persistence) {
        self.persistence = Some(persistence_to_set);
    }

    fn clear_persistence(&mut self) {
        self.persistence = None
    }
}

impl WorkbookCertificate {
    fn get_id_and_name(&self) -> (&String, &String) {
        match self {
            WorkbookCertificate::PKCS8PEM { id, name, .. } => (id, name),
            WorkbookCertificate::PEM { id, name, .. } => (id, name),
            WorkbookCertificate::PKCS12 { id, name, .. } => (id, name),
        }
    }

    /// Append certificate to builder
    pub fn append_to_builder(
        &self,
        builder: ClientBuilder,
    ) -> Result<ClientBuilder, ApicizeError> {
        let identity_result = match self {
            WorkbookCertificate::PKCS12 { pfx, password, .. } => Identity::from_pkcs12_der(
                pfx,
                password.clone().unwrap_or(String::from("")).as_str(),
            ),
            WorkbookCertificate::PKCS8PEM { pem, key, .. } => Identity::from_pkcs8_pem(pem, key),
            WorkbookCertificate::PEM { pem, .. } => Identity::from_pem(pem),
        };

        match identity_result {
            Ok(identity) => {
                // request_certificate = Some(cert.clone());
                Ok(
                    builder
                        .identity(identity)
                        // .connection_verbose(true)
                        .use_native_tls(), // .tls_info(true)
                )
            }
            Err(err) => Err(ApicizeError::from_reqwest(err)),
        }
    }    
}

impl Identifable for WorkbookCertificate {
    fn get_id(&self) -> &String {
        let (id, _) = self.get_id_and_name();
        id
    }

    fn get_name(&self) -> &String {
        let (_, name) = self.get_id_and_name();
        name
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

impl WorkspaceParameter<WorkbookCertificate> for WorkbookCertificate {
    fn get_persistence(&self) -> Option<Persistence> {
        match self {
            WorkbookCertificate::PKCS8PEM { persistence, .. } => *persistence,
            WorkbookCertificate::PEM { persistence, .. } => *persistence,
            WorkbookCertificate::PKCS12 { persistence, .. } => *persistence,
        }
    }

    fn set_persistence(&mut self, persistence_to_set: Persistence) {
        match self {
            WorkbookCertificate::PKCS8PEM { persistence, .. } => {
                *persistence = Some(persistence_to_set)
            }
            WorkbookCertificate::PEM { persistence, .. } => *persistence = Some(persistence_to_set),
            WorkbookCertificate::PKCS12 { persistence, .. } => {
                *persistence = Some(persistence_to_set)
            }
        }
    }

    fn clear_persistence(&mut self) {
        match self {
            WorkbookCertificate::PKCS8PEM { persistence, .. } => *persistence = None,
            WorkbookCertificate::PEM { persistence, .. } => *persistence = None,
            WorkbookCertificate::PKCS12 { persistence, .. } => *persistence = None,
        }
    }
}

impl WorkbookProxy {
    /// Append proxy to builder
    pub fn append_to_builder(&self, builder: ClientBuilder) -> Result<ClientBuilder, Error> {
        match Proxy::all(&self.url) {
            Ok(proxy) => Ok(builder.proxy(proxy)),
            Err(err) => Err(err),
        }
    }
}

impl Identifable for WorkbookProxy {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            format!("{} (Unnamed)", self.id)
        } else {
            self.name.to_string()
        }
    }
}

impl WorkspaceParameter<WorkbookProxy> for WorkbookProxy {
    fn get_persistence(&self) -> Option<Persistence> {
        self.persistence
    }

    fn set_persistence(&mut self, persistence_to_set: Persistence) {
        self.persistence = Some(persistence_to_set);
    }

    fn clear_persistence(&mut self) {
        self.persistence = None;
    }
}
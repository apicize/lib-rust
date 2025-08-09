use crate::{Authorization, Certificate, Proxy, Scenario};
use serde::{Deserialize, Serialize};
use super::{ExternalData, StoredRequestEntry, WorkbookDefaultParameters};

/// Persisted Apcizize requests and scenario definitions
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Workbook {
    /// Version of workbook format (should not be changed manually)
    pub version: f32,
    /// List of requests/request groups
    pub requests: Vec<StoredRequestEntry>,
    /// List of scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenarios: Option<Vec<Scenario>>,
    /// Workbook Authorizations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizations: Option<Vec<Authorization>>,
    /// Workbook certificates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificates: Option<Vec<Certificate>>,
    /// Workbook proxy servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<Proxy>>,
    /// External data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<ExternalData>>,
    /// Workbook defaults
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<WorkbookDefaultParameters>
}

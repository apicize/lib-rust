//! Workbook models submodule
//!
//! Storage of workbooks (requests and public parameters)
use std::path::PathBuf;
use crate::{save_data_file, Authorization, Certificate, Proxy, RequestEntry, Scenario, SerializationFailure, SerializationSaveSuccess};
use serde::{Deserialize, Serialize};

use super::WorkbookDefaultParameters;

/// Persisted Apcizize requests and scenario definitions
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Workbook {
    /// Version of workbook format (should not be changed manually)
    pub version: f32,
    /// List of requests/request groups
    pub requests: Vec<RequestEntry>,
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
    /// Workbook defaults
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<WorkbookDefaultParameters>
}

impl Workbook {
    /// Save workbook information to the specified file
    pub fn save(
        file_name: PathBuf,
        requests: Vec<RequestEntry>,
        scenarios: Option<Vec<Scenario>>,
        authorizations: Option<Vec<Authorization>>,
        certificates: Option<Vec<Certificate>>,
        proxies: Option<Vec<Proxy>>,
        defaults: Option<WorkbookDefaultParameters>
    ) -> Result<SerializationSaveSuccess, SerializationFailure> {
        let save_scenarios = match scenarios {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.to_vec())
                }
            },
            None => None,
        };
        let save_authorizations = match authorizations {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.to_vec())
                }
            },
            None => None,
        };
        let save_certiificates = match certificates {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.to_vec())
                }
            },
            None => None,
        };
        let save_proxies = match proxies {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.to_vec())
                }
            },
            None => None,
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

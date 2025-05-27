//! Workspace models submodule
//!
//! This submodule defines modules used to manage workspaces

use crate::{
    open_data_file, ApicizeError, Authorization, Certificate, FileAccessError, Identifiable,
    PersistedIndex, Proxy, RequestEntry, Scenario, SelectedParameters, SerializationSaveSuccess,
    Workbook, WorkbookDefaultParameters,
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use super::{
    validated_selected_parameters::ValidatedSelectedParameters,
    indexed_entities::NO_SELECTION_ID, ExternalData, IndexedEntities, Parameters, VariableCache,
};

/// Data type for entities used by Apicize during testing and editing.  This will be
/// the combination of ,  credential and global settings values
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// Requests for the workspace
    pub requests: IndexedEntities<RequestEntry>,

    /// Scenarios for the workspace
    pub scenarios: IndexedEntities<Scenario>,

    /// Authorizations for the workspace
    pub authorizations: IndexedEntities<Authorization>,

    /// Certificates for the workspace
    pub certificates: IndexedEntities<Certificate>,

    /// Proxies for the workspace
    pub proxies: IndexedEntities<Proxy>,

    /// External data for the workspace
    pub data: Vec<ExternalData>,

    /// Default values for requests and groups
    pub defaults: WorkbookDefaultParameters,
}

impl Workspace {
    /// Create a new workspace, including globals specified (if any)
    pub fn new() -> Result<Workspace, FileAccessError> {
        // Populate parameters from global vault, if available
        let global_parameters = Parameters::open(&Parameters::get_globals_filename(), true)?;

        Ok(Workspace {
            requests: IndexedEntities::<RequestEntry>::default(),
            scenarios: IndexedEntities::<Scenario>::new(
                None,
                None,
                global_parameters.scenarios.as_deref(),
            ),
            authorizations: IndexedEntities::<Authorization>::new(
                None,
                None,
                global_parameters.authorizations.as_deref(),
            ),
            certificates: IndexedEntities::<Certificate>::new(
                None,
                None,
                global_parameters.certificates.as_deref(),
            ),
            proxies: IndexedEntities::<Proxy>::new(
                None,
                None,
                global_parameters.proxies.as_deref(),
            ),
            data: Vec::new(),
            defaults: WorkbookDefaultParameters::default(),
        })
    }

    /// Open the specified  and globals file names
    pub fn open(workbook_file_name: &PathBuf) -> Result<Workspace, FileAccessError> {
        // Open workbook
        let workbook = match open_data_file(workbook_file_name) {
            Ok(success) => success.data,
            Err(error) => {
                return Err(error);
            }
        };

        // Load private parameters if file exists
        let private_parameters = Parameters::open(
            &Parameters::get_workbook_vault_filename(workbook_file_name),
            true,
        )?;

        // Load globals if file exists
        let global_parameters = Parameters::open(&Parameters::get_globals_filename(), true)?;

        Self::build_workspace(workbook, private_parameters, global_parameters)
    }

    pub fn build_workspace(
        workbook: Workbook,
        private_parameters: Parameters,
        global_parameters: Parameters,
    ) -> Result<Workspace, FileAccessError> {
        let workspace_requests = workbook
            .requests
            .into_iter()
            .map(|r| r.to_workspace())
            .collect::<Vec<RequestEntry>>();

        let mut workspace = Workspace {
            requests: IndexedEntities::new(&workspace_requests),
            scenarios: IndexedEntities::<Scenario>::new(
                workbook.scenarios.as_deref(),
                private_parameters.scenarios.as_deref(),
                global_parameters.scenarios.as_deref(),
            ),
            authorizations: IndexedEntities::<Authorization>::new(
                workbook.authorizations.as_deref(),
                private_parameters.authorizations.as_deref(),
                global_parameters.authorizations.as_deref(),
            ),
            certificates: IndexedEntities::<Certificate>::new(
                workbook.certificates.as_deref(),
                private_parameters.certificates.as_deref(),
                global_parameters.certificates.as_deref(),
            ),
            proxies: IndexedEntities::<Proxy>::new(
                workbook.proxies.as_deref(),
                private_parameters.proxies.as_deref(),
                global_parameters.proxies.as_deref(),
            ),
            data: workbook.data.unwrap_or_default(),
            defaults: workbook.defaults.unwrap_or_default(),
        };

        workspace.validate_selections();


        Ok(workspace)
    }

    pub fn validate_selections(
        &mut self
    ) {
        // Validate the default scenarios, etc. selected for testing
        let scenarios = self
            .scenarios
            .entities
            .iter()
            .map(|(id, e)| (id.clone(), e.get_name().to_string()))
            .collect::<HashMap<String, String>>();
        let authorizations = self
            .authorizations
            .entities
            .iter()
            .map(|(id, e)| (id.clone(), e.get_name().to_string()))
            .collect::<HashMap<String, String>>();
        let certificates = self
            .certificates
            .entities
            .iter()
            .map(|(id, e)| (id.clone(), e.get_name().to_string()))
            .collect::<HashMap<String, String>>();
        let proxies = self
            .proxies
            .entities
            .iter()
            .map(|(id, e)| (id.clone(), e.get_name().to_string()))
            .collect::<HashMap<String, String>>();
        let data = self
            .data
            .iter()
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect::<HashMap<String, String>>();

        self.defaults.validate_scenario(&scenarios);
        self.defaults.validate_authorization(&authorizations);
        self.defaults.validate_certificate(&certificates);
        self.defaults.validate_proxy(&proxies);
        self.defaults.validate_data(&data);

        for entity in self.requests.entities.values_mut() {
            entity.validate_scenario(&scenarios);
            entity.validate_authorization(&authorizations);
            entity.validate_certificate(&certificates);
            entity.validate_proxy(&proxies);
            entity.validate_data(&data);
        }

    }
    

    /// Save workspace to specified path, including private and global parameters
    pub fn save(
        &self,
        workbook_path: &PathBuf,
    ) -> Result<Vec<SerializationSaveSuccess>, FileAccessError> {
        let mut successes: Vec<SerializationSaveSuccess> = vec![];

        match Workbook::save(
            PathBuf::from(workbook_path),
            self.requests.to_entities(),
            self.scenarios.get_workbook(),
            self.authorizations.get_workbook(),
            self.certificates.get_workbook(),
            self.proxies.get_workbook(),
            if self.data.is_empty() {
                None
            } else {
                Some(self.data.clone())
            },
            if self.defaults.any_values_set() {
                Some(self.defaults.clone())
            } else {
                None
            },
        ) {
            Ok(success) => successes.push(success),
            Err(error) => return Err(error),
        }

        let private_parameters = Parameters::new(
            self.scenarios.get_private(),
            self.authorizations.get_private(),
            self.certificates.get_private(),
            self.proxies.get_private(),
        );

        match private_parameters.save(&Parameters::get_workbook_vault_filename(workbook_path)) {
            Ok(success) => successes.push(success),
            Err(error) => return Err(error),
        }

        let global_parameters = Parameters::new(
            self.scenarios.get_vault(),
            self.authorizations.get_vault(),
            self.certificates.get_vault(),
            self.proxies.get_vault(),
        );

        match global_parameters.save(&Parameters::get_globals_filename()) {
            Ok(success) => successes.push(success),
            Err(error) => return Err(error),
        }

        Ok(successes)
    }

    /// Retrieve the parameters IDs and scenario variables for the specified request,
    /// merging in the variables and data to scenario (if specified)
    pub fn retrieve_request_parameters(
        &self,
        request: &RequestEntry,
        value_cache: &Mutex<VariableCache>,
        params: &RequestExecutionParameters,
    ) -> Result<RequestExecutionParameters, ApicizeError> {
        let mut done = false;

        // let id = request.get_id();
        // println!("ID: {}", id);

        let mut current = request;

        let mut scenario: Option<&Scenario> = None;
        let mut authorization: Option<&Authorization> = None;
        let mut certificate: Option<&Certificate> = None;
        let mut proxy: Option<&Proxy> = None;
        let mut data: Option<&ExternalData> = None;

        let mut auth_certificate_id: Option<String> = None;
        let mut auth_proxy_id: Option<String> = None;

        let mut allow_scenario = true;
        let mut allow_authorization = true;
        let mut allow_certificate = true;
        let mut allow_proxy = true;
        let mut allow_data = true;

        let mut encountered_ids = HashSet::<String>::new();

        while !done {
            // Set the credential values at the current request value
            if allow_scenario && scenario.is_none() {
                match self.scenarios.find(current.selected_scenario()) {
                    SelectedOption::UseDefault => {}
                    SelectedOption::Off => {
                        allow_scenario = false;
                    }
                    SelectedOption::Some(s) => {
                        scenario = Some(s);
                    }
                }
            }
            if allow_authorization && authorization.is_none() {
                match self.authorizations.find(current.selected_authorization()) {
                    SelectedOption::UseDefault => {}
                    SelectedOption::Off => {
                        allow_authorization = false;
                    }
                    SelectedOption::Some(a) => {
                        authorization = Some(a);
                    }
                }
            }
            if allow_certificate && certificate.is_none() {
                match self.certificates.find(current.selected_certificate()) {
                    SelectedOption::UseDefault => {}
                    SelectedOption::Off => {
                        allow_certificate = false;
                    }
                    SelectedOption::Some(c) => {
                        certificate = Some(c);
                    }
                }
            }
            if allow_proxy && proxy.is_none() {
                match self.proxies.find(current.selected_proxy()) {
                    SelectedOption::UseDefault => {}
                    SelectedOption::Off => {
                        allow_proxy = false;
                    }
                    SelectedOption::Some(p) => {
                        proxy = Some(p);
                    }
                }
            }
            if allow_data && data.is_none() {
                if let Some(sd) = current.selected_data() {
                    if sd.id == NO_SELECTION_ID {
                        allow_data = false;
                    } else if let Some(matching_data) = self.data.iter().find(|d| d.id == sd.id) {
                        data = Some(matching_data);
                    } else {
                        allow_data = false;
                    };
                }
            }
            done = (scenario.is_some() || !allow_scenario)
                && (authorization.is_some() || !allow_authorization)
                && (certificate.is_some() || !allow_certificate)
                && (proxy.is_some() || !allow_proxy)
                && (data.is_some() || !allow_data);

            if !done {
                // Get the parent
                let id = current.get_id().to_string();

                let mut parent: Option<&RequestEntry> = None;
                for (parent_id, children) in self.requests.child_ids.iter() {
                    if children.contains(&id) {
                        parent = self.requests.entities.get(&parent_id.clone());
                        break;
                    }
                }

                if let Some(found_parent) = parent {
                    let parent_id = found_parent.get_id();
                    if encountered_ids.contains(parent_id) {
                        done = true
                    } else {
                        current = found_parent;
                    }
                } else {
                    done = true;
                }

                encountered_ids.insert(id);
            }
        }

        // Load from defaults if required
        if scenario.is_none() && allow_scenario {
            if let SelectedOption::Some(s) = self.scenarios.find(&self.defaults.selected_scenario) {
                scenario = Some(s);
            }
        }
        if authorization.is_none() && allow_authorization {
            if let SelectedOption::Some(a) = self
                .authorizations
                .find(&self.defaults.selected_authorization)
            {
                authorization = Some(a);
            }
        }
        if certificate.is_none() && allow_certificate {
            if let SelectedOption::Some(c) =
                self.certificates.find(&self.defaults.selected_certificate)
            {
                certificate = Some(c);
            }
        }
        if proxy.is_none() && allow_proxy {
            if let SelectedOption::Some(p) = self.proxies.find(&self.defaults.selected_proxy) {
                proxy = Some(p);
            }
        }
        if data.is_none() && allow_data {
            if let Some(selected_data) = &self.defaults.selected_data {
                if selected_data.id == NO_SELECTION_ID {
                    allow_data = false;
                } else if let Some(selected_data) = self
                    .data
                    .iter()
                    .find(|data| data.id == selected_data.id || data.name == selected_data.name)
                {
                    data = Some(selected_data);
                } else {
                    allow_data = false;
                }
            }
        }

        // Set up OAuth2 cert/proxy if specified
        if let Some(Authorization::OAuth2Client {
            selected_certificate,
            selected_proxy,
            ..
        }) = authorization
        {
            if let SelectedOption::Some(c) = self.certificates.find(selected_certificate) {
                auth_certificate_id = Some(c.get_id().to_string());
            }

            if let SelectedOption::Some(proxy) = self.proxies.find(selected_proxy) {
                auth_proxy_id = Some(proxy.get_id().to_string());
            }
        }

        let mut locked_cache = value_cache.lock().unwrap();

        // Build out variables for the request from scenario variables
        let variables = if let Some(active_scenario) = scenario {
            Map::from_iter(
                locked_cache
                    .get_scenario_values(active_scenario)
                    .iter()
                    .filter_map(|(name, value)| match value {
                        Ok(v) => Some((name.clone(), v.clone())),
                        Err(_) => None,
                    }),
            )
        } else {
            Map::new()
        };

        // Retrieve data set if requested
        let mut data_enabled = true;
        let data_set = if allow_data {
            if params.data_set.is_some() {
                Arc::new(None)
            } else {
                match data {
                    Some(d) => Arc::new(match locked_cache.get_external_data(d) {
                        Ok(rows) => {
                            if rows.is_empty() {
                                None
                            } else {
                                Some(RequestDataSet {
                                    id: d.id.clone(),
                                    data: rows.clone(),
                                })
                            }
                        }
                        Err(err) => {
                            return Err(err.clone());
                        }
                    }),
                    None => params.data_set.clone(),
                }
            }
        } else {
            data_enabled = false;
            Arc::new(None)
        };

        Ok(RequestExecutionParameters {
            variables: if variables.is_empty() {
                None
            } else {
                Some(variables)
            },
            data_set,
            data_enabled,
            authorization_id: authorization.map(|a| a.get_id().to_string()),
            certificate_id: certificate.map(|a| a.get_id().to_string()),
            proxy_id: proxy.map(|p| p.get_id().to_string()),
            auth_certificate_id,
            auth_proxy_id,
        })
    }
}

/// Parameters to use when executing a request/group,
/// these should not change during execution
#[derive(Clone, Default)]
pub struct RequestExecutionParameters {
    pub data_set: Arc<Option<RequestDataSet>>,
    pub data_enabled: bool,
    pub variables: Option<Map<String, Value>>,
    pub authorization_id: Option<String>,
    pub certificate_id: Option<String>,
    pub proxy_id: Option<String>,
    pub auth_certificate_id: Option<String>,
    pub auth_proxy_id: Option<String>,
}

/// Thse values may change during the execution of a request/group
#[derive(Default, Clone)]
pub struct RequestExecutionState {
    pub row: Option<RequestDataRow>,
    pub output_variables: Option<RequestDataRow>,
}

pub type RequestDataRow = Map<String, Value>;
pub struct RequestDataSet {
    pub id: String,
    pub data: Vec<RequestDataRow>,
}

// impl RequestDataSet {
//     // If the data set defined for a request/group is not default, is off or does not match
//     // data that is in use, then turn off data, becuase we currently can only have one
//     // active data set at a time
//     fn is_ok_to_use(&self, selected_dataset_id: &str) -> bool {
//         selected_dataset_id != NO_SELECTION_ID && selected_dataset_id == self.id
//     }
// }

/// State of a selected option
pub enum SelectedOption<T> {
    /// Use default parent selection (if available)
    UseDefault,
    /// Do not send a value for this selection
    Off,
    /// Use this value
    Some(T),
}

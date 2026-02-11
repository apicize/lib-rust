//! Workspace models submodule
//!
//! This submodule defines modules used to manage workspaces

use crate::{
    ApicizeError, Authorization, Certificate, DataSourceType, ExecutionReportCsv,
    ExecutionReportCsvSingleRun, ExecutionReportFormat, ExecutionReportJson,
    ExecutionResultSummary, Identifiable, PersistedIndex, Proxy, RequestEntry, Scenario,
    SelectedParameters, Selection, SerializationSaveSuccess, StoredRequestEntry, Validated,
    Workbook, WorkbookDefaultParameters, indexed_entities::NO_SELECTION_ID, open_data_file,
    open_data_stream, save_data_file, selected_parameters::SelectableParameters,
};

use csv::WriterBuilder;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, ser::PrettyFormatter};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    io::stdin,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use super::{DataSet, IndexedEntities, Parameters, VariableCache};

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

    /// Data sets for the workspace
    pub data: IndexedEntities<DataSet>,

    /// Default values for requests and groups
    pub defaults: WorkbookDefaultParameters,
}

impl Workspace {
    /// Create a new workspace, including globals specified (if any)
    pub fn new() -> Result<Workspace, ApicizeError> {
        // Populate parameters from global vault, if available
        let global_parameters = Parameters::open(&Parameters::get_globals_filename(), true)?;

        Ok(Workspace {
            requests: IndexedEntities::<RequestEntry>::default(),
            scenarios: IndexedEntities::<Scenario>::new(None, None, global_parameters.scenarios),
            authorizations: IndexedEntities::<Authorization>::new(
                None,
                None,
                global_parameters.authorizations,
            ),
            certificates: IndexedEntities::<Certificate>::new(
                None,
                None,
                global_parameters.certificates,
            ),
            proxies: IndexedEntities::<Proxy>::new(None, None, global_parameters.proxies),
            data: IndexedEntities::default(),
            defaults: WorkbookDefaultParameters::default(),
        })
    }

    /// Open the specified  and globals file names
    pub fn open(
        workbook_file_name: Option<&PathBuf>,
        override_default_scenario: Option<String>,
        override_default_authorization: Option<String>,
        override_default_certificate: Option<String>,
        override_default_proxy: Option<String>,
        override_data_seed: Option<String>,
        allowed_data_path: &Path,
    ) -> Result<Workspace, ApicizeError> {
        // Open workbook
        let mut workbook: Workbook = match workbook_file_name {
            Some(input_file_name) => open_data_file(input_file_name),
            None => open_data_stream("STDIN".to_string(), &mut stdin()),
        }?
        .data;

        // Load private parameters if file exists
        let private_parameters = match workbook_file_name {
            Some(input_file_name) => Parameters::open(
                &Parameters::get_workbook_vault_filename(input_file_name),
                true,
            )?,
            None => Parameters::default(),
        };

        // Load globals if file exists
        let global_parameters = Parameters::open(&Parameters::get_globals_filename(), true)?;

        if workbook.defaults.is_none() {
            workbook.defaults = Some(WorkbookDefaultParameters::default());
        }

        if let Some(s) = Self::find_selection(
            &override_default_scenario,
            vec![
                &workbook.scenarios,
                &private_parameters.scenarios,
                &global_parameters.scenarios,
            ],
            "scenario",
        )? {
            workbook.defaults.as_mut().unwrap().selected_scenario = Some(s);
        }

        if let Some(s) = Self::find_selection(
            &override_default_authorization,
            vec![
                &workbook.authorizations,
                &private_parameters.authorizations,
                &global_parameters.authorizations,
            ],
            "authorization",
        )? {
            workbook.defaults.as_mut().unwrap().selected_authorization = Some(s);
        }

        if let Some(s) = Self::find_selection(
            &override_default_certificate,
            vec![
                &workbook.certificates,
                &private_parameters.certificates,
                &global_parameters.certificates,
            ],
            "certificate",
        )? {
            workbook.defaults.as_mut().unwrap().selected_certificate = Some(s);
        }

        if let Some(s) = Self::find_selection(
            &override_default_proxy,
            vec![
                &workbook.proxies,
                &private_parameters.proxies,
                &global_parameters.proxies,
            ],
            "proxy",
        )? {
            workbook.defaults.as_mut().unwrap().selected_proxy = Some(s);
        }

        // Commnad line seed may be an ID, a name or a file name
        if let Some(seed) = override_data_seed {
            let mut found_data = false;

            if let Some(workbook_data) = workbook.data.as_mut()
                && let Some(id) = workbook_data.iter().find_map(|d| {
                    if d.id == seed || d.name == seed {
                        Some(d.id.clone())
                    } else {
                        None
                    }
                })
            {
                // writeln!(feedback, "Using seed entry \"{}\"", seed.white()).unwrap();

                workbook.defaults.as_mut().unwrap().selected_data = Some(Selection {
                    id,
                    name: "Command line seed".to_string(),
                });
                found_data = true;
            }

            if !found_data {
                let full_seed_name = allowed_data_path.join(&seed);
                if full_seed_name.is_file() {
                    // writeln!(
                    //     feedback,
                    //     "{}",
                    //     format!("Using seed entry \"{}\"", &seed).white()
                    // )
                    // .unwrap();

                    let ext = full_seed_name
                        .extension()
                        .unwrap_or(OsStr::new(""))
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    let source_type = match ext.as_str() {
                        "json" => DataSourceType::FileJSON,
                        "csv" => DataSourceType::FileCSV,
                        _ => {
                            return Err(ApicizeError::Error {
                                description: format!(
                                    "Error: seed file \"{seed}\" does not end with .csv or .json"
                                ),
                            });
                        }
                    };

                    let default_data = DataSet {
                        source_type,
                        source: seed,
                        ..Default::default()
                    };

                    match workbook.data.as_mut() {
                        Some(data) => {
                            data.insert(0, default_data);
                        }
                        None => {
                            workbook.data = Some(vec![default_data]);
                        }
                    }

                    workbook.defaults.as_mut().unwrap().selected_data = Some(Selection {
                        id: "\0".to_string(),
                        name: "Command line seed".to_string(),
                    });
                } else {
                    return Err(ApicizeError::FileAccess {
                        description: "seed file not found".to_string(),
                        file_name: Some(seed),
                    });
                }
            }
        }

        Self::build_workspace(workbook, private_parameters, global_parameters)
    }

    /// Return matching selection (if any)
    fn find_selection<T: Identifiable>(
        requested_selection: &Option<String>,
        all_entities: Vec<&Option<Vec<T>>>,
        label: &str,
    ) -> Result<Option<Selection>, ApicizeError> {
        match requested_selection {
            Some(selection) => {
                if selection.is_empty() {
                    Ok(None)
                } else {
                    for entities in all_entities.iter().filter_map(|f| match f {
                        Some(f1) => Some(f1),
                        None => None,
                    }) {
                        let matching = entities.iter().find_map(|e| {
                            let id = e.get_id();
                            let name = e.get_name();
                            if id == selection || name == selection {
                                Some(Selection {
                                    id: id.to_string(),
                                    name: name.to_string(),
                                })
                            } else {
                                None
                            }
                        });
                        if matching.is_some() {
                            return Ok(matching);
                        }
                    }

                    Err(ApicizeError::Error {
                        description: format!("Unable to locate {label} \"{selection}\"")
                            .to_string(),
                    })
                }
            }
            &None => Ok(None),
        }
    }

    /// Build a workspace based upon the workbook file, private params file and global params
    fn build_workspace(
        workbook: Workbook,
        private_parameters: Parameters,
        global_parameters: Parameters,
    ) -> Result<Workspace, ApicizeError> {
        let workspace_requests = workbook
            .requests
            .into_iter()
            .map(|r| r.to_workspace())
            .collect::<Vec<RequestEntry>>();

        let mut workspace = Workspace {
            requests: IndexedEntities::<RequestEntry>::new(&workspace_requests),
            scenarios: IndexedEntities::<Scenario>::new(
                workbook.scenarios,
                private_parameters.scenarios,
                global_parameters.scenarios,
            ),
            authorizations: IndexedEntities::<Authorization>::new(
                workbook.authorizations,
                private_parameters.authorizations,
                global_parameters.authorizations,
            ),
            certificates: IndexedEntities::<Certificate>::new(
                workbook.certificates,
                private_parameters.certificates,
                global_parameters.certificates,
            ),
            proxies: IndexedEntities::<Proxy>::new(
                workbook.proxies,
                private_parameters.proxies,
                global_parameters.proxies,
            ),
            data: IndexedEntities::<DataSet>::new(workbook.data),
            defaults: workbook.defaults.unwrap_or_default(),
        };

        workspace.perform_all_validations();

        Ok(workspace)
    }

    pub fn perform_all_validations(&mut self) {
        self.requests
            .entities
            .values_mut()
            .for_each(|entity| entity.perform_validation());

        self.scenarios
            .entities
            .values_mut()
            .for_each(|entity| entity.perform_validation());

        self.authorizations
            .entities
            .values_mut()
            .for_each(|entity| entity.perform_validation());

        self.certificates
            .entities
            .values_mut()
            .for_each(|entity| entity.perform_validation());

        self.proxies
            .entities
            .values_mut()
            .for_each(|entity| entity.perform_validation());

        self.data
            .entities
            .values_mut()
            .for_each(|entity| entity.perform_validation());

        self.validate_selections();
    }

    pub fn validate_selections(&mut self) -> Option<InvalidSelections> {
        let selectables = self.get_selectables();
        let invalid_request_ids = self
            .requests
            .entities
            .values_mut()
            .filter_map(|e| {
                if selectables.validate_request_or_group(e) {
                    None
                } else {
                    Some(e.get_id().to_string())
                }
            })
            .collect::<Vec<String>>();
        let invalid_auth_ids = self
            .authorizations
            .entities
            .values_mut()
            .filter_map(|e| {
                if selectables.validate_authorization(e) {
                    None
                } else {
                    Some(e.get_id().to_string())
                }
            })
            .collect::<Vec<String>>();

        let defaults_valid = !selectables.validate_request_or_group(&mut self.defaults);

        if !invalid_request_ids.is_empty() || !invalid_auth_ids.is_empty() || !defaults_valid {
            Some(InvalidSelections {
                request_or_group_ids: invalid_request_ids,
                authorization_ids: invalid_auth_ids,
                defaults: defaults_valid,
            })
        } else {
            None
        }
    }

    fn get_selectables(&self) -> SelectableParameters {
        SelectableParameters {
            scenarios: self
                .scenarios
                .entities
                .values()
                .map(|e| (e.id.clone(), e.name.clone()))
                .collect::<HashMap<String, String>>(),
            authorizations: self
                .authorizations
                .entities
                .values()
                .map(|e| (e.get_id().to_string(), e.get_name().to_string()))
                .collect::<HashMap<String, String>>(),
            certificates: self
                .certificates
                .entities
                .values()
                .map(|e| (e.get_id().to_string(), e.get_name().to_string()))
                .collect::<HashMap<String, String>>(),
            proxies: self
                .proxies
                .entities
                .values()
                .map(|e| (e.id.clone(), e.name.clone()))
                .collect::<HashMap<String, String>>(),
            data: self
                .data
                .entities
                .values()
                .map(|e| (e.id.clone(), e.name.clone()))
                .collect::<HashMap<String, String>>(),
        }
    }

    /// Save workspace to specified path, including private and global parameters
    pub fn save(
        &self,
        workbook_path: &PathBuf,
    ) -> Result<Vec<SerializationSaveSuccess>, ApicizeError> {
        fn clone_to_storage<T: Validated + Clone>(parameter: &T) -> T {
            let mut cloned = parameter.clone();
            cloned.set_validation_errors(None);
            cloned.set_validation_warnings(None);
            cloned
        }

        let mut successes: Vec<SerializationSaveSuccess> = vec![];

        let stored_scenarios = match self.scenarios.get_workbook() {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.iter().map(clone_to_storage).collect())
                }
            }
            None => None,
        };

        let stored_authorizations: Option<Vec<Authorization>> =
            match self.authorizations.get_workbook() {
                Some(entities) => {
                    if entities.is_empty() {
                        None
                    } else {
                        // Don't save selected certificates or proxies set to None
                        Some(
                            entities
                                .iter()
                                .map(|auth| {
                                    let mut auth = clone_to_storage(auth);
                                    if let Authorization::OAuth2Client {
                                        ref mut selected_certificate,
                                        ref mut selected_proxy,
                                        ..
                                    } = auth
                                    {
                                        if selected_certificate
                                            .as_ref()
                                            .map(|s| s.id == NO_SELECTION_ID)
                                            .unwrap_or(false)
                                        {
                                            *selected_certificate = None;
                                        }
                                        if selected_proxy
                                            .as_ref()
                                            .map(|s| s.id == NO_SELECTION_ID)
                                            .unwrap_or(false)
                                        {
                                            *selected_proxy = None;
                                        }
                                    }
                                    auth
                                })
                                .collect(),
                        )
                    }
                }
                None => None,
            };

        let stored_certiificates = match self.certificates.get_workbook() {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.iter().map(clone_to_storage).collect())
                }
            }
            None => None,
        };

        let stored_proxies = match self.proxies.get_workbook() {
            Some(entities) => {
                if entities.is_empty() {
                    None
                } else {
                    Some(entities.iter().map(clone_to_storage).collect())
                }
            }
            None => None,
        };

        let stored_defaults = match self.defaults.any_values_set() {
            true => {
                // Do not save default selections explicitly set to None
                let mut defaults = clone_to_storage(&self.defaults);
                if defaults
                    .selected_scenario
                    .as_ref()
                    .map(|s| s.id == NO_SELECTION_ID)
                    .unwrap_or(false)
                {
                    defaults.selected_scenario = None;
                }
                if defaults
                    .selected_authorization
                    .as_ref()
                    .map(|s| s.id == NO_SELECTION_ID)
                    .unwrap_or(false)
                {
                    defaults.selected_authorization = None;
                }
                if defaults
                    .selected_certificate
                    .as_ref()
                    .map(|s| s.id == NO_SELECTION_ID)
                    .unwrap_or(false)
                {
                    defaults.selected_certificate = None;
                }
                if defaults
                    .selected_proxy
                    .as_ref()
                    .map(|s| s.id == NO_SELECTION_ID)
                    .unwrap_or(false)
                {
                    defaults.selected_proxy = None;
                }
                if defaults
                    .selected_data
                    .as_ref()
                    .map(|s| s.id == NO_SELECTION_ID)
                    .unwrap_or(false)
                {
                    defaults.selected_data = None;
                }
                Some(defaults)
            }
            false => None,
        };

        let stored_requests = self
            .requests
            .to_entities()
            .into_iter()
            .map(StoredRequestEntry::from_workspace)
            .collect();

        let mut stored_data = self
            .data
            .entities
            .values()
            .map(|ds| {
                let mut ds = ds.clone();
                ds.source_error = None;
                ds.validation_warnings = None;
                ds.validation_errors = None;
                ds
            })
            .collect::<Vec<DataSet>>();

        stored_data.sort_by(|a, b| {
            let idx_a = self
                .data
                .top_level_ids
                .iter()
                .position(|i| i == &a.id)
                .unwrap_or(0);
            let idx_b = self
                .data
                .top_level_ids
                .iter()
                .position(|i| i == &b.id)
                .unwrap_or(0);
            idx_a.cmp(&idx_b)
        });

        let workbook = Workbook {
            version: 1.0,
            requests: stored_requests,
            scenarios: stored_scenarios,
            authorizations: stored_authorizations,
            certificates: stored_certiificates,
            proxies: stored_proxies,
            data: if stored_data.is_empty() {
                None
            } else {
                Some(stored_data)
            },
            defaults: stored_defaults,
        };

        match save_data_file(workbook_path, &workbook) {
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

        let mut current = request;

        let mut scenario: Option<&Scenario> = None;
        let mut authorization: Option<&Authorization> = None;
        let mut certificate: Option<&Certificate> = None;
        let mut proxy: Option<&Proxy> = None;
        let mut data: Option<&DataSet> = None;

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
                match self.data.find(current.selected_data()) {
                    SelectedOption::UseDefault => {}
                    SelectedOption::Off => {
                        allow_data = false;
                    }
                    SelectedOption::Some(p) => {
                        data = Some(p);
                    }
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
        if scenario.is_none()
            && allow_scenario
            && let SelectedOption::Some(s) = self.scenarios.find(&self.defaults.selected_scenario)
        {
            scenario = Some(s);
        }

        if authorization.is_none()
            && allow_authorization
            && let SelectedOption::Some(a) = self
                .authorizations
                .find(&self.defaults.selected_authorization)
        {
            authorization = Some(a);
        }

        if certificate.is_none()
            && allow_certificate
            && let SelectedOption::Some(c) =
                self.certificates.find(&self.defaults.selected_certificate)
        {
            certificate = Some(c);
        }

        if proxy.is_none()
            && allow_proxy
            && let SelectedOption::Some(p) = self.proxies.find(&self.defaults.selected_proxy)
        {
            proxy = Some(p);
        }

        if data.is_none()
            && allow_data
            && let SelectedOption::Some(p) = self.data.find(&self.defaults.selected_data)
        {
            data = Some(p);
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
                Some(Arc::new(variables))
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

    /// Append specified index, including children, to the results
    fn generate_json(
        exec_ctr: &usize,
        summaries: &IndexMap<usize, ExecutionResultSummary>,
        report: &mut Vec<ExecutionReportJson>,
    ) -> Result<(), ApicizeError> {
        match summaries.get(exec_ctr) {
            Some(summary) => {
                if summary.error.is_some() {
                    // Deal with summaries with errors
                    report.push(ExecutionReportJson::from_summary(summary, None, None));
                    Ok(())
                } else if let Some(child_indexes) = &summary.child_exec_ctrs {
                    let children = if child_indexes.is_empty() {
                        None
                    } else {
                        let mut children = Vec::<ExecutionReportJson>::new();
                        for child_index in child_indexes {
                            Self::generate_json(child_index, summaries, &mut children)?;
                        }
                        Some(children)
                    };
                    report.push(ExecutionReportJson::from_summary(summary, children, None));
                    Ok(())
                } else {
                    // Deal with executed behavior results
                    report.push(ExecutionReportJson::from_summary(
                        summary,
                        None,
                        summary.test_results.clone(),
                    ));

                    Ok(())
                }
            }
            None => Err(ApicizeError::Error {
                description: format!("Invalid execution counter ({exec_ctr})").to_string(),
            }),
        }
    }

    // Append specified index, including children, to the results
    fn generate_csv(
        exec_ctr: &usize,
        summaries: &IndexMap<usize, ExecutionResultSummary>,
        parent_names: &[&str],
        report: &mut Vec<ExecutionReportCsv>,
        run_number: usize,
    ) -> Result<(), ApicizeError> {
        match summaries.get(exec_ctr) {
            Some(summary) => {
                let mut name_parts = Vec::from(parent_names);
                let is_first = parent_names.is_empty();
                let name_part = if !is_first
                    && let Some(row_number) = summary.row_number
                    && let Some(row_count) = summary.row_count
                {
                    &format!("Row {row_number} of {row_count}")
                } else if !is_first
                    && let Some(run_number) = summary.run_number
                    && let Some(run_count) = summary.run_count
                {
                    &format!("Run {run_number} of {run_count}")
                } else {
                    &summary.name
                };

                name_parts.push(name_part);

                if summary.error.is_some() {
                    // Deal with summaries with errors
                    report.push(ExecutionReportCsv {
                        run_number,
                        name: name_parts.join(", "),
                        key: summary.key.clone(),
                        executed_at: summary.executed_at,
                        duration: summary.duration,
                        method: summary.method.clone(),
                        url: summary.url.clone(),
                        success: summary.success.clone(),
                        status: summary.status,
                        status_text: summary.status_text.clone(),
                        test_name: None,
                        test_tag: None,
                        test_success: None,
                        test_logs: None,
                        test_error: None,
                        error: summary.error.clone(),
                    });
                } else if let Some(child_indexes) = &summary.child_exec_ctrs
                    && !child_indexes.is_empty()
                {
                    // Deal with "parent" scenarois
                    for child_index in child_indexes {
                        Self::generate_csv(
                            child_index,
                            summaries,
                            &name_parts,
                            report,
                            run_number,
                        )?;
                    }
                } else if let Some(test_results) = &summary.test_results {
                    // Deal with executed behavior results with tests
                    for test_result in test_results {
                        report.push(ExecutionReportCsv {
                            run_number,
                            name: name_parts.join(", "),
                            key: summary.key.clone(),
                            executed_at: summary.executed_at,
                            duration: summary.duration,
                            method: summary.method.clone(),
                            url: summary.url.clone(),
                            success: summary.success.clone(),
                            status: summary.status,
                            status_text: summary.status_text.clone(),
                            error: summary.error.clone(),
                            test_name: Some(test_result.name.clone()),
                            test_tag: test_result.tag.clone(),
                            test_success: Some(test_result.success),
                            test_logs: test_result.logs.as_ref().map(|l| l.join("; ")),
                            test_error: test_result.error.clone(),
                        });
                    }
                } else {
                    // Deal with executed behavior results without tests
                    report.push(ExecutionReportCsv {
                        run_number,
                        name: name_parts.join(", "),
                        key: summary.key.clone(),
                        executed_at: summary.executed_at,
                        duration: summary.duration,
                        method: summary.method.clone(),
                        url: summary.url.clone(),
                        success: summary.success.clone(),
                        status: summary.status,
                        status_text: summary.status_text.clone(),
                        error: summary.error.clone(),
                        test_name: None,
                        test_tag: None,
                        test_success: None,
                        test_logs: None,
                        test_error: None,
                    });
                }
                Ok(())
            }
            None => Err(ApicizeError::Error {
                description: format!("Invalid execution counter ({exec_ctr})").to_string(),
            }),
        }
    }

    /// Generate a report from summarized execution results
    pub fn geneate_report(
        exec_ctr: &usize,
        summaries: &IndexMap<usize, ExecutionResultSummary>,
        format: ExecutionReportFormat,
    ) -> Result<String, ApicizeError> {
        match format {
            ExecutionReportFormat::JSON => {
                let mut data = Vec::<ExecutionReportJson>::new();
                Self::generate_json(exec_ctr, summaries, &mut data)?;
                let mut buf = Vec::new();
                let formatter = PrettyFormatter::with_indent(b"    ");
                let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                data.serialize(&mut ser).unwrap();
                Ok(String::from_utf8(buf).unwrap())
            }
            ExecutionReportFormat::CSV => {
                let mut data = Vec::<ExecutionReportCsv>::new();
                Self::generate_csv(exec_ctr, summaries, &[], &mut data, 1)?;
                Self::generate_csv_text(data, false)
            }
        }
    }

    /// Generate a report from summarized execution results
    pub fn generate_multirun_report(
        all_run_summaries: &IndexMap<usize, IndexMap<usize, ExecutionResultSummary>>,
        format: &ExecutionReportFormat,
    ) -> Result<String, ApicizeError> {
        match format {
            ExecutionReportFormat::JSON => {
                let mut all_data = HashMap::<usize, Vec<ExecutionReportJson>>::new();

                for (run_number, run_summaries) in all_run_summaries {
                    let mut data = Vec::<ExecutionReportJson>::new();
                    Self::generate_json(run_number, run_summaries, &mut data)?;
                    if let Some(entry) = all_data.get_mut(run_number) {
                        entry.extend(data);
                    } else {
                        all_data.insert(*run_number, data);
                    }
                }

                let mut buf = Vec::new();
                let formatter = PrettyFormatter::with_indent(b"    ");
                let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                all_data.serialize(&mut ser).unwrap();
                Ok(String::from_utf8(buf).unwrap())
            }
            ExecutionReportFormat::CSV => {
                let entry_count: usize = all_run_summaries
                    .values()
                    .fold(0, |total, summaries| total + summaries.len());
                let mut run_data = Vec::<ExecutionReportCsv>::with_capacity(entry_count);

                for (run_number, run_summaries) in all_run_summaries {
                    for index in run_summaries.keys() {
                        Self::generate_csv(index, run_summaries, &[], &mut run_data, *run_number)?;
                    }
                }

                Self::generate_csv_text(run_data, all_run_summaries.len() > 1)
            }
        }
    }

    fn generate_csv_text(
        run_data: Vec<ExecutionReportCsv>,
        multi_run: bool,
    ) -> Result<String, ApicizeError> {
        let mut writer = WriterBuilder::new().from_writer(Vec::new());
        if multi_run {
            for d in run_data {
                if let Err(err) = writer.serialize(d) {
                    return Err(ApicizeError::Error {
                        description: format!("{}", &err),
                    });
                }
            }
        } else {
            for d in run_data
                .into_iter()
                .map(|d| ExecutionReportCsvSingleRun::from(d))
            {
                if let Err(err) = writer.serialize(d) {
                    return Err(ApicizeError::Error {
                        description: format!("{}", &err),
                    });
                }
            }
        }

        Ok(String::from_utf8(writer.into_inner().unwrap()).unwrap())
    }
}

/// Parameters to use when executing a request/group,
/// these should not change during execution
#[derive(Clone, Default)]
pub struct RequestExecutionParameters {
    pub data_set: Arc<Option<RequestDataSet>>,
    pub data_enabled: bool,
    pub variables: Option<Arc<Map<String, Value>>>,
    pub authorization_id: Option<String>,
    pub certificate_id: Option<String>,
    pub proxy_id: Option<String>,
    pub auth_certificate_id: Option<String>,
    pub auth_proxy_id: Option<String>,
}

/// Thse values may change during the execution of a request/group
#[derive(Default, Clone)]
pub struct RequestExecutionState {
    pub row: Option<Arc<RequestDataRow>>,
    pub output_variables: Option<Arc<RequestDataRow>>,
}

pub type RequestDataRow = Map<String, Value>;
pub struct RequestDataSet {
    pub id: String,
    pub data: Vec<RequestDataRow>,
}

/// State of a selected option
pub enum SelectedOption<T> {
    /// Use default parent selection (if available)
    UseDefault,
    /// Do not send a value for this selection
    Off,
    /// Use this value
    Some(T),
}

/// List of all invalid selections (requests, groups or auths with
/// selections that are no longer valid)
pub struct InvalidSelections {
    pub request_or_group_ids: Vec<String>,
    pub authorization_ids: Vec<String>,
    pub defaults: bool,
}

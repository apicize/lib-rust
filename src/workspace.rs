//! Workspace models submodule
//!
//! This submodule defines modules used to manage workspaces

use crate::{
    apicize::{ApicizeExecution, ApicizeExecutionItem, ExecutionTotals},
    run_request_item,
};

use super::{
    open_data_file, workbook::*, Identifable, Parameters, SelectableOptions, SerializationFailure,
    SerializationSaveSuccess,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tokio::{select, task::JoinSet};
use tokio_util::sync::CancellationToken;

const NO_SELECTION_ID: &str = "\tNONE\t";

/// Trait representing parameter entity with a unique identifier
pub trait WorkspaceParameter<T> {
    /// Get persistence
    fn get_persistence(&self) -> Option<Persistence>;

    /// Set persistence
    fn set_persistence(&mut self, persistence_to_set: Persistence);

    /// Set persistence
    fn clear_persistence(&mut self);
}

/// Entity that has warnings that should be shown to user upon access
pub trait Warnings {
    /// Retrieve warnings
    fn get_warnings(&self) -> &Option<Vec<String>>;

    /// Set warnings
    fn add_warning(&mut self, warning: String);
}

/// Generic for indexed, ordered entities, optionally with children
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedRequests {
    /// Top level entity IDs
    pub top_level_ids: Vec<String>,

    /// Map of parent to child entity IDs
    pub child_ids: Option<HashMap<String, Vec<String>>>,

    /// Entities indexed by ID
    pub entities: HashMap<String, WorkbookRequestEntry>,
}

/// Generic for indexed, ordered entities
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedEntities<T> {
    /// Top level entity IDs
    pub top_level_ids: Vec<String>,

    /// Entities indexed by ID
    pub entities: HashMap<String, T>,
}

/// Implemented IndexEntry methods
impl<T: Identifable> IndexedEntities<T> {
    /// Find a match based upon ID or name
    pub fn find_match(&self, selection: &Selection) -> bool {
        selection.id == NO_SELECTION_ID
            || self.entities.contains_key(&selection.id)
            || self
                .entities
                .values()
                .any(|e| e.get_name().to_lowercase() == selection.name.to_lowercase())
    }
}

/// Parameters applicable to a request
pub struct RequestParameters<'a> {
    /// Variables (inherited from scenario) for the request
    pub variables: HashMap<String, Value>,
    /// Applicable authorization (if any) for the request
    pub authorization: Option<&'a WorkbookAuthorization>,
    /// Applicable certificate (if any) for the request
    pub certificate: Option<&'a WorkbookCertificate>,
    /// Applicable proxy (if any) for the request
    pub proxy: Option<&'a WorkbookProxy>,
    /// Applicable certificate (if any) for the request's authorization
    pub auth_certificate: Option<&'a WorkbookCertificate>,
    /// Applicable proxy (if any) for the request's authorization
    pub auth_proxy: Option<&'a WorkbookProxy>,
}

/// Data type for entities used by Apicize during testing and editing.  This will be
/// the combination of workbook, workbook credential and global settings values
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// Requests for the workspace
    pub requests: IndexedRequests,

    /// Scenarios for the workspace
    pub scenarios: IndexedEntities<WorkbookScenario>,

    /// Authorizations for the workspace
    pub authorizations: IndexedEntities<WorkbookAuthorization>,

    /// Certificates for the workspace
    pub certificates: IndexedEntities<WorkbookCertificate>,

    /// Proxies for the workspace
    pub proxies: IndexedEntities<WorkbookProxy>,

    /// Default values for requests and groups
    pub defaults: Option<WorkbookDefaults>,

    /// Warnings regarding workspace
    pub warnings: Option<Vec<String>>,
}

impl Workspace {
    fn validate_workbook_defaults(&mut self) {
        if self.defaults.is_none() {
            return;
        }

        if let Some(selection) = self.get_selected_scenario() {
            let ok = self.scenarios.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Default selected scenario {} not found, defaulting to Off",
                    selection.get_title(),
                ));
                self.set_selected_scenario(Some(Selection {
                    id: String::from(NO_SELECTION_ID),
                    name: String::from("Off"),
                }));
            }
        }

        if let Some(selection) = self.get_selected_authorization() {
            let ok = self.authorizations.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Default authorization scenario {} not found, defaulting to Off",
                    selection.get_title(),
                ));
                self.set_selected_authorization(Some(Selection {
                    id: String::from(NO_SELECTION_ID),
                    name: String::from("Off"),
                }));
            }
        }

        if let Some(selection) = self.get_selected_certificate() {
            let ok = self.scenarios.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Default selected certificate {} not found, defaulting to Off",
                    selection.get_title(),
                ));
                self.set_selected_certificate(Some(Selection {
                    id: String::from(NO_SELECTION_ID),
                    name: String::from("Off"),
                }));
            }
        }

        if let Some(selection) = self.get_selected_proxy() {
            let ok = self.scenarios.find_match(selection);
            if !ok {
                self.add_warning(format!(
                    "Default selected proxy {} not found, defaulting to Off",
                    selection.get_title(),
                ));
                self.set_selected_proxy(Some(Selection {
                    id: String::from(NO_SELECTION_ID),
                    name: String::from("Off"),
                }));
            }
        }
    }

    /// Populate indexes
    fn populate_indexes<T: Clone + Identifable + WorkspaceParameter<T>>(
        entities: &Option<Vec<T>>,
        index: &mut IndexedEntities<T>,
        persistence: Persistence,
    ) {
        if let Some(existing) = entities {
            for e in existing {
                let id = e.get_id();
                let mut cloned = e.clone();
                if !index.top_level_ids.contains(id) {
                    index.top_level_ids.push(id.to_string());
                }
                cloned.set_persistence(persistence);
                index.entities.insert(id.to_string(), cloned);
            }
        }
    }

    /// Populate the workspace request list
    fn populate_requests(
        entities: &mut [WorkbookRequestEntry],
        indexed_requests: &mut IndexedRequests,
        parent_id: Option<String>,
        scenarios: &IndexedEntities<WorkbookScenario>,
        authorizations: &IndexedEntities<WorkbookAuthorization>,
        certificates: &IndexedEntities<WorkbookCertificate>,
        proxies: &IndexedEntities<WorkbookProxy>,
    ) {
        let active_parent_id = parent_id.unwrap_or(String::from(""));
        for e in entities.iter_mut() {
            e.validate_parameters(scenarios, authorizations, certificates, proxies);

            match e {
                WorkbookRequestEntry::Info(info) => {
                    if active_parent_id.is_empty() {
                        indexed_requests.top_level_ids.push(info.id.clone());
                    } else {
                        match indexed_requests.child_ids.as_mut() {
                            Some(existing) => {
                                let updated_child_ids = match existing.get(&active_parent_id) {
                                    Some(matching_group) => {
                                        let mut updated = matching_group.to_vec();
                                        updated.push(info.id.clone());
                                        updated
                                    }
                                    None => Vec::from([info.id.clone()]),
                                };
                                existing.insert(active_parent_id.clone(), updated_child_ids);
                            }
                            None => {
                                indexed_requests.child_ids = Some(HashMap::from([(
                                    active_parent_id.clone(),
                                    Vec::from([info.id.clone()]),
                                )]));
                            }
                        }
                    }
                    indexed_requests
                        .entities
                        .insert(info.id.clone(), WorkbookRequestEntry::Info(info.clone()));
                }
                WorkbookRequestEntry::Group(group) => {
                    if active_parent_id.is_empty() {
                        indexed_requests.top_level_ids.push(group.id.clone());
                    } else {
                        match indexed_requests.child_ids.as_mut() {
                            Some(existing) => {
                                let updated_child_ids = match existing.get(&active_parent_id) {
                                    Some(matching_group) => {
                                        let mut updated = matching_group.to_vec();
                                        updated.push(group.id.clone());
                                        updated
                                    }
                                    None => Vec::from([group.id.clone()]),
                                };
                                existing.insert(active_parent_id.clone(), updated_child_ids);
                            }
                            None => {
                                indexed_requests.child_ids = Some(HashMap::from([(
                                    active_parent_id.clone(),
                                    Vec::from([group.id.clone()]),
                                )]));
                            }
                        }
                    }

                    let mut cloned_group = group.clone();
                    cloned_group.children = None;
                    indexed_requests
                        .entities
                        .insert(group.id.clone(), WorkbookRequestEntry::Group(cloned_group));

                    if let Some(children) = group.children.as_mut() {
                        Self::populate_requests(
                            children,
                            indexed_requests,
                            Some(group.id.clone()),
                            scenarios,
                            authorizations,
                            certificates,
                            proxies,
                        );

                        group.children = None;
                    }
                }
            };
        }
    }

    /// Find entity (Scenario, Authorization, etc.)
    pub fn find_matching_selection<'a, T: WorkspaceParameter<T> + Identifable>(
        selection: &Option<Selection>,
        list: &'a IndexedEntities<T>,
    ) -> WorkbookSelectedOption<&'a T> {
        match selection {
            Some(s) => {
                if s.id == NO_SELECTION_ID {
                    WorkbookSelectedOption::Off
                } else if let Some(found) = list.entities.get(&s.id) {
                    WorkbookSelectedOption::Some(found)
                } else {
                    match list.entities.values().find(|v| {
                        let name = v.get_name();
                        name.eq_ignore_ascii_case(&s.name)
                    }) {
                        Some(found_by_name) => WorkbookSelectedOption::Some(found_by_name),
                        None => WorkbookSelectedOption::UseDefault,
                    }
                }
            }
            None => WorkbookSelectedOption::UseDefault,
        }
    }

    /// Find matching scenario, if any
    pub fn find_scenario(
        &self,
        selection: &Option<Selection>,
    ) -> WorkbookSelectedOption<&WorkbookScenario> {
        Workspace::find_matching_selection(selection, &self.scenarios)
    }

    /// Create a new workspace, including globals specified (if any)
    pub fn new(
        globals_filename: Option<PathBuf>,
    ) -> Result<Workspace, SerializationFailure> {
        let wkspc_requests = IndexedRequests {
            top_level_ids: vec![],
            child_ids: None,
            entities: HashMap::new(),
        };
        let mut wkspc_scenarios = IndexedEntities::<WorkbookScenario> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };
        let mut wkspc_authorizations = IndexedEntities::<WorkbookAuthorization> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };
        let mut wkspc_certificates = IndexedEntities::<WorkbookCertificate> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };
        let mut wkspc_proxies = IndexedEntities::<WorkbookProxy> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };

        // Open globals using either the specified file name or falling back to the default name,
        match Parameters::open_global_parameters(globals_filename) {
            Ok(success) => {
                let globals = success.data;
                // Populate entries from global storage
                Self::populate_indexes(
                    &globals.scenarios,
                    &mut wkspc_scenarios,
                    Persistence::Global,
                );
                Self::populate_indexes(
                    &globals.authorizations,
                    &mut wkspc_authorizations,
                    Persistence::Global,
                );
                Self::populate_indexes(
                    &globals.certificates,
                    &mut wkspc_certificates,
                    Persistence::Global,
                );
                Self::populate_indexes(&globals.proxies, &mut wkspc_proxies, Persistence::Global);
                
                let mut workspace = Workspace {
                    requests: wkspc_requests,
                    scenarios: wkspc_scenarios,
                    authorizations: wkspc_authorizations,
                    certificates: wkspc_certificates,
                    proxies: wkspc_proxies,
                    defaults: None,
                    warnings: None,
                };

                // Validate the default workbook scenarios, etc. selected for testing
                workspace.validate_workbook_defaults();

                // for request in wkspc_requests.entities.values() {
                // }

                Ok(workspace)                
            }
            Err(failure) => Err(failure),
        }
    }

    /// Open the specified workbook and globals file names
    pub fn open_from_file(
        workbook_file_name: &PathBuf,
        globals_filename: Option<PathBuf>,
    ) -> Result<Workspace, SerializationFailure> {
        // Open workbook
        let mut wkbk: Workbook;
        match open_data_file(workbook_file_name) {
            Ok(success) => {
                wkbk = success.data;
            }
            Err(error) => {
                return Err(error);
            }
        }

        // Load private parameters if file exists
        let privates: Option<Parameters>;
        let mut private_path = workbook_file_name.clone();
        private_path.set_extension("apicize-priv");
        if Path::new(&private_path).is_file() {
            match Parameters::open_workbook_private_parameters(&private_path) {
                Ok(success) => {
                    privates = Some(success.data);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        } else {
            privates = None;
        }

        Self::open_from_workbook(&mut wkbk, privates, globals_filename)
    }

    /// Open a workspace using the specified workbook, taking into account private parameters file (if existing)
    /// and global settings
    pub fn open_from_workbook(
        wkbk: &mut Workbook,
        privates: Option<Parameters>,
        globals_filename: Option<PathBuf>,
    ) -> Result<Workspace, SerializationFailure> {
        let mut wkspc_requests = IndexedRequests {
            top_level_ids: vec![],
            child_ids: None,
            entities: HashMap::new(),
        };
        let mut wkspc_scenarios = IndexedEntities::<WorkbookScenario> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };
        let mut wkspc_authorizations = IndexedEntities::<WorkbookAuthorization> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };
        let mut wkspc_certificates = IndexedEntities::<WorkbookCertificate> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };
        let mut wkspc_proxies = IndexedEntities::<WorkbookProxy> {
            top_level_ids: vec![],
            entities: HashMap::new(),
        };

        // Open globals using either the specified file name or falling back to the default name,
        match Parameters::open_global_parameters(globals_filename) {
            Ok(success) => {
                let globals = success.data;
                // Populate entries from global storage
                Self::populate_indexes(
                    &globals.scenarios,
                    &mut wkspc_scenarios,
                    Persistence::Global,
                );
                Self::populate_indexes(
                    &globals.authorizations,
                    &mut wkspc_authorizations,
                    Persistence::Global,
                );
                Self::populate_indexes(
                    &globals.certificates,
                    &mut wkspc_certificates,
                    Persistence::Global,
                );
                Self::populate_indexes(&globals.proxies, &mut wkspc_proxies, Persistence::Global);

                // Populate entries from private parameter files, if any
                if let Some(private) = privates {
                    Self::populate_indexes(
                        &private.scenarios,
                        &mut wkspc_scenarios,
                        Persistence::Private,
                    );
                    Self::populate_indexes(
                        &private.authorizations,
                        &mut wkspc_authorizations,
                        Persistence::Private,
                    );
                    Self::populate_indexes(
                        &private.certificates,
                        &mut wkspc_certificates,
                        Persistence::Private,
                    );
                    Self::populate_indexes(
                        &private.proxies,
                        &mut wkspc_proxies,
                        Persistence::Private,
                    );
                }

                Self::populate_indexes(
                    &wkbk.scenarios,
                    &mut wkspc_scenarios,
                    Persistence::Workbook,
                );

                Self::populate_indexes(
                    &wkbk.authorizations,
                    &mut wkspc_authorizations,
                    Persistence::Workbook,
                );

                Self::populate_indexes(
                    &wkbk.certificates,
                    &mut wkspc_certificates,
                    Persistence::Workbook,
                );

                Self::populate_indexes(&wkbk.proxies, &mut wkspc_proxies, Persistence::Workbook);

                Self::populate_requests(
                    &mut wkbk.requests,
                    &mut wkspc_requests,
                    None,
                    &wkspc_scenarios,
                    &wkspc_authorizations,
                    &wkspc_certificates,
                    &wkspc_proxies,
                );

                let mut workspace = Workspace {
                    requests: wkspc_requests,
                    scenarios: wkspc_scenarios,
                    authorizations: wkspc_authorizations,
                    certificates: wkspc_certificates,
                    proxies: wkspc_proxies,
                    defaults: wkbk.defaults.clone(),
                    warnings: None,
                };

                // Validate the default workbook scenarios, etc. selected for testing
                workspace.validate_workbook_defaults();

                // for request in wkspc_requests.entities.values() {
                // }

                Ok(workspace)
            }
            Err(failure) => Err(failure),
        }
    }

    /// Recursively add requests to the list to save
    fn build_requests(
        ids: &[String],
        indexed_requests: &IndexedRequests,
    ) -> Vec<WorkbookRequestEntry> {
        let mut results: Vec<WorkbookRequestEntry> = vec![];
        ids.iter().for_each(|id| {
            if let Some(entry) = indexed_requests.entities.get(id) {
                match entry {
                    WorkbookRequestEntry::Info(info) => {
                        results.push(WorkbookRequestEntry::Info(info.clone()));
                    }
                    WorkbookRequestEntry::Group(group) => {
                        let mut group_to_add = group.clone();
                        group_to_add.children = None;
                        if let Some(child_id_list) = indexed_requests.child_ids.as_ref() {
                            if let Some(child_ids) = child_id_list.get(id) {
                                let children = Self::build_requests(child_ids, indexed_requests);
                                if !children.is_empty() {
                                    group_to_add.children = Some(children);
                                }
                            }
                        }
                        results.push(WorkbookRequestEntry::Group(group_to_add));
                    }
                }
            }
        });
        results
    }

    // Add entities to the global, private and workbook lists, depending upon persistence
    fn append_entities<T: WorkspaceParameter<T> + Clone + Identifable>(
        ids: &[String],
        list: &IndexedEntities<T>,
        globals: &mut Vec<T>,
        private: &mut Vec<T>,
        workbook: &mut Vec<T>,
    ) {
        ids.iter().for_each(|id| {
            if let Some(entity) = list.entities.get(id) {
                let mut cloned_entity = entity.clone();
                // Clear the persistence value from the saved value, we don't need to save it
                cloned_entity.clear_persistence();
                match entity.get_persistence() {
                    Some(Persistence::Global) => globals.push(cloned_entity),
                    Some(Persistence::Private) => private.push(cloned_entity),
                    Some(Persistence::Workbook) => workbook.push(cloned_entity),
                    None => {}
                }
            }
        });
    }

    /// Save workspace to specified path, including workbook and global parameters
    pub fn save(
        &self,
        workbook_path: &PathBuf,
    ) -> Result<Vec<SerializationSaveSuccess>, SerializationFailure> {
        let mut global_scenarios: Vec<WorkbookScenario> = vec![];
        let mut global_authorizations: Vec<WorkbookAuthorization> = vec![];
        let mut global_certificates: Vec<WorkbookCertificate> = vec![];
        let mut global_proxies: Vec<WorkbookProxy> = vec![];

        let mut priv_scenarios: Vec<WorkbookScenario> = vec![];
        let mut priv_authorizations: Vec<WorkbookAuthorization> = vec![];
        let mut priv_certificates: Vec<WorkbookCertificate> = vec![];
        let mut priv_proxies: Vec<WorkbookProxy> = vec![];

        let mut wkbk_scenarios: Vec<WorkbookScenario> = vec![];
        let mut wkbk_authorizations: Vec<WorkbookAuthorization> = vec![];
        let mut wkbk_certificates: Vec<WorkbookCertificate> = vec![];
        let mut wkbk_proxies: Vec<WorkbookProxy> = vec![];

        let wkbk_requests = Self::build_requests(&self.requests.top_level_ids, &self.requests);

        Self::append_entities(
            &self.scenarios.top_level_ids,
            &self.scenarios,
            &mut global_scenarios,
            &mut priv_scenarios,
            &mut wkbk_scenarios,
        );
        Self::append_entities(
            &self.authorizations.top_level_ids,
            &self.authorizations,
            &mut global_authorizations,
            &mut priv_authorizations,
            &mut wkbk_authorizations,
        );
        Self::append_entities(
            &self.certificates.top_level_ids,
            &self.certificates,
            &mut global_certificates,
            &mut priv_certificates,
            &mut wkbk_certificates,
        );
        Self::append_entities(
            &self.proxies.top_level_ids,
            &self.proxies,
            &mut global_proxies,
            &mut priv_proxies,
            &mut wkbk_proxies,
        );

        let defaults = self.defaults.clone();

        let mut successes: Vec<SerializationSaveSuccess> = vec![];

        match Workbook::save_workbook(
            PathBuf::from(workbook_path),
            wkbk_requests,
            wkbk_scenarios,
            wkbk_authorizations,
            wkbk_certificates,
            wkbk_proxies,
            defaults,
        ) {
            Ok(success) => successes.push(success),
            Err(error) => return Err(error),
        }

        match Parameters::save_workbook_private_parameters(
            workbook_path,
            &priv_scenarios,
            &priv_authorizations,
            &priv_certificates,
            &priv_proxies,
        ) {
            Ok(success) => successes.push(success),
            Err(error) => return Err(error),
        }

        match Parameters::save_global_parameters(
            &global_scenarios,
            &global_authorizations,
            &global_certificates,
            &global_proxies,
        ) {
            Ok(success) => successes.push(success),
            Err(error) => return Err(error),
        }

        Ok(successes)
    }

    /// Dispatch requests/groups in the specified workspace, optionally forcing the number of runs.
    pub async fn run(
        workspace: Arc<Workspace>,
        request_ids: Option<Vec<String>>,
        cancellation_token: Option<CancellationToken>,
        tests_started: Arc<Instant>,
    ) -> Result<ApicizeExecution, String> {
        let cancellation = match cancellation_token {
            Some(t) => t,
            None => CancellationToken::new(),
        };

        let request_ids_to_execute =
            request_ids.unwrap_or(workspace.requests.top_level_ids.clone());

        let mut executing_items: JoinSet<Option<ApicizeExecutionItem>> = JoinSet::new();
        for request_id in request_ids_to_execute {
            let cloned_cancellation = cancellation.clone();
            let executed_item = run_request_item(
                workspace.clone(),
                cancellation.clone(),
                tests_started.clone(),
                request_id.clone(),
                Arc::new(HashMap::new()),
                None,
            );
            executing_items.spawn(async move {
                select! {
                    _ = cloned_cancellation.cancelled() => None,
                    result = executed_item => {
                        Some(result)
                    }
                }
            });
        }

        let completed_items = executing_items.join_all().await;
        let items: Vec<ApicizeExecutionItem> = completed_items.into_iter().flatten().collect();

        let mut result = ApicizeExecution {
            duration: tests_started.elapsed().as_millis(),
            items: vec![],
            success: true,
            requests_with_passed_tests_count: 0,
            requests_with_failed_tests_count: 0,
            requests_with_errors: 0,
            passed_test_count: 0,
            failed_test_count: 0,
        };

        for item in &items {
            result.add_totals(item);
        }

        result.items = items;
        Ok(result)
    }

    /// Retrieve the parameters for the specified request, merging in the specified variables to scenario (if specified)
    pub fn retrieve_parameters(
        &self,
        request: &WorkbookRequestEntry,
        variables: &HashMap<String, Value>,
    ) -> RequestParameters {
        let mut done = false;

        let mut current = request;

        let mut scenario: Option<&WorkbookScenario> = None;
        let mut authorization: Option<&WorkbookAuthorization> = None;
        let mut certificate: Option<&WorkbookCertificate> = None;
        let mut proxy: Option<&WorkbookProxy> = None;

        let mut auth_certificate: Option<&WorkbookCertificate> = None;
        let mut auth_proxy: Option<&WorkbookProxy> = None;

        let mut allow_scenario = true;
        let mut allow_authorization = true;
        let mut allow_certificate = true;
        let mut allow_proxy = true;

        let mut encountered_ids = HashSet::<String>::new();

        while !done {
            // Set the credential values at the current request value
            if allow_scenario && scenario.is_none() {
                match Self::find_matching_selection(
                    current.get_selected_scenario(),
                    &self.scenarios,
                ) {
                    WorkbookSelectedOption::UseDefault => {}
                    WorkbookSelectedOption::Off => {
                        scenario = None;
                        allow_scenario = false;
                    }
                    WorkbookSelectedOption::Some(s) => {
                        scenario = Some(s);
                    }
                }
            }
            if allow_authorization && authorization.is_none() {
                match Self::find_matching_selection(
                    current.get_selected_authorization(),
                    &self.authorizations,
                ) {
                    WorkbookSelectedOption::UseDefault => {}
                    WorkbookSelectedOption::Off => {
                        authorization = None;
                        allow_authorization = false;
                    }
                    WorkbookSelectedOption::Some(s) => {
                        authorization = Some(s);
                    }
                }
            }
            if allow_certificate && certificate.is_none() {
                match Self::find_matching_selection(
                    current.get_selected_certificate(),
                    &self.certificates,
                ) {
                    WorkbookSelectedOption::UseDefault => {}
                    WorkbookSelectedOption::Off => {
                        certificate = None;
                        allow_certificate = false;
                    }
                    WorkbookSelectedOption::Some(s) => {
                        certificate = Some(s);
                    }
                }
            }
            if allow_proxy && proxy.is_none() {
                match Self::find_matching_selection(current.get_selected_proxy(), &self.proxies) {
                    WorkbookSelectedOption::UseDefault => {}
                    WorkbookSelectedOption::Off => {
                        proxy = None;
                        allow_proxy = false;
                    }
                    WorkbookSelectedOption::Some(s) => {
                        proxy = Some(s);
                    }
                }
            }

            done = ((scenario.is_some() || !allow_scenario)
                && (authorization.is_some() || !allow_authorization)
                && (certificate.is_some())
                || !allow_certificate)
                && (proxy.is_some() || !allow_proxy);

            if !done {
                // Get the parent
                let id = current.get_id();
                encountered_ids.insert(id.clone());

                let mut parent: Option<&WorkbookRequestEntry> = None;
                if let Some(child_ids) = &self.requests.child_ids {
                    for (parent_id, children) in child_ids.iter() {
                        if children.contains(id) {
                            parent = self.requests.entities.get(&parent_id.clone());
                            break;
                        }
                    }
                }

                if let Some(found_parent) = parent {
                    let parent_id = found_parent.get_id();
                    if encountered_ids.contains(parent_id) {
                        println!(
                            "Recursive parent found at {}, cancelling traversal",
                            parent_id
                        );
                        done = true
                    } else {
                        current = found_parent;
                    }
                } else {
                    done = true;
                }
            }
        }

        // Load from workbook defaults if required
        if let Some(defaults) = &self.defaults {
            if scenario.is_none() && allow_scenario {
                if let WorkbookSelectedOption::Some(v) =
                    Self::find_matching_selection(&defaults.selected_scenario, &self.scenarios)
                {
                    scenario = Some(v);
                }
            }
            if authorization.is_none() && allow_authorization {
                if let WorkbookSelectedOption::Some(v) = Self::find_matching_selection(
                    &defaults.selected_authorization,
                    &self.authorizations,
                ) {
                    authorization = Some(v);
                }
            }
            if certificate.is_none() && allow_certificate {
                if let WorkbookSelectedOption::Some(v) = Self::find_matching_selection(
                    &defaults.selected_certificate,
                    &self.certificates,
                ) {
                    certificate = Some(v);
                }
            }
            if proxy.is_none() && allow_proxy {
                if let WorkbookSelectedOption::Some(v) =
                    Self::find_matching_selection(&defaults.selected_proxy, &self.proxies)
                {
                    proxy = Some(v);
                }
            }
        }

        // Set up OAuth2 cert/proxy if specified
        if let Some(WorkbookAuthorization::OAuth2Client {
            selected_certificate,
            selected_proxy,
            ..
        }) = authorization
        {
            if let Some(cert) = selected_certificate {
                auth_certificate = self.certificates.entities.get(&cert.id);
            }
            if let Some(proxy) = selected_proxy {
                auth_proxy = self.proxies.entities.get(&proxy.id);
            }
        }

        let mut result_variables = variables.clone();
        if let Some(active_scenario) = scenario {
            if let Some(variables) = &active_scenario.variables {
                for pair in variables {
                    result_variables.insert(pair.name.clone(), Value::from(pair.value.clone()));
                }
            }
        };

        RequestParameters {
            variables: result_variables,
            authorization,
            certificate,
            proxy,
            auth_certificate,
            auth_proxy,
        }
    }
}

impl SelectableOptions for Workspace {
    fn get_selected_scenario(&self) -> &Option<Selection> {
        if let Some(defaults) = &self.defaults {
            &defaults.selected_scenario
        } else {
            &None
        }
    }

    fn get_selected_authorization(&self) -> &Option<Selection> {
        if let Some(defaults) = &self.defaults {
            &defaults.selected_authorization
        } else {
            &None
        }
    }

    fn get_selected_certificate(&self) -> &Option<Selection> {
        if let Some(defaults) = &self.defaults {
            &defaults.selected_certificate
        } else {
            &None
        }
    }

    fn get_selected_proxy(&self) -> &Option<Selection> {
        if let Some(defaults) = &self.defaults {
            &defaults.selected_proxy
        } else {
            &None
        }
    }

    fn set_selected_scenario(&mut self, value: Option<Selection>) {
        if let Some(defaults) = self.defaults.as_mut() {
            defaults.selected_scenario = value;
        } else {
            self.defaults = Some(WorkbookDefaults {
                selected_scenario: value,
                selected_authorization: None,
                selected_certificate: None,
                selected_proxy: None,
            });
        }
    }

    fn set_selected_authorization(&mut self, value: Option<Selection>) {
        if let Some(defaults) = self.defaults.as_mut() {
            defaults.selected_authorization = value;
        } else {
            self.defaults = Some(WorkbookDefaults {
                selected_scenario: None,
                selected_authorization: value,
                selected_certificate: None,
                selected_proxy: None,
            });
        }
    }

    fn set_selected_certificate(&mut self, value: Option<Selection>) {
        if let Some(defaults) = self.defaults.as_mut() {
            defaults.selected_certificate = value;
        } else {
            self.defaults = Some(WorkbookDefaults {
                selected_scenario: None,
                selected_authorization: None,
                selected_certificate: value,
                selected_proxy: None,
            });
        }
    }

    fn set_selected_proxy(&mut self, value: Option<Selection>) {
        if let Some(defaults) = self.defaults.as_mut() {
            defaults.selected_proxy = value;
        } else {
            self.defaults = Some(WorkbookDefaults {
                selected_scenario: None,
                selected_authorization: None,
                selected_certificate: None,
                selected_proxy: value,
            });
        }
    }
}

impl Warnings for Workspace {
    fn get_warnings(&self) -> &Option<Vec<String>> {
        &self.warnings
    }

    fn add_warning(&mut self, warning: String) {
        match &mut self.warnings {
            Some(warnings) => warnings.push(warning),
            None => self.warnings = Some(vec![warning]),
        }
    }
}

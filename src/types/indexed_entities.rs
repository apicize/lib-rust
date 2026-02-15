use std::collections::HashMap;

use crate::{
    ApicizeError, PERSIST_PRIVATE, PERSIST_VAULT, PERSIST_WORKBOOK, PersistedIndex, RequestEntry,
};
use serde::{Deserialize, Serialize};

use super::{
    Authorization, Certificate, DataSet, Identifiable, Proxy, Scenario, Selection,
    workspace::SelectedOption,
};

pub const NO_SELECTION_ID: &str = "\tNONE\t";

/// Generic for indexed, ordered entities, optionally with children
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedEntities<T> {
    /// Top level entity IDs
    pub top_level_ids: Vec<String>,

    /// Map of parent to child entity IDs
    pub child_ids: HashMap<String, Vec<String>>,

    /// Entities indexed by ID
    pub entities: HashMap<String, T>,
}

impl<T: Identifiable + Clone> Default for IndexedEntities<T> {
    fn default() -> Self {
        Self {
            top_level_ids: Default::default(),
            child_ids: Default::default(),
            entities: Default::default(),
        }
    }
}

impl<T: Identifiable + Clone> IndexedEntities<T> {
    /// Find a match based upon ID or name
    pub fn is_valid(&self, selection: &Selection) -> bool {
        selection.id == NO_SELECTION_ID
            || self.entities.contains_key(&selection.id)
            || self
                .entities
                .values()
                .any(|e| e.get_name().to_lowercase() == selection.name.to_lowercase())
    }

    /// Return entry matched by ID
    pub fn get(&self, id: &str) -> Option<&T> {
        if id == NO_SELECTION_ID {
            None
        } else {
            self.entities.get(id)
        }
    }

    /// Return entry matched by ID as mutable
    pub fn get_mut(&mut self, id: &str) -> Option<&mut T> {
        if id == NO_SELECTION_ID {
            None
        } else {
            self.entities.get_mut(id)
        }
    }

    /// Return entry matched by optional ID
    pub fn get_optional(&self, id: &Option<String>) -> Option<&T> {
        match id {
            Some(id_to_find) => {
                if id_to_find == NO_SELECTION_ID {
                    None
                } else {
                    self.entities.get(id_to_find)
                }
            }
            None => None,
        }
    }

    /// Return entry ID matched by ID
    pub fn find_by_id_or_name(
        &self,
        id_or_name: &Option<String>,
    ) -> Result<Option<String>, ApicizeError> {
        match id_or_name {
            Some(id_to_find) => {
                if id_to_find == NO_SELECTION_ID {
                    Ok(None)
                } else if let Some(found) = self.entities.get(id_to_find) {
                    Ok(Some(found.get_id().to_string()))
                } else if let Some(found_by_name) =
                    self.entities.values().find(|e| e.get_name() == id_to_find)
                {
                    Ok(Some(found_by_name.get_id().to_string()))
                } else {
                    Err(ApicizeError::Error {
                        description: format!("Invalid ID {}", &id_to_find),
                    })
                }
            }
            None => Ok(None),
        }
    }

    /// Find entity (Scenario, Authorization, etc.)
    pub fn find<'a>(&'a self, selection: &Option<Selection>) -> SelectedOption<&'a T> {
        if let Some(s) = selection {
            if s.id == NO_SELECTION_ID {
                return SelectedOption::Off;
            }

            // First, look for matches based upon ID
            if let Some(found_by_id) = self.entities.get(&s.id) {
                return SelectedOption::Some(found_by_id);
            }

            // Otherwise, look for name matches
            if let Some(found_by_name) = self.entities.values().find(|v| {
                let name = v.get_name();
                name.eq_ignore_ascii_case(&s.name)
            }) {
                return SelectedOption::Some(found_by_name);
            }
        }
        SelectedOption::UseDefault
    }
}

impl IndexedEntities<RequestEntry> {
    /// Build IndexRequests from a list of nested Workbook requests
    pub fn new(entities: &[RequestEntry]) -> IndexedEntities<RequestEntry> {
        let mut results = IndexedEntities::<RequestEntry> {
            top_level_ids: Vec::new(),
            child_ids: HashMap::new(),
            entities: HashMap::new(),
        };
        Self::populate_requests(entities, &mut results, None);
        results
    }

    /// Populate the workspace request list
    fn populate_requests(
        entities: &[RequestEntry],
        indexed_requests: &mut IndexedEntities<RequestEntry>,
        parent_id: Option<&str>,
    ) {
        for e in entities.iter() {
            let id = e.get_id();
            match parent_id {
                Some(pid) => {
                    indexed_requests
                        .child_ids
                        .entry(pid.to_string())
                        .or_default()
                        .push(id.to_string());
                }
                None => {
                    indexed_requests.top_level_ids.push(id.to_string());
                }
            }

            match e {
                RequestEntry::Request(info) => {
                    indexed_requests
                        .entities
                        .insert(info.id.clone(), RequestEntry::Request(info.to_owned()));
                }
                RequestEntry::Group(group) => {
                    let mut owned_group = group.to_owned();
                    owned_group.children = None;
                    indexed_requests
                        .entities
                        .insert(group.id.clone(), RequestEntry::Group(owned_group));

                    if let Some(children) = group.children.as_ref() {
                        Self::populate_requests(children, indexed_requests, Some(&group.id));
                    }
                }
            };
        }
    }

    /// Build list of Workbook request entries from indexed requests
    pub fn to_entities(&self) -> Vec<RequestEntry> {
        self.get_workbook(&self.top_level_ids)
    }

    /// Recursively add requests to the list to save
    fn get_workbook(&self, ids: &[String]) -> Vec<RequestEntry> {
        let mut results: Vec<RequestEntry> = vec![];
        ids.iter().for_each(|id| {
            if let Some(entry) = self.entities.get(id) {
                match entry {
                    RequestEntry::Request(info) => {
                        results.push(RequestEntry::Request(info.clone()));
                    }
                    RequestEntry::Group(group) => {
                        let mut group_to_add = group.clone();
                        group_to_add.children = None;
                        if let Some(child_ids) = self.child_ids.get(id) {
                            let children = self.get_workbook(child_ids);
                            if !children.is_empty() {
                                group_to_add.children = Some(children);
                            }
                        }
                        results.push(RequestEntry::Group(group_to_add));
                    }
                }
            }
        });
        results
    }
}

impl IndexedEntities<DataSet> {
    /// Build IndexRequests from a list of Workbook data sets
    pub fn new(entities: Option<Vec<DataSet>>) -> IndexedEntities<DataSet> {
        match entities {
            Some(entities) => {
                let top_level_ids = entities.iter().map(|e| e.id.clone()).collect::<Vec<String>>();
                let entities = entities.into_iter().map(|e| (e.id.clone(), e)).collect::<HashMap<String, DataSet>>();
                IndexedEntities::<DataSet> {
                    top_level_ids,
                    child_ids: HashMap::new(),
                    entities,
                }
            },
            None => IndexedEntities::default(),
        }
    }
}

/// Convert indexed parameters to a persistable list
fn to_persisted_list<T: Clone>(index: &IndexedEntities<T>, persistence: &str) -> Option<Vec<T>> {
    let mut result = Vec::<T>::new();
    if let Some(ids) = index.child_ids.get(persistence) {
        for id in ids {
            if let Some(entity) = index.entities.get(id) {
                result.push(entity.clone());
            }
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Generate indexed entries for parameters stored in workbook, private and/or vault files,
/// note that we do not set top-level IDs, because we are categorizing into public, private
/// and vault/globals
fn from_persisted_lists<T: Identifiable + Clone>(
    workbook: Option<Vec<T>>,
    private: Option<Vec<T>>,
    vault: Option<Vec<T>>,
) -> IndexedEntities<T> {
    
    let map_ids = |list: &Option<Vec<T>>| -> Vec<String> {
        match list {
            Some(entries) => entries.iter().map(|e| e.get_id().to_string()).collect(),
            None => Vec::default()
        }
    };
    
    let workbook_ids = map_ids(&workbook);
    let private_ids = map_ids(&private);
    let vault_ids = map_ids(&vault);

    let mut entities: HashMap<String, T> = match workbook {
        Some(entries) => entries
            .into_iter()
            .map(|e| (e.get_id().to_string(), e))
            .collect::<HashMap<String, T>>(),
        None => HashMap::new(),
    };
    if let Some(entries) = private {
        entities.extend(
            entries
                .into_iter()
                .filter(|e| !entities.contains_key(e.get_id()))
                .map(|e| (e.get_id().to_string(), e))
                .collect::<HashMap<String, T>>(),
        );
    };
    if let Some(entries) = vault {
        entities.extend(
            entries
                .into_iter()
                .filter(|e| !entities.contains_key(e.get_id()))
                .map(|e| (e.get_id().to_string(), e))
                .collect::<HashMap<String, T>>(),
        );
    };

    IndexedEntities::<T> {
        top_level_ids: vec![],
        child_ids: HashMap::from([
            (
                PERSIST_WORKBOOK.to_string(),
                workbook_ids,
            ),
            (
                PERSIST_PRIVATE.to_string(),
                private_ids,
            ),
            (
                PERSIST_VAULT.to_string(),
                vault_ids,
            ),
        ]),
        entities,
    }
}

impl PersistedIndex<Scenario> for IndexedEntities<Scenario> {
    fn get_workbook(&self) -> Option<Vec<Scenario>> {
        to_persisted_list(self, PERSIST_WORKBOOK)
    }

    fn get_private(&self) -> Option<Vec<Scenario>> {
        to_persisted_list(self, PERSIST_PRIVATE)
    }

    fn get_vault(&self) -> Option<Vec<Scenario>> {
        to_persisted_list(self, PERSIST_VAULT)
    }

    fn new(
        workbook: Option<Vec<Scenario>>,
        private: Option<Vec<Scenario>>,
        vault: Option<Vec<Scenario>>,
    ) -> IndexedEntities<Scenario> {
        from_persisted_lists(workbook, private, vault)
    }
}

impl PersistedIndex<Authorization> for IndexedEntities<Authorization> {
    fn get_workbook(&self) -> Option<Vec<Authorization>> {
        to_persisted_list(self, PERSIST_WORKBOOK)
    }

    fn get_private(&self) -> Option<Vec<Authorization>> {
        to_persisted_list(self, PERSIST_PRIVATE)
    }

    fn get_vault(&self) -> Option<Vec<Authorization>> {
        to_persisted_list(self, PERSIST_VAULT)
    }

    fn new(
        workbook: Option<Vec<Authorization>>,
        private: Option<Vec<Authorization>>,
        vault: Option<Vec<Authorization>>,
    ) -> IndexedEntities<Authorization> {
        from_persisted_lists(workbook, private, vault)
    }
}

impl PersistedIndex<Certificate> for IndexedEntities<Certificate> {
    fn get_workbook(&self) -> Option<Vec<Certificate>> {
        to_persisted_list(self, PERSIST_WORKBOOK)
    }

    fn get_private(&self) -> Option<Vec<Certificate>> {
        to_persisted_list(self, PERSIST_PRIVATE)
    }

    fn get_vault(&self) -> Option<Vec<Certificate>> {
        to_persisted_list(self, PERSIST_VAULT)
    }

    fn new(
        workbook: Option<Vec<Certificate>>,
        private: Option<Vec<Certificate>>,
        vault: Option<Vec<Certificate>>,
    ) -> IndexedEntities<Certificate> {
        from_persisted_lists(workbook, private, vault)
    }
}

impl PersistedIndex<Proxy> for IndexedEntities<Proxy> {
    fn get_workbook(&self) -> Option<Vec<Proxy>> {
        to_persisted_list(self, PERSIST_WORKBOOK)
    }

    fn get_private(&self) -> Option<Vec<Proxy>> {
        to_persisted_list(self, PERSIST_PRIVATE)
    }

    fn get_vault(&self) -> Option<Vec<Proxy>> {
        to_persisted_list(self, PERSIST_VAULT)
    }

    fn new(
        workbook: Option<Vec<Proxy>>,
        private: Option<Vec<Proxy>>,
        vault: Option<Vec<Proxy>>,
    ) -> IndexedEntities<Proxy> {
        from_persisted_lists(workbook, private, vault)
    }
}


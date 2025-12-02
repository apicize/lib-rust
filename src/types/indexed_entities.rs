use std::collections::HashMap;

use crate::{
    ApicizeError, PERSIST_PRIVATE, PERSIST_VAULT, PERSIST_WORKBOOK, PersistedIndex, RequestEntry,
};
use serde::{Deserialize, Serialize};

use super::{
    Authorization, Certificate, ExternalData, Identifiable, Proxy, Scenario, Selection,
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
    /// Build IndexRequests from a list of nexted Workbook requests
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
        parent_id: Option<String>,
    ) {
        let active_parent_id = parent_id.unwrap_or(String::from(""));
        for e in entities.iter() {
            match e {
                RequestEntry::Request(info) => {
                    if active_parent_id.is_empty() {
                        indexed_requests.top_level_ids.push(info.id.clone());
                    } else {
                        let updated_child_ids =
                            match indexed_requests.child_ids.get(&active_parent_id) {
                                Some(matching_group) => {
                                    let mut updated = matching_group.to_vec();
                                    updated.push(info.id.clone());
                                    updated
                                }
                                None => Vec::from([info.id.clone()]),
                            };
                        indexed_requests
                            .child_ids
                            .insert(active_parent_id.clone(), updated_child_ids);
                    }
                    indexed_requests
                        .entities
                        .insert(info.id.clone(), RequestEntry::Request(info.to_owned()));
                }
                RequestEntry::Group(group) => {
                    if active_parent_id.is_empty() {
                        indexed_requests.top_level_ids.push(group.id.clone());
                    } else {
                        let updated_child_ids =
                            match indexed_requests.child_ids.get(&active_parent_id) {
                                Some(matching_group) => {
                                    let mut updated = matching_group.to_vec();
                                    updated.push(group.id.clone());
                                    updated
                                }
                                None => Vec::from([group.id.clone()]),
                            };
                        indexed_requests
                            .child_ids
                            .insert(active_parent_id.clone(), updated_child_ids);
                    }

                    let mut owned_group = group.to_owned();
                    owned_group.children = None;
                    indexed_requests
                        .entities
                        .insert(group.id.clone(), RequestEntry::Group(owned_group));

                    if let Some(children) = group.children.as_ref() {
                        Self::populate_requests(children, indexed_requests, Some(group.id.clone()));
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
/// and vault/globalls
fn from_persisted_lists<T: Identifiable + Clone>(
    workbook: Option<&[T]>,
    private: Option<&[T]>,
    vault: Option<&[T]>,
) -> IndexedEntities<T> {
    let mut entities: HashMap<String, T> = match workbook {
        Some(entries) => entries
            .iter()
            .map(|e| (e.get_id().to_string(), e.clone()))
            .collect::<HashMap<String, T>>(),
        None => HashMap::new(),
    };
    if let Some(entries) = private {
        entities.extend(
            entries
                .iter()
                .filter(|e| !entities.contains_key(e.get_id()))
                .map(|e| (e.get_id().to_string(), e.clone()))
                .collect::<HashMap<String, T>>(),
        );
    };
    if let Some(entries) = vault {
        entities.extend(
            entries
                .iter()
                .filter(|e| !entities.contains_key(e.get_id()))
                .map(|e| (e.get_id().to_string(), e.clone()))
                .collect::<HashMap<String, T>>(),
        );
    };

    IndexedEntities::<T> {
        top_level_ids: vec![],
        child_ids: HashMap::from([
            (
                PERSIST_WORKBOOK.to_string(),
                match workbook {
                    Some(entries) => entries.iter().map(|e| e.get_id().to_string()).collect(),
                    None => vec![],
                },
            ),
            (
                PERSIST_PRIVATE.to_string(),
                match private {
                    Some(entries) => entries.iter().map(|e| e.get_id().to_string()).collect(),
                    None => vec![],
                },
            ),
            (
                PERSIST_VAULT.to_string(),
                match vault {
                    Some(entries) => entries.iter().map(|e| e.get_id().to_string()).collect(),
                    None => vec![],
                },
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
        workbook: Option<&[Scenario]>,
        private: Option<&[Scenario]>,
        vault: Option<&[Scenario]>,
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
        workbook: Option<&[Authorization]>,
        private: Option<&[Authorization]>,
        vault: Option<&[Authorization]>,
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
        workbook: Option<&[Certificate]>,
        private: Option<&[Certificate]>,
        vault: Option<&[Certificate]>,
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
        workbook: Option<&[Proxy]>,
        private: Option<&[Proxy]>,
        vault: Option<&[Proxy]>,
    ) -> IndexedEntities<Proxy> {
        from_persisted_lists(workbook, private, vault)
    }
}

impl PersistedIndex<ExternalData> for IndexedEntities<ExternalData> {
    fn get_workbook(&self) -> Option<Vec<ExternalData>> {
        to_persisted_list(self, PERSIST_WORKBOOK)
    }

    fn get_private(&self) -> Option<Vec<ExternalData>> {
        to_persisted_list(self, PERSIST_PRIVATE)
    }

    fn get_vault(&self) -> Option<Vec<ExternalData>> {
        to_persisted_list(self, PERSIST_VAULT)
    }

    fn new(
        workbook: Option<&[ExternalData]>,
        private: Option<&[ExternalData]>,
        vault: Option<&[ExternalData]>,
    ) -> IndexedEntities<ExternalData> {
        from_persisted_lists(workbook, private, vault)
    }
}

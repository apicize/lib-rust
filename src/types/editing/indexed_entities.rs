use std::vec;

use serde::{Deserialize, Serialize};

use crate::{ApicizeError, Identifiable, IndexedEntities};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexedEntityPosition {
    Before,
    After,
    Under,
}

impl<T: Identifiable> IndexedEntities<T> {
    /// Add an indexec entity relative to a specified ID
    pub fn add_entity(
        &mut self,
        entity: T,
        relative_to_id: Option<&str>,
        relative_position: Option<IndexedEntityPosition>,
    ) -> Result<String, ApicizeError> {
        let entity_id = entity.get_id().to_string();
        self.set_position(&entity_id, relative_to_id, relative_position)?;
        self.entities.insert(entity_id.to_string(), entity);
        Ok(entity_id)
    }

    /// Remove an indexed entity and its children completely from the list
    pub fn remove_entity(&mut self, entity_id: &str) -> Result<T, ApicizeError> {
        // Remove from the entity and top level list
        match self.entities.remove(entity_id) {
            Some(entity) => {
                self.clear_position(entity_id, true)?;
                Ok(entity)
            }
            None => Err(ApicizeError::InvalidId {
                description: format!("Invalid entity ID {}", entity_id),
            }),
        }
    }

    /// Move an entity to a different position in the list
    pub fn move_entity(
        &mut self,
        entity_id: &str,
        relative_to_id: &str,
        relative_position: IndexedEntityPosition,
    ) -> Result<bool, ApicizeError> {
        if entity_id == relative_to_id {
            Ok(false)
        } else {
            self.clear_position(entity_id, false)?;
            self.set_position(entity_id, Some(relative_to_id), Some(relative_position))?;
            Ok(true)
        }
    }

    /// Position entity in the index as specified by relative_to
    fn set_position(
        &mut self,
        entity_id: &str,
        relative_to_id: Option<&str>,
        relative_position: Option<IndexedEntityPosition>,
    ) -> Result<(), ApicizeError> {
        let mut is_inserted = false;
        if let Some(relative_to_id) = relative_to_id {
            let insert_in_list = |list: &mut Vec<String>| {
                if let Some(pos) = list.iter().position(|id| id == relative_to_id) {
                    list.insert(
                        if relative_position == Some(IndexedEntityPosition::Before) {
                            pos
                        } else {
                            pos + 1
                        },
                        entity_id.to_string(),
                    );
                    true
                } else {
                    false
                }
            };

            if relative_position == Some(IndexedEntityPosition::Under) {
                if let Some(children) = self.child_ids.get_mut(relative_to_id) {
                    children.push(entity_id.to_string());
                } else {
                    self.child_ids
                        .insert(relative_to_id.to_string(), vec![entity_id.to_string()]);
                }
                self.parent_ids.insert(entity_id.to_string(), relative_to_id.to_string());
                is_inserted = true;
            } else {
                // Try inserting at top level
                is_inserted = insert_in_list(&mut self.top_level_ids);
                if !is_inserted {
                    // Use the parent reverse index to find which parent owns relative_to_id
                    if let Some(parent_id) = self.parent_ids.get(relative_to_id).cloned() {
                        if let Some(child_ids) = self.child_ids.get_mut(&parent_id) {
                            is_inserted = insert_in_list(child_ids);
                            if is_inserted {
                                self.parent_ids.insert(entity_id.to_string(), parent_id);
                            }
                        }
                    }
                    if !is_inserted {
                        // Fallback: search all child lists (handles deserialized data without parent_ids)
                        let mut found_parent: Option<String> = None;
                        for (parent_key, child_ids) in self.child_ids.iter_mut() {
                            if insert_in_list(child_ids) {
                                found_parent = Some(parent_key.clone());
                                is_inserted = true;
                                break;
                            }
                        }
                        if let Some(parent_key) = found_parent {
                            self.parent_ids.insert(entity_id.to_string(), parent_key);
                        }
                    }
                } else {
                    // Inserted at top level — remove any stale parent mapping
                    self.parent_ids.remove(entity_id);
                }
            }
        }

        if !is_inserted {
            // If not relative to anything, just append to the top level
            self.top_level_ids.push(entity_id.to_string());
            self.parent_ids.remove(entity_id);
        }

        Ok(())
    }

    /// Clear indexed position information for the specified entity ID
    fn clear_position(&mut self, entity_id: &str, remove_children: bool) -> Result<(), ApicizeError> {
        // Use parent reverse index for O(1) lookup when available
        if let Some(parent_id) = self.parent_ids.remove(entity_id) {
            if let Some(children) = self.child_ids.get_mut(&parent_id) {
                if let Some(idx) = children.iter().position(|c| c == entity_id) {
                    children.remove(idx);
                }
            }
        } else {
            // No parent mapping — check top-level IDs
            if let Some(idx) = self.top_level_ids.iter().position(|c| c == entity_id) {
                self.top_level_ids.remove(idx);
            }
        }

        // Remove child entries if requested
        if remove_children {
            self.child_ids.remove_entry(entity_id);
        }

        Ok(())
    }
}

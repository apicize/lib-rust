use serde::{Deserialize, Serialize};

use crate::Identifiable;

use super::identifiable::CloneIdentifiable;

/// Information about a selected entity, include both ID and name
/// to give the maximum chance of finding a match
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    /// ID of selected entity
    pub id: String,
    /// Name of selected entity
    #[serde(skip_serializing_if = "String::is_empty", default = "String::default")]
    pub name: String,
}

impl Identifiable for Selection {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
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

impl CloneIdentifiable for Selection {
    fn clone_as_new(&self, _: String) -> Self {
        self.clone()
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            id: Selection::DEFAULT_SELECTION_ID.to_string(),
            name: Default::default(),
        }
    }
}

impl Selection {
    pub const DEFAULT_SELECTION_ID: &str = "\tDEFAULT\t";
    pub const NO_SELECTION_ID: &str = "\tNONE\t";

    pub fn is_default_or_none(&self) -> bool {
        self.id == Selection::DEFAULT_SELECTION_ID || self.id == Selection::NO_SELECTION_ID
    }

    pub fn is_default(&self) -> bool {
        self.id == Selection::DEFAULT_SELECTION_ID
    }

    pub fn is_none(&self) -> bool {
        self.id == Selection::NO_SELECTION_ID
    }

    pub fn new_default() -> Selection {
        Selection {
            id: Selection::DEFAULT_SELECTION_ID.to_string(),
            name: "(Default)".to_string(),
        }
    }

    pub fn new_none() -> Selection {
        Selection {
            id: Selection::NO_SELECTION_ID.to_string(),
            name: "None (Off)".to_string(),
        }
    }
}

/// Type of seletion to fall back to if the specified selection entry is invalid
pub enum SelectionIfInvalid {
    Default,
    None,
}

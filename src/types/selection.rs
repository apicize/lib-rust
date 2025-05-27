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

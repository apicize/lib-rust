use serde::{Deserialize, Serialize};

use crate::Identifable;

/// Information about a selected entity, include both ID and name
/// to give the maximum chance of finding a match
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    /// ID of selected entity
    pub id: String,
    /// Name of selected entity
    pub name: String,
}

impl Identifable for Selection {
    fn get_id(&self) -> &String {
        &self.id
    }
    
    fn get_name(&self) -> &String {
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



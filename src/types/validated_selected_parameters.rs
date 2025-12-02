use std::collections::HashMap;

use crate::{
    Identifiable, Selection, indexed_entities::NO_SELECTION_ID
};

pub trait ValidatedSelectedParameters {
    /// Validate selected scenario
    fn validate_scenario(&mut self, valid_values: &HashMap<String, String>);

    /// Validate selected authorization
    fn validate_authorization(&mut self, valid_values: &HashMap<String, String>);

    /// Validate selected certificate
    fn validate_certificate(&mut self, valid_values: &HashMap<String, String>);

    /// Validate selected proxy
    fn validate_proxy(&mut self, valid_values: &HashMap<String, String>);

    /// Validate selected data
    fn validate_data(&mut self, valid_values: &HashMap<String, String>);
}

/// Check to see if the selection is in the list of valid options. If not,
/// return a value to add
pub fn validate_selection(
    entity_label: &str,
    value: &mut Option<Selection>,
    value_label: &str,
    valid_options: &HashMap<String, String>,
) -> Option<String> {
    let mut is_invalid = false;
    let warning = match value {
        Some(v) => {
            let id = v.id.clone();
            if id == NO_SELECTION_ID || valid_options.contains_key(&id) {
                None
            } else {
                let name = v.get_name().to_string();
                match valid_options.iter().find_map(|(id, selection_name)| {
                    if selection_name == &name {
                        Some(id.clone())
                    } else {
                        None
                    }
                }) {
                    Some(use_id) => {
                        v.id = use_id;
                        Some(format!(
                            "{} selected {} \"{}\" was found with a different ID, using entry with ID {}",
                            entity_label, value_label, name, &v.id
                        ))

                    }
                    None => {
                        is_invalid = true;
                        Some(format!(
                            "{} selected {} \"{}\" (ID {}) not found, defaulting to Off",
                            entity_label, value_label, name, &id,
                        ))
                    }
                }
            }
        }
        None => None,
    };

    if is_invalid {
        *value = None;
    }

    warning
}


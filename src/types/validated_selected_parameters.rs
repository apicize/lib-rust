use std::collections::HashMap;

use crate::{
    indexed_entities::NO_SELECTION_ID, EditableWarnings, Identifiable, RequestEntry, Selection,
    WorkbookDefaultParameters,
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
fn validate_selection(
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

impl ValidatedSelectedParameters for RequestEntry {
    fn validate_scenario(&mut self, valid_values: &HashMap<String, String>) {
        match self {
            RequestEntry::Request(request) => {
                if let Some(warning) = validate_selection(
                    &request.get_title(),
                    &mut request.selected_scenario,
                    "scenario",
                    valid_values,
                ) {
                    request.set_warnings(Some(vec![warning]));
                }
            }
            RequestEntry::Group(group) => {
                if let Some(warning) = validate_selection(
                    &group.get_title(),
                    &mut group.selected_scenario,
                    "scenario",
                    valid_values,
                ) {
                    group.set_warnings(Some(vec![warning]));
                }
            }
        }
    }

    fn validate_authorization(&mut self, valid_values: &HashMap<String, String>) {
        match self {
            RequestEntry::Request(request) => {
                if let Some(warning) = validate_selection(
                    &request.get_title(),
                    &mut request.selected_authorization,
                    "authorization",
                    valid_values,
                ) {
                    request.set_warnings(Some(vec![warning]));
                }
            }
            RequestEntry::Group(group) => {
                if let Some(warning) = validate_selection(
                    &group.get_title(),
                    &mut group.selected_authorization,
                    "authorization",
                    valid_values,
                ) {
                    group.set_warnings(Some(vec![warning]));
                }
            }
        }
    }

    fn validate_certificate(&mut self, valid_values: &HashMap<String, String>) {
        match self {
            RequestEntry::Request(request) => {
                if let Some(warning) = validate_selection(
                    &request.get_title(),
                    &mut request.selected_certificate,
                    "certificate",
                    valid_values,
                ) {
                    request.set_warnings(Some(vec![warning]));
                }
            }
            RequestEntry::Group(group) => {
                if let Some(warning) = validate_selection(
                    &group.get_title(),
                    &mut group.selected_certificate,
                    "certificate",
                    valid_values,
                ) {
                    group.set_warnings(Some(vec![warning]));
                }
            }
        }
    }

    fn validate_proxy(&mut self, valid_values: &HashMap<String, String>) {
        match self {
            RequestEntry::Request(request) => {
                if let Some(warning) = validate_selection(
                    &request.get_title(),
                    &mut request.selected_proxy,
                    "proxy",
                    valid_values,
                ) {
                    request.set_warnings(Some(vec![warning]));
                }
            }
            RequestEntry::Group(group) => {
                if let Some(warning) = validate_selection(
                    &group.get_title(),
                    &mut group.selected_proxy,
                    "proxy",
                    valid_values,
                ) {
                    group.set_warnings(Some(vec![warning]));
                }
            }
        }
    }

    fn validate_data(&mut self, valid_values: &HashMap<String, String>) {
        match self {
            RequestEntry::Request(request) => {
                if let Some(warning) = validate_selection(
                    &request.get_title(),
                    &mut request.selected_data,
                    "data",
                    valid_values,
                ) {
                    request.set_warnings(Some(vec![warning]));
                }
            }
            RequestEntry::Group(group) => {
                if let Some(warning) = validate_selection(
                    &group.get_title(),
                    &mut group.selected_data,
                    "data",
                    valid_values,
                ) {
                    group.set_warnings(Some(vec![warning]));
                }
            }
        }
    }
}

impl ValidatedSelectedParameters for WorkbookDefaultParameters {
    fn validate_scenario(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_scenario,
            "scenario",
            valid_values,
        ) {
            self.set_warnings(Some(vec![warning]));
        }
    }

    fn validate_authorization(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_authorization,
            "authorization",
            valid_values,
        ) {
            self.set_warnings(Some(vec![warning]));
        }
    }

    fn validate_certificate(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_certificate,
            "certificate",
            valid_values,
        ) {
            self.set_warnings(Some(vec![warning]));
        }
    }

    fn validate_proxy(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_proxy,
            "proxy",
            valid_values,
        ) {
            self.set_warnings(Some(vec![warning]));
        }
    }

    fn validate_data(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_data,
            "data",
            valid_values,
        ) {
            self.set_warnings(Some(vec![warning]));
        }
    }
}

use std::collections::HashMap;

use crate::{Authorization, Identifiable, Validated, indexed_entities::NO_SELECTION_ID};

use super::Selection;

/// Trait indicating scenarios, authorizations, etc. can be
pub trait SelectedParameters {
    /// Get selected scenario, if any
    fn selected_scenario(&self) -> &Option<Selection>;

    /// Get selected authorization, if any
    fn selected_authorization(&self) -> &Option<Selection>;

    /// Get selected certificate, if any
    fn selected_certificate(&self) -> &Option<Selection>;

    /// Get selected proxy, if any
    fn selected_proxy(&self) -> &Option<Selection>;

    /// Get selected data, if any
    fn selected_data(&self) -> &Option<Selection>;

    /// Get selected scenario, if any
    fn selected_scenario_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected authorization, if any
    fn selected_authorization_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected certificate, if any
    fn selected_certificate_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected proxy, if any
    fn selected_proxy_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected data, if any
    fn selected_data_as_mut(&mut self) -> &mut Option<Selection>;
}

/// Name/value pairs for the domain of parameters selectable within a workbook/workspace
pub struct SelectableParameters {
    pub scenarios: HashMap<String, String>,
    pub authorizations: HashMap<String, String>,
    pub certificates: HashMap<String, String>,
    pub proxies: HashMap<String, String>,
    pub data: HashMap<String, String>,
}

pub trait IdentityWithSelectedParameters: SelectedParameters + Validated + Identifiable {}

impl<T: ?Sized> IdentityWithSelectedParameters for T where
    T: SelectedParameters + Validated + Identifiable
{
}

impl SelectableParameters {
    pub fn validate_request_or_group(
        &self,
        entity: &mut dyn IdentityWithSelectedParameters,
    ) -> bool {
        let mut warnings = Vec::<String>::new();
        let entity_label = entity.get_title();

        if let Some(warning) = validate_selection(
            &entity_label,
            entity.selected_scenario_as_mut(),
            "scenario",
            &self.scenarios,
        ) {
            warnings.push(warning);
        }

        if let Some(warning) = validate_selection(
            &entity_label,
            entity.selected_authorization_as_mut(),
            "authorization",
            &self.authorizations,
        ) {
            warnings.push(warning);
        }

        if let Some(warning) = validate_selection(
            &entity_label,
            entity.selected_certificate_as_mut(),
            "certificate",
            &self.certificates,
        ) {
            warnings.push(warning);
        }

        if let Some(warning) = validate_selection(
            &entity_label,
            entity.selected_proxy_as_mut(),
            "proxy",
            &self.proxies,
        ) {
            warnings.push(warning);
        }

        if let Some(warning) = validate_selection(
            &entity_label,
            entity.selected_data_as_mut(),
            "data",
            &self.data,
        ) {
            warnings.push(warning);
        }

        let no_warnings = warnings.is_empty();

        entity.set_validation_warnings(if no_warnings { None } else { Some(warnings) });

        no_warnings
    }

    pub fn validate_authorization(&self, entity: &mut Authorization) -> bool {
        let mut warnings = Vec::<String>::new();
        let entity_label = entity.get_title();

        if let Authorization::OAuth2Client {
            selected_certificate,
            selected_proxy,
            ..
        } = entity
        {
            if let Some(warning) = validate_selection(
                &entity_label,
                selected_certificate,
                "certificate",
                &self.certificates,
            ) {
                warnings.push(warning);
            }

            if let Some(warning) =
                validate_selection(&entity_label, selected_proxy, "proxy", &self.proxies)
            {
                warnings.push(warning);
            }

            let no_warnings = warnings.is_empty();
            entity.set_validation_warnings(if no_warnings { None } else { Some(warnings) });
            no_warnings
        } else {
            true
        }
    }
}

/// Check to see if the selection is in the list of valid options. If not,
/// return a value to add to warnings
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
                let selected_name = v.name.as_str();
                match valid_options.iter().find_map(|(id, name)| {
                    if selected_name == name {
                        Some(id.clone())
                    } else {
                        None
                    }
                }) {
                    Some(use_id) => {
                        v.id = use_id;
                        Some(format!(
                            "{} selected {} \"{}\" was found with a different ID, using entry with ID {}",
                            entity_label, value_label, selected_name, &v.id
                        ))
                    }
                    None => {
                        is_invalid = true;
                        Some(format!(
                            "{} selected {} \"{}\" (ID {}) not found, switching to default",
                            entity_label, value_label, selected_name, &id,
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

use std::collections::HashMap;

use super::{SelectedParameters, Selection, indexed_entities::NO_SELECTION_ID};
use crate::{Validated, ValidatedSelectedParameters, ValidationState, validate_selection};
use serde::{Deserialize, Serialize};

/// Default parameters for the workbook
#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookDefaultParameters {
    /// Selected scenario, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_scenario: Option<Selection>,
    /// Selected authorization, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_authorization: Option<Selection>,
    /// Selected certificate, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_certificate: Option<Selection>,
    /// Selected proxy, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_proxy: Option<Selection>,
    /// Selected external data, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_data: Option<Selection>,
    /// Validation state
    #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
    pub validation_state: ValidationState,
    /// Warnings for invalid values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_warnings: Option<Vec<String>>,
    /// Validation errors
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<HashMap<String, String>>,
}

impl WorkbookDefaultParameters {
    pub fn any_values_set(&self) -> bool {
        !(self
            .selected_scenario
            .as_ref()
            .is_none_or(|s| s.id == NO_SELECTION_ID)
            && self
                .selected_authorization
                .as_ref()
                .is_none_or(|s| s.id == NO_SELECTION_ID)
            && self
                .selected_certificate
                .as_ref()
                .is_none_or(|s| s.id == NO_SELECTION_ID)
            && self
                .selected_proxy
                .as_ref()
                .is_none_or(|s| s.id == NO_SELECTION_ID)
            && self
                .selected_data
                .as_ref()
                .is_none_or(|s| s.id == NO_SELECTION_ID))
    }
}

impl Validated for WorkbookDefaultParameters {
    fn get_validation_state(&self) -> &super::ValidationState {
        &self.validation_state
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        &self.validation_warnings
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        self.validation_warnings = warnings;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        &self.validation_errors
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        self.validation_errors = errors;
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }
}

impl SelectedParameters for WorkbookDefaultParameters {
    fn selected_scenario(&self) -> &Option<Selection> {
        &self.selected_scenario
    }

    fn selected_authorization(&self) -> &Option<Selection> {
        &self.selected_authorization
    }

    fn selected_certificate(&self) -> &Option<Selection> {
        &self.selected_certificate
    }

    fn selected_proxy(&self) -> &Option<Selection> {
        &self.selected_proxy
    }

    fn selected_data(&self) -> &Option<Selection> {
        &self.selected_data
    }

    fn selected_scenario_as_mut(&mut self) -> &mut Option<Selection> {
        &mut self.selected_scenario
    }

    fn selected_authorization_as_mut(&mut self) -> &mut Option<Selection> {
        &mut self.selected_authorization
    }

    fn selected_certificate_as_mut(&mut self) -> &mut Option<Selection> {
        &mut self.selected_certificate
    }

    fn selected_proxy_as_mut(&mut self) -> &mut Option<Selection> {
        &mut self.selected_proxy
    }

    fn selected_data_as_mut(&mut self) -> &mut Option<Selection> {
        &mut self.selected_data
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
            self.set_validation_warnings(Some(vec![warning]));
        }
    }

    fn validate_authorization(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_authorization,
            "authorization",
            valid_values,
        ) {
            self.set_validation_warnings(Some(vec![warning]));
        }
    }

    fn validate_certificate(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) = validate_selection(
            "Default",
            &mut self.selected_certificate,
            "certificate",
            valid_values,
        ) {
            self.set_validation_warnings(Some(vec![warning]));
        }
    }

    fn validate_proxy(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) =
            validate_selection("Default", &mut self.selected_proxy, "proxy", valid_values)
        {
            self.set_validation_warnings(Some(vec![warning]));
        }
    }

    fn validate_data(&mut self, valid_values: &HashMap<String, String>) {
        if let Some(warning) =
            validate_selection("Default", &mut self.selected_data, "data", valid_values)
        {
            self.set_validation_warnings(Some(vec![warning]));
        }
    }
}

use std::collections::HashMap;

use super::{SelectedParameters, Selection};
use crate::{Identifiable, Validated, ValidationState};
use serde::{Deserialize, Serialize};

/// Default parameters for the workbook
#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookDefaultParameters {
    /// Selected scenario, if applicable
    #[serde(skip_serializing_if = "Selection::is_none", default = "Selection::new_none")]
    pub selected_scenario: Selection,
    /// Selected authorization, if applicable
    #[serde(skip_serializing_if = "Selection::is_none", default = "Selection::new_none")]
    pub selected_authorization: Selection,
    /// Selected certificate, if applicable
    #[serde(skip_serializing_if = "Selection::is_none", default = "Selection::new_none")]
    pub selected_certificate: Selection,
    /// Selected proxy, if applicable
    #[serde(skip_serializing_if = "Selection::is_none", default = "Selection::new_none")]
    pub selected_proxy: Selection,
    /// Selected external data, if applicable
    #[serde(skip_serializing_if = "Selection::is_none", default = "Selection::new_none")]
    pub selected_data: Selection,
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

impl Identifiable for WorkbookDefaultParameters {
    fn get_id(&self) -> &str {
        "defaults"
    }

    fn get_name(&self) -> &str {
        "Defaults"
    }

    fn get_title(&self) -> String {
        "Defaults".to_string()
    }
}

impl WorkbookDefaultParameters {
    pub fn any_values_set(&self) -> bool {
        !(self
            .selected_scenario
            .is_default_or_none()
            && self
                .selected_authorization
                .is_default_or_none()
            && self
                .selected_certificate
                .is_default_or_none()
            && self
                .selected_proxy
                .is_default_or_none()
            && self
                .selected_data
                .is_default_or_none())
    }
}

impl Validated for WorkbookDefaultParameters {
    fn get_validation_state(&self) -> super::ValidationState {
        self.validation_state
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
    fn selected_scenario(&self) -> &Selection {
        &self.selected_scenario
    }

    fn selected_authorization(&self) -> &Selection {
        &self.selected_authorization
    }

    fn selected_certificate(&self) -> &Selection {
        &self.selected_certificate
    }

    fn selected_proxy(&self) -> &Selection {
        &self.selected_proxy
    }

    fn selected_data(&self) -> &Selection {
        &self.selected_data
    }

    fn selected_scenario_as_mut(&mut self) -> &mut Selection {
        &mut self.selected_scenario
    }

    fn selected_authorization_as_mut(&mut self) -> &mut Selection {
        &mut self.selected_authorization
    }

    fn selected_certificate_as_mut(&mut self) -> &mut Selection {
        &mut self.selected_certificate
    }

    fn selected_proxy_as_mut(&mut self) -> &mut Selection {
        &mut self.selected_proxy
    }

    fn selected_data_as_mut(&mut self) -> &mut Selection {
        &mut self.selected_data
    }
}

impl WorkbookDefaultParameters {
    pub fn perform_validation(&mut self) {
        self.set_validation_errors(None);
        self.validation_state =
            ValidationState::from(&self.validation_warnings, &self.validation_errors);
    }
}

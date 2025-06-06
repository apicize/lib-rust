use serde::{Deserialize, Serialize};

use super::{indexed_entities::NO_SELECTION_ID, EditableWarnings, SelectedParameters, Selection, Warnings};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

impl WorkbookDefaultParameters {
    pub fn any_values_set(&self) -> bool {
        ! (self.selected_scenario.as_ref().is_none_or(|s| s.id == NO_SELECTION_ID)
            && self.selected_authorization.as_ref().is_none_or(|s| s.id == NO_SELECTION_ID)
            && self.selected_certificate.as_ref().is_none_or(|s| s.id == NO_SELECTION_ID)
            && self.selected_proxy.as_ref().is_none_or(|s| s.id == NO_SELECTION_ID)
            && self.selected_data.as_ref().is_none_or(|s| s.id == NO_SELECTION_ID))
    }
}

impl Warnings for WorkbookDefaultParameters {
    fn get_warnings(&self) -> &Option<Vec<String>> {
        &self.warnings
    }
}

impl EditableWarnings for WorkbookDefaultParameters {
    fn set_warnings(&mut self, warnings: Option<Vec<String>>) {
        self.warnings = warnings;
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


use serde::{Deserialize, Serialize};

use super::{SelectedParameters, Selection};

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
}
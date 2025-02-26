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
    
    /// Get selected scenario, if any
    fn selected_scenario_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected authorization, if any
    fn selected_authorization_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected certificate, if any
    fn selected_certificate_as_mut(&mut self) -> &mut Option<Selection>;

    /// Get selected proxy, if any
    fn selected_proxy_as_mut(&mut self) -> &mut Option<Selection>;

}
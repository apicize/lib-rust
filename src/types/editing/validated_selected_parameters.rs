use std::collections::HashSet;

use crate::{indexed_entities::NO_SELECTION_ID, RequestEntry, WorkbookDefaultParameters};

pub trait ValidatedSelectedParameters {
    /// Validate selected scenario
    fn validate_scenario(&mut self, valid_ids: &HashSet<String>) -> ();

    /// Validate selected authorization
    fn validate_authorization(&mut self, valid_ids: &HashSet<String>) -> ();

    /// Validate selected certificate
    fn validate_certificate(&mut self, valid_ids: &HashSet<String>) -> ();

    /// Validate selected proxy
    fn validate_proxy(&mut self, valid_ids: &HashSet<String>) -> ();

    /// Validate selected data
    fn validate_data(&mut self, valid_ids: &HashSet<String>) -> ();
}

impl ValidatedSelectedParameters for RequestEntry {
    fn validate_scenario(&mut self, valid_ids: &HashSet<String>) -> () {
        match self {
            RequestEntry::Request(request) => {
                if let Some(s) = &request.selected_scenario {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        request.selected_scenario = None;
                    }
                }
            }
            RequestEntry::Group(group) => {
                if let Some(s) = &group.selected_scenario {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        group.selected_scenario = None;
                    }
                }
            }
        }
    }

    fn validate_authorization(&mut self, valid_ids: &HashSet<String>) -> () {
        match self {
            RequestEntry::Request(request) => {
                if let Some(s) = &request.selected_authorization {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        request.selected_authorization = None;
                    }
                }
            }
            RequestEntry::Group(group) => {
                if let Some(s) = &group.selected_authorization {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        group.selected_authorization = None;
                    }
                }
            }
        }
    }

    fn validate_certificate(&mut self, valid_ids: &HashSet<String>) -> () {
        match self {
            RequestEntry::Request(request) => {
                if let Some(s) = &request.selected_certificate {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        request.selected_certificate = None;
                    }
                }
            }
            RequestEntry::Group(group) => {
                if let Some(s) = &group.selected_certificate {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        group.selected_certificate = None;
                    }
                }
            }
        }
    }

    fn validate_proxy(&mut self, valid_ids: &HashSet<String>) -> () {
        match self {
            RequestEntry::Request(request) => {
                if let Some(s) = &request.selected_proxy {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        request.selected_proxy = None;
                    }
                }
            }
            RequestEntry::Group(group) => {
                if let Some(s) = &group.selected_proxy {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        group.selected_proxy = None;
                    }
                }
            }
        }
    }

    fn validate_data(&mut self, valid_ids: &HashSet<String>) -> () {
        match self {
            RequestEntry::Request(request) => {
                if let Some(s) = &request.selected_data {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        request.selected_data = None;
                    }
                }
            }
            RequestEntry::Group(group) => {
                if let Some(s) = &group.selected_data {
                    if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                        group.selected_data = None;
                    }
                }
            }
        }
    }
}

impl ValidatedSelectedParameters for WorkbookDefaultParameters {
    fn validate_scenario(&mut self, valid_ids: &HashSet<String>) -> () {
        if let Some(s) = &mut self.selected_scenario {
            if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                self.selected_scenario = None;
            }
        }
    }

    fn validate_authorization(&mut self, valid_ids: &HashSet<String>) -> () {
        if let Some(s) = &mut self.selected_authorization {
            if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                self.selected_authorization = None;
            }
        }
    }

    fn validate_certificate(&mut self, valid_ids: &HashSet<String>) -> () {
        if let Some(s) = &mut self.selected_certificate {
            if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                self.selected_certificate = None;
            }
        }
    }

    fn validate_proxy(&mut self, valid_ids: &HashSet<String>) -> () {
        if let Some(s) = &mut self.selected_proxy {
            if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                self.selected_proxy = None;
            }
        }
    }

    fn validate_data(&mut self, valid_ids: &HashSet<String>) -> () {
        if let Some(s) = &mut self.selected_data {
            if s.id != NO_SELECTION_ID && !valid_ids.contains(&s.id) {
                self.selected_data = None;
            }
        }
    }
}

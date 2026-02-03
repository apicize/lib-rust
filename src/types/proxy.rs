use std::collections::HashMap;

use crate::{
    Identifiable, Validated, ValidationState, add_validation_error, identifiable::CloneIdentifiable, remove_validation_error, utility::*
};
use regex::Regex;
use reqwest::{ClientBuilder, Error};
use serde::{Deserialize, Serialize};

/// An HTTP or SOCKS5 proxy that can be used to tunnel requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Proxy {
    /// Uniquely identify proxy
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Name of proxy
    pub name: String,
    /// Location of proxy (URL for HTTP proxy, IP for SOCKS)
    pub url: String,
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

impl Proxy {
    /// Append proxy to builder
    pub fn append_to_builder(&self, builder: ClientBuilder) -> Result<ClientBuilder, Error> {
        match reqwest::Proxy::all(&self.url) {
            Ok(proxy) => Ok(builder.proxy(proxy)),
            Err(err) => Err(err),
        }
    }
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: Default::default(),
            url: Default::default(),
            validation_state: Default::default(),
            validation_warnings: None,
            validation_errors: None,
        }
    }
}

impl Identifiable for Proxy {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            "(Unnamed)".to_string()
        } else {
            self.name.to_string()
        }
    }
}
impl CloneIdentifiable for Proxy {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

impl Validated for Proxy {
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

impl Proxy {
    pub fn perform_validation(&mut self) {
        self.validate_name();
        self.validate_url();
    }

    pub fn validate_name(&mut self) {
        let name_ok = ! self.name.trim().is_empty();
        if name_ok {
            remove_validation_error(&mut self.validation_errors, "name");
        } else {
            add_validation_error(&mut self.validation_errors, "name", "Name is required");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }
    
    pub fn validate_url(&mut self) {
        let regex_url = Regex::new(r"^(\{\{.+\}\}|https?:\/\/|socks5:\/\/)(\w+:?\w*)?(\S+)(:\d+)?(\/|\/([\w#!:.?+=&%!\-\/]))?$").unwrap();
        if regex_url.is_match(&self.url) {
            remove_validation_error(&mut self.validation_errors, "url");
        } else {
            add_validation_error(&mut self.validation_errors, "url", "URL must include http/https/socks5 protocol prefix and address");
        }
        self.validation_state.set(ValidationState::ERROR, self.validation_errors.is_some());
    }
}
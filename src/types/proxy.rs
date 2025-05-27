use std::collections::HashMap;

use crate::utility::*;
use crate::Identifiable;
use reqwest::{ClientBuilder, Error};
use serde::{Deserialize, Serialize};

use super::identifiable::CloneIdentifiable;
use super::ValidationErrors;
use super::Warnings;

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
    /// Validation errors
    validation_errors: Option<HashMap<String, String>>,
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
            validation_errors: Default::default(),
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

impl Warnings for Proxy {
    fn get_warnings(&self) -> &Option<Vec<String>> {
        &None
    }
}

impl ValidationErrors for Proxy {
    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        &self.validation_errors
    }
}

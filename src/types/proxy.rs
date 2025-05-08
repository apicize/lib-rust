use crate::utility::*;
use crate::Identifiable;
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
    /// Warning if invalid
    pub warning: Option<String>,
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
            warning: Default::default() 
        }
    }
}

impl Identifiable for Proxy {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            "(Unnamed)".to_string()
        } else {
            self.name.to_string()
        }
    }

    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        cloned.id = generate_uuid();
        cloned.name = new_name;
        cloned
    }
}

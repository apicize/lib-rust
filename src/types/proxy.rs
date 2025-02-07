use reqwest::{ClientBuilder, Error};
use serde::{Deserialize, Serialize};
use crate::Identifable;
use crate::utility::*;

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

impl Identifable for Proxy {
    fn get_id(&self) -> &String {
        &self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> String {
        if self.name.is_empty() {
            format!("{} (Unnamed)", self.id)
        } else {
            self.name.to_string()
        }
    }
}

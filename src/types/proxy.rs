use std::collections::HashMap;

use crate::{
    ApicizeError, Identifiable, Validated, ValidationState, add_validation_error, decrypt, encrypt,
    identifiable::CloneIdentifiable,
    parameters::{EncryptableParameter, ParameterCipher, ParameterEncryption},
    remove_validation_error,
    utility::*,
};
use regex::Regex;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static PROXY_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\{\{.+\}\}|https?:\/\/|socks5:\/\/)(\w+:?\w*)?(\S+)(:\d+)?(\/|\/([\w#!:.?+=&%!\-\/]))?$").unwrap()
});

/// An HTTP or SOCKS5 proxy that can be used to tunnel requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Proxy {
    Cipher(ParameterCipher),
    Plain(Box<ProxyPlain>),
}

/// An HTTP or SOCKS5 proxy that can be used to tunnel requests
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ProxyPlain {
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

/// Values that are stored as when encrypting a proxy
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct ProxyEncryptedData {
    /// Location of proxy (URL for HTTP proxy, IP for SOCKS)
    pub url: String,
}

impl Proxy {
    /// Append proxy to builder
    pub fn append_to_builder(&self, builder: ClientBuilder) -> Result<ClientBuilder, ApicizeError> {
        match self {
            Proxy::Cipher(_proxy) => Err(ApicizeError::Encryption {
                description: "Encyrpted proxies cannot be added to requests".to_string(),
            }),
            Proxy::Plain(proxy) => match reqwest::Proxy::all(&proxy.url) {
                Ok(proxy) => Ok(builder.proxy(proxy)),
                Err(err) => Err(ApicizeError::from_reqwest(err, None)),
            },
        }
    }
}

impl Default for ProxyPlain {
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

impl Default for Proxy {
    fn default() -> Self {
        Proxy::Plain(Box::default())
    }
}

impl Identifiable for ProxyPlain {
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

impl Identifiable for Proxy {
    fn get_id(&self) -> &str {
        match self {
            Proxy::Plain(plain) => plain.get_id(),
            Proxy::Cipher(cipher) => cipher.get_id(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Proxy::Plain(plain) => plain.get_name(),
            Proxy::Cipher(cipher) => cipher.get_name(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            Proxy::Plain(plain) => plain.get_title(),
            Proxy::Cipher(cipher) => cipher.get_title(),
        }
    }
}

impl CloneIdentifiable for Proxy {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        match &mut cloned {
            Proxy::Cipher(cipher) => {
                cipher.id = generate_uuid();
                cipher.name = new_name;
            }
            Proxy::Plain(plain) => {
                plain.id = generate_uuid();
                plain.name = new_name;
            }
        }
        cloned
    }
}

impl EncryptableParameter for Proxy {
    fn is_encrypted(&self) -> bool {
        matches!(self, Proxy::Cipher(_))
    }

    fn encrypt(&self, password: &str, method: ParameterEncryption) -> Result<Proxy, ApicizeError> {
        let Proxy::Plain(proxy) = self else {
            return Err(ApicizeError::Encryption {
                description: "Encrypted proxys cannot be re-encrypted".to_string(),
            });
        };

        let data = serde_json::to_string(&ProxyEncryptedData {
            url: proxy.url.clone(),
        })
        .map_err(|err| ApicizeError::Encryption {
            description: format!("Unable to serialize proxy - {}", err),
        })?;
        Ok(Proxy::Cipher(ParameterCipher {
            id: proxy.id.to_string(),
            name: proxy.name.to_string(),
            data: encrypt(&data, password, method)?,
        }))
    }

    fn decrypt(&self, password: &str, method: ParameterEncryption) -> Result<Proxy, ApicizeError> {
        let Proxy::Cipher(proxy) = self else {
            return Err(ApicizeError::Encryption {
                description: "Proxy is already decrypted".to_string(),
            });
        };

        let data =
            serde_json::from_str::<ProxyEncryptedData>(&decrypt(&proxy.data, password, method)?)
                .map_err(|err| ApicizeError::Encryption {
                    description: format!("Unable to deserialize proxy - {}", err),
                })?;

        Ok(Proxy::Plain(Box::new(ProxyPlain {
            id: proxy.id.to_string(),
            name: proxy.name.to_string(),
            url: data.url,
            validation_state: ValidationState::empty(),
            validation_warnings: None,
            validation_errors: None,
        })))
    }
}

impl ProxyPlain {
    pub fn perform_validation(&mut self) {
        self.validate_name();
        self.validate_url();
    }

    pub fn validate_name(&mut self) {
        let name_ok = !self.name.trim().is_empty();
        if name_ok {
            remove_validation_error(&mut self.validation_errors, "name");
        } else {
            add_validation_error(&mut self.validation_errors, "name", "Name is required");
        }
        self.validation_state
            .set(ValidationState::ERROR, self.validation_errors.is_some());
    }

    pub fn validate_url(&mut self) {
        if PROXY_URL_REGEX.is_match(&self.url) {
            remove_validation_error(&mut self.validation_errors, "url");
        } else {
            add_validation_error(
                &mut self.validation_errors,
                "url",
                "URL must include http/https/socks5 protocol prefix and address",
            );
        }
        self.validation_state
            .set(ValidationState::ERROR, self.validation_errors.is_some());
    }
}

impl Validated for ProxyPlain {
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

impl Validated for Proxy {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            Proxy::Cipher(_cipher) => ValidationState::empty(),
            Proxy::Plain(plain) => plain.get_validation_state(),
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            Proxy::Cipher(_cipher) => &None,
            Proxy::Plain(plain) => plain.get_validation_warnings(),
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        if let Proxy::Plain(proxy) = self {
            proxy.set_validation_warnings(warnings)
        }
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            Proxy::Cipher(_cipher) => &None,
            Proxy::Plain(plain) => plain.get_validation_errors(),
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        if let Proxy::Plain(proxy) = self {
            proxy.set_validation_errors(errors)
        }
    }
}

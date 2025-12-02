use std::collections::HashMap;

use crate::{ApicizeError, Identifiable, Validated, ValidationState, utility::*};
use reqwest::{ClientBuilder, Identity};
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

use super::identifiable::CloneIdentifiable;

/// Client certificate used to identify caller
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum Certificate {
    /// PKCS 12 certificate and and password (.p12 or .pfx)
    #[serde(rename = "PKCS12")]
    PKCS12 {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Certificate
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pfx: Vec<u8>,
        /// Password
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        /// Validation state
        #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
        validation_state: ValidationState,
        /// Warnings for invalid values
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validation_warnings: Option<Vec<String>>,
        /// Validation errors
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validation_errors: Option<HashMap<String, String>>,
    },
    /// PEM-encoded certificate and PKCS8 encoded private key files
    #[serde(rename = "PKCS8_PEM")]
    PKCS8PEM {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
        /// Optional key file, if not combining in PKCS8 format
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        key: Vec<u8>,
        /// Validation state
        #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
        validation_state: ValidationState,
        /// Warnings for invalid values
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validation_warnings: Option<Vec<String>>,
        /// Validation errors
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validation_errors: Option<HashMap<String, String>>,
    },
    /// PEM encoded certificate and key file
    #[serde(rename = "PEM")]
    PEM {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
        /// Validation state
        #[serde(default, skip_serializing_if = "ValidationState::is_empty")]
        validation_state: ValidationState,
        /// Warnings for invalid values
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validation_warnings: Option<Vec<String>>,
        /// Validation errors
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validation_errors: Option<HashMap<String, String>>,
    },
}

impl Certificate {
    /// Append certificate to builder
    pub fn append_to_builder(&self, builder: ClientBuilder) -> Result<ClientBuilder, ApicizeError> {
        let identity = match self {
            Certificate::PKCS12 { pfx, password, .. } => Identity::from_pkcs12_der(
                pfx,
                password.clone().unwrap_or(String::from("")).as_str(),
            ),
            Certificate::PKCS8PEM { pem, key, .. } => Identity::from_pkcs8_pem(pem, key),
            Certificate::PEM { pem, .. } => Identity::from_pem(pem),
        }.map_err(|err| ApicizeError::from_reqwest(err, None))?;

        Ok(builder.identity(identity).use_native_tls())
    }
}

impl Default for Certificate {
    fn default() -> Self {
        Certificate::PEM {
            id: generate_uuid(),
            name: String::default(),
            pem: Vec::default(),
            validation_state: Default::default(),
            validation_warnings: None,
            validation_errors: None,
        }
    }
}

impl Identifiable for Certificate {
    fn get_id(&self) -> &str {
        match self {
            Certificate::PEM { id, .. } => id,
            Certificate::PKCS8PEM { id, .. } => id,
            Certificate::PKCS12 { id, .. } => id,
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Certificate::PEM { name, .. } => name,
            Certificate::PKCS8PEM { name, .. } => name,
            Certificate::PKCS12 { name, .. } => name,
        }
    }

    fn get_title(&self) -> String {
        let name = self.get_name();
        if name.is_empty() {
            "(Unamed)".to_string()
        } else {
            name.to_string()
        }
    }
}

impl CloneIdentifiable for Certificate {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        let new_id = generate_uuid();

        match cloned {
            Certificate::PEM {
                ref mut id,
                ref mut name,
                ..
            } => {
                *id = new_id;
                *name = new_name;
            }
            Certificate::PKCS8PEM {
                ref mut id,
                ref mut name,
                ..
            } => {
                *id = new_id;
                *name = new_name;
            }
            Certificate::PKCS12 {
                ref mut id,
                ref mut name,
                ..
            } => {
                *id = new_id;
                *name = new_name;
            }
        }

        cloned
    }
}

impl Validated for Certificate {
    fn get_validation_state(&self) -> &ValidationState {
        match self {
            Certificate::PKCS12 {
                validation_state, ..
            } => validation_state,
            Certificate::PKCS8PEM {
                validation_state, ..
            } => validation_state,
            Certificate::PEM {
                validation_state, ..
            } => validation_state,
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            Certificate::PKCS12 {
                validation_warnings, ..
            } => validation_warnings,
            Certificate::PKCS8PEM {
                validation_warnings, ..
            } => validation_warnings,
            Certificate::PEM {
                validation_warnings, ..
            } => validation_warnings,
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        match self {
            Certificate::PKCS12 {
                validation_warnings, validation_errors, validation_state, ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            },
            Certificate::PKCS8PEM {
                validation_warnings, validation_errors, validation_state, ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            },
            Certificate::PEM {
                validation_warnings, validation_errors, validation_state, ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            },
        }
    }
    
    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            Certificate::PKCS12 {
                validation_errors, ..
            } => validation_errors,
            Certificate::PKCS8PEM {
                validation_errors, ..
            } => validation_errors,
            Certificate::PEM {
                validation_errors, ..
            } => validation_errors,
        }
    }
    
    
    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        match self {
            Certificate::PKCS12 {
                validation_warnings, validation_errors, validation_state, ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            },
            Certificate::PKCS8PEM {
                validation_warnings,validation_errors, validation_state, ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            },
            Certificate::PEM {
                validation_warnings,validation_errors, validation_state, ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            },
        }
    }
}

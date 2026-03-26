use std::collections::HashMap;

use crate::parameters::{EncryptableParameter, ParameterCipher, ParameterEncryption};
use crate::{
    ApicizeError, Identifiable, Validated, ValidationState, add_validation_error, decrypt, encrypt, remove_validation_error, utility::*
};
use reqwest::{ClientBuilder, Identity};
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

use super::identifiable::CloneIdentifiable;

/// Client certificate used to identify caller

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Certificate {
    Cipher(ParameterCipher),
    Plain(Box<CertificatePlain>),
}

/// Client certificate used to identify caller
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum CertificatePlain {
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

/// Client certificate used to identify caller
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum CertificateEncryptedData {
    /// PKCS 12 certificate and and password (.p12 or .pfx)
    #[serde(rename = "PKCS12")]
    PKCS12 {
        /// Certificate
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pfx: Vec<u8>,
        /// Password
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
    /// PEM-encoded certificate and PKCS8 encoded private key files
    #[serde(rename = "PKCS8_PEM")]
    PKCS8PEM {
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
        /// Optional key file, if not combining in PKCS8 format
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        key: Vec<u8>,
    },
    /// PEM encoded certificate and key file
    #[serde(rename = "PEM")]
    PEM {
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
    },
}

impl Default for CertificatePlain {
    fn default() -> Self {
        CertificatePlain::PEM {
            id: generate_uuid(),
            name: String::default(),
            pem: Vec::default(),
            validation_state: Default::default(),
            validation_warnings: None,
            validation_errors: None,
        }
    }
}

impl Default for Certificate {
    fn default() -> Self {
        Certificate::Plain(Box::default())
    }
}

impl Identifiable for CertificatePlain {
    fn get_id(&self) -> &str {
        match self {
            CertificatePlain::PEM { id, .. } => id,
            CertificatePlain::PKCS8PEM { id, .. } => id,
            CertificatePlain::PKCS12 { id, .. } => id,
        }
    }

    fn get_name(&self) -> &str {
        match self {
            CertificatePlain::PEM { name, .. } => name,
            CertificatePlain::PKCS8PEM { name, .. } => name,
            CertificatePlain::PKCS12 { name, .. } => name,
        }
    }

    fn get_title(&self) -> String {
        let name = self.get_name();
        if name.is_empty() {
            "(Unnamed)".to_string()
        } else {
            name.to_string()
        }
    }
}

impl Identifiable for Certificate {
    fn get_id(&self) -> &str {
        match self {
            Certificate::Cipher(cipher) => cipher.get_id(),
            Certificate::Plain(plain) => plain.get_id(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Certificate::Cipher(cipher) => cipher.get_name(),
            Certificate::Plain(plain) => plain.get_name(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            Certificate::Cipher(cipher) => cipher.get_title(),
            Certificate::Plain(plain) => plain.get_title(),
        }
    }
}

impl CloneIdentifiable for Certificate {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        let new_id = generate_uuid();

        match &mut cloned {
            Certificate::Cipher(cipher) => {
                cipher.id = new_id;
                cipher.name = new_name;
            }
            Certificate::Plain(plain) => match plain.as_mut() {
                CertificatePlain::PEM { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
                CertificatePlain::PKCS8PEM { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
                CertificatePlain::PKCS12 { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
            },
        }

        cloned
    }
}

impl EncryptableParameter for Certificate {
    fn is_encrypted(&self) -> bool {
        matches!(self, Certificate::Cipher(_))
    }

    fn encrypt(&self, password: &str, method: ParameterEncryption) -> Result<Certificate, ApicizeError> {
        let Certificate::Plain(certificate) = self else {
            return Err(ApicizeError::Encryption {
                description: "Encrypted certificates cannot be re-encrypted".to_string(),
            });
        };
        let certificate = &**certificate;

        let serialize = |val: &_| {
            serde_json::to_string(val).map_err(|err| ApicizeError::Encryption {
                description: format!("Unable to serialize certificate - {}", err),
            })
        };

        let (id, name, data) = match certificate {
            CertificatePlain::PKCS12 {
                id,
                name,
                pfx,
                password,
                ..
            } => (
                id,
                name,
                serialize(&CertificateEncryptedData::PKCS12 {
                    pfx: pfx.clone(),
                    password: password.clone(),
                })?,
            ),
            CertificatePlain::PKCS8PEM {
                id, name, pem, key, ..
            } => (
                id,
                name,
                serialize(&CertificateEncryptedData::PKCS8PEM {
                    pem: pem.clone(),
                    key: key.clone(),
                })?,
            ),
            CertificatePlain::PEM { id, name, pem, .. } => (
                id,
                name,
                serialize(&CertificateEncryptedData::PEM { pem: pem.clone() })?,
            ),
        };

        Ok(Certificate::Cipher(ParameterCipher {
            id: id.to_string(),
            name: name.to_string(),
            data: encrypt(&data, password, method)?,
        }))
    }

    fn decrypt(&self, password: &str, method: ParameterEncryption) -> Result<Certificate, ApicizeError> {
        let Certificate::Cipher(certificate) = self else {
            return Err(ApicizeError::Encryption {
                description: "Certificate is already decrypted".to_string(),
            });
        };

        let data = serde_json::from_str::<CertificateEncryptedData>(&decrypt(
            &certificate.data,
            password,
            method,
        )?)
        .map_err(|err| ApicizeError::Encryption {
            description: format!("Unable to deserialize certificate - {}", err),
        })?;

        Ok(match data {
            CertificateEncryptedData::PKCS12 { pfx, password } => {
                Certificate::Plain(Box::new(CertificatePlain::PKCS12 {
                    id: certificate.id.to_owned(),
                    name: certificate.name.to_owned(),
                    pfx,
                    password,
                    validation_state: ValidationState::empty(),
                    validation_warnings: None,
                    validation_errors: None,
                }))
            }
            CertificateEncryptedData::PKCS8PEM { pem, key } => {
                Certificate::Plain(Box::new(CertificatePlain::PKCS8PEM {
                    id: certificate.id.to_owned(),
                    name: certificate.name.to_owned(),
                    pem,
                    key,
                    validation_state: ValidationState::empty(),
                    validation_warnings: None,
                    validation_errors: None,
                }))
            }
            CertificateEncryptedData::PEM { pem } => {
                Certificate::Plain(Box::new(CertificatePlain::PEM {
                    id: certificate.id.to_owned(),
                    name: certificate.name.to_owned(),
                    pem,
                    validation_state: ValidationState::empty(),
                    validation_warnings: None,
                    validation_errors: None,
                }))
            }
        })
    }
}

impl Certificate {
    /// Append certificate to builder
    pub fn append_to_builder(&self, builder: ClientBuilder) -> Result<ClientBuilder, ApicizeError> {
        let identity = match self {
            Certificate::Cipher(_cipher) => {
                return Err(ApicizeError::Encryption {
                    description: "Encyrpted certificates cannot be added to requests".to_string(),
                });
            }
            Certificate::Plain(plain) => match plain.as_ref() {
                CertificatePlain::PKCS12 { pfx, password, .. } => Identity::from_pkcs12_der(
                    pfx,
                    password.clone().unwrap_or(String::from("")).as_str(),
                ),
                CertificatePlain::PKCS8PEM { pem, key, .. } => Identity::from_pkcs8_pem(pem, key),
                CertificatePlain::PEM { pem, .. } => Identity::from_pem(pem),
            },
        }
        .map_err(|err| ApicizeError::from_reqwest(err, None))?;

        Ok(builder.identity(identity).use_native_tls())
    }
    
}

impl Validated for CertificatePlain {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            CertificatePlain::PKCS12 {
                validation_state, ..
            } => *validation_state,
            CertificatePlain::PKCS8PEM {
                validation_state, ..
            } => *validation_state,
            CertificatePlain::PEM {
                validation_state, ..
            } => *validation_state,
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            CertificatePlain::PKCS12 {
                validation_warnings,
                ..
            } => validation_warnings,
            CertificatePlain::PKCS8PEM {
                validation_warnings,
                ..
            } => validation_warnings,
            CertificatePlain::PEM {
                validation_warnings,
                ..
            } => validation_warnings,
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        match self {
            CertificatePlain::PKCS12 {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            CertificatePlain::PKCS8PEM {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            CertificatePlain::PEM {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
        }
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            CertificatePlain::PKCS12 {
                validation_errors, ..
            } => validation_errors,
            CertificatePlain::PKCS8PEM {
                validation_errors, ..
            } => validation_errors,
            CertificatePlain::PEM {
                validation_errors, ..
            } => validation_errors,
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        match self {
            CertificatePlain::PKCS12 {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            CertificatePlain::PKCS8PEM {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            CertificatePlain::PEM {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
        }
    }
}

impl Validated for Certificate {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            Certificate::Cipher(ParameterCipher { .. }) => ValidationState::empty(),
            Certificate::Plain(plain) => plain.get_validation_state(),
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            Certificate::Cipher(ParameterCipher { .. }) => &None,
            Certificate::Plain(plain) => plain.get_validation_warnings(),
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        if let Certificate::Plain(certificate) = self {
            certificate.set_validation_warnings(warnings);
        }
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            Certificate::Cipher(ParameterCipher { .. }) => &None,
            Certificate::Plain(plain) => plain.get_validation_errors(),
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        if let Certificate::Plain(certificate) = self {
            certificate.set_validation_errors(errors);
        }
    }
}

impl CertificatePlain {
    pub fn perform_validation(&mut self) {
        if self.get_name().is_empty() {
            self.set_validation_errors(Some(HashMap::from([(
                "name".to_string(),
                "Name is required".to_string(),
            )])));
        } else {
            self.set_validation_errors(None);
        }
    }

    pub fn validate_name(&mut self) {
        let perform_validation =
            |name: &str,
             validation_errors: &mut Option<HashMap<String, String>>,
             validation_state: &mut ValidationState| {
                let name_ok = !name.trim().is_empty();
                if name_ok {
                    remove_validation_error(validation_errors, "name");
                } else {
                    add_validation_error(validation_errors, "name", "Name is required");
                }
                validation_state.set(ValidationState::ERROR, validation_errors.is_some());
            };

        match self {
            CertificatePlain::PKCS12 {
                name,
                validation_errors,
                validation_state,
                ..
            } => {
                perform_validation(name, validation_errors, validation_state);
            }
            CertificatePlain::PKCS8PEM {
                name,
                validation_errors,
                validation_state,
                ..
            } => {
                perform_validation(name, validation_errors, validation_state);
            }
            CertificatePlain::PEM {
                name,
                validation_errors,
                validation_state,
                ..
            } => {
                perform_validation(name, validation_errors, validation_state);
            }
        }
    }
}

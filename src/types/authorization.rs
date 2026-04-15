use std::collections::HashMap;

use super::{Selection, ValidationState, identifiable::CloneIdentifiable};
use crate::{
    ApicizeError, Identifiable, Validated, add_validation_error, decrypt, encrypt,
    parameters::{EncryptableParameter, ParameterCipher, ParameterEncryption},
    remove_validation_error,
    utility::*,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Authorization configuration used when dispatching an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Authorization {
    Cipher(ParameterCipher),
    Plain(Box<AuthorizationPlain>),
}

/// Authorization configuration used when dispatching an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum AuthorizationPlain {
    /// Basic authentication (basic authorization header)
    #[serde(rename_all = "camelCase")]
    Basic {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of authorization configuration
        name: String,
        /// User name
        username: String,
        /// Password
        password: String,
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
    /// OAuth2 client flow (bearer authorization header)
    #[serde(rename_all = "camelCase")]
    OAuth2Client {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        /// Client secret (allowed to be blank)
        client_secret: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        /// Audience to add to token
        audience: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: String,
        /// Selected certificate, if applicable
        #[serde(
            skip_serializing_if = "Selection::is_none",
            default = "Selection::new_none"
        )]
        selected_certificate: Selection,
        /// Selected proxy, if applicable
        #[serde(
            skip_serializing_if = "Selection::is_none",
            default = "Selection::new_none"
        )]
        selected_proxy: Selection,
        /// If true, OAuth credentials are sent in body instead of header
        #[serde(skip_serializing_if = "Option::is_none")]
        send_credentials_in_body: Option<bool>,
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
    /// OAuth2 PKCE flow (note, this can only be used interactively)
    #[serde(rename_all = "camelCase")]
    OAuth2Pkce {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// URL for authorization
        authorize_url: String,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// If true, credentials are sent in the body as opposed to authorization header
        send_credentials_in_body: Option<bool>,
        /// Currently active token (needs to be set before usage)
        #[serde(skip_serializing)]
        token: Option<String>,
        /// Currently active refresh token if available (needs to be set before usage)
        #[serde(skip_serializing)]
        refresh_token: Option<String>,
        /// Expiration of currently active token in seconds past Unix epoch (needs to be set before usage)
        #[serde(skip_serializing)]
        expiration: Option<u64>,
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
    /// API key authentication (sent in HTTP header)
    #[serde(rename_all = "camelCase")]
    ApiKey {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// Name of header (ex. "x-api-key")
        header: String,
        /// Value of key to include as header value
        value: String,
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
pub enum AuthorizationEncryptedData {
    /// Basic authentication (basic authorization header)
    #[serde(rename_all = "camelCase")]
    Basic {
        /// User name
        username: String,
        /// Password
        password: String,
    },
    /// OAuth2 client flow (bearer authorization header)
    #[serde(rename_all = "camelCase")]
    OAuth2Client {
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        /// Client secret (allowed to be blank)
        client_secret: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        /// Audience to add to token
        audience: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: String,
        /// Selected certificate, if applicable
        #[serde(
            skip_serializing_if = "Selection::is_none",
            default = "Selection::new_none"
        )]
        selected_certificate: Selection,
        /// Selected proxy, if applicable
        #[serde(
            skip_serializing_if = "Selection::is_none",
            default = "Selection::new_none"
        )]
        selected_proxy: Selection,
        /// If true, OAuth credentials are sent in body instead of header
        #[serde(skip_serializing_if = "Option::is_none")]
        send_credentials_in_body: Option<bool>,
    },
    /// OAuth2 PKCE flow (note, this can only be used interactively)
    #[serde(rename_all = "camelCase")]
    OAuth2Pkce {
        /// Uniquely identifies authorization configuration
        /// URL for authorization
        authorize_url: String,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        send_credentials_in_body: Option<bool>,
    },
    /// API key authentication (sent in HTTP header)
    #[serde(rename_all = "camelCase")]
    ApiKey {
        /// Name of header (ex. "x-api-key")
        header: String,
        /// Value of key to include as header value
        value: String,
    },
}

impl Default for AuthorizationPlain {
    fn default() -> Self {
        AuthorizationPlain::ApiKey {
            id: generate_uuid(),
            name: Default::default(),
            header: Default::default(),
            value: Default::default(),
            validation_state: Default::default(),
            validation_warnings: None,
            validation_errors: None,
        }
    }
}

impl Default for Authorization {
    fn default() -> Self {
        Authorization::Plain(Box::default())
    }
}

impl Identifiable for AuthorizationPlain {
    fn get_id(&self) -> &str {
        match self {
            AuthorizationPlain::Basic { id, .. } => id,
            AuthorizationPlain::OAuth2Client { id, .. } => id,
            AuthorizationPlain::OAuth2Pkce { id, .. } => id,
            AuthorizationPlain::ApiKey { id, .. } => id,
        }
    }

    fn get_name(&self) -> &str {
        match self {
            AuthorizationPlain::Basic { name, .. } => name,
            AuthorizationPlain::OAuth2Client { name, .. } => name,
            AuthorizationPlain::OAuth2Pkce { name, .. } => name,
            AuthorizationPlain::ApiKey { name, .. } => name,
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

impl Identifiable for Authorization {
    fn get_id(&self) -> &str {
        match self {
            Authorization::Cipher(cipher) => cipher.get_id(),
            Authorization::Plain(plain) => plain.get_id(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Authorization::Cipher(cipher) => cipher.get_name(),
            Authorization::Plain(plain) => plain.get_name(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            Authorization::Cipher(cipher) => cipher.get_title(),
            Authorization::Plain(plain) => plain.get_title(),
        }
    }
}

impl CloneIdentifiable for Authorization {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        let new_id = generate_uuid();

        match &mut cloned {
            Authorization::Cipher(cipher) => {
                cipher.id = new_id;
                cipher.name = new_name;
            }
            Authorization::Plain(plain) => match plain.as_mut() {
                AuthorizationPlain::Basic { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
                AuthorizationPlain::OAuth2Client { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
                AuthorizationPlain::OAuth2Pkce { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
                AuthorizationPlain::ApiKey { id, name, .. } => {
                    *id = new_id;
                    *name = new_name;
                }
            },
        }

        cloned
    }
}

impl EncryptableParameter for Authorization {
    fn is_encrypted(&self) -> bool {
        matches!(self, Authorization::Cipher(_))
    }

    fn encrypt(
        &self,
        password: &str,
        method: ParameterEncryption,
    ) -> Result<Authorization, ApicizeError> {
        let Authorization::Plain(authorization) = self else {
            return Err(ApicizeError::Encryption {
                description: "Encrypted authorizations cannot be re-encrypted".to_string(),
            });
        };
        let authorization = &**authorization;

        let serialize = |val: &_| {
            serde_json::to_string(val).map_err(|err| ApicizeError::Encryption {
                description: format!("Unable to serialize authorization - {}", err),
            })
        };

        let (id, name, data) = match authorization {
            AuthorizationPlain::Basic {
                id,
                name,
                username,
                password,
                ..
            } => (
                id,
                name,
                serialize(&AuthorizationEncryptedData::Basic {
                    username: username.to_string(),
                    password: password.to_string(),
                })?,
            ),
            AuthorizationPlain::OAuth2Client {
                id,
                name,
                access_token_url,
                client_id,
                client_secret,
                audience,
                scope,
                selected_certificate,
                selected_proxy,
                send_credentials_in_body,
                ..
            } => (
                id,
                name,
                serialize(&AuthorizationEncryptedData::OAuth2Client {
                    access_token_url: access_token_url.to_string(),
                    client_id: client_id.to_string(),
                    client_secret: client_secret.clone(),
                    audience: audience.clone(),
                    scope: scope.clone(),
                    selected_certificate: selected_certificate.clone(),
                    selected_proxy: selected_proxy.clone(),
                    send_credentials_in_body: *send_credentials_in_body,
                })?,
            ),
            AuthorizationPlain::OAuth2Pkce {
                id,
                name,
                authorize_url,
                access_token_url,
                client_id,
                scope,
                send_credentials_in_body,
                ..
            } => (
                id,
                name,
                serialize(&AuthorizationEncryptedData::OAuth2Pkce {
                    authorize_url: authorize_url.to_string(),
                    access_token_url: access_token_url.to_string(),
                    client_id: client_id.to_string(),
                    scope: scope.to_string(),
                    send_credentials_in_body: *send_credentials_in_body,
                })?,
            ),
            AuthorizationPlain::ApiKey {
                id,
                name,
                header,
                value,
                ..
            } => (
                id,
                name,
                serialize(&AuthorizationEncryptedData::ApiKey {
                    header: header.to_string(),
                    value: value.to_string(),
                })?,
            ),
        };

        Ok(Authorization::Cipher(ParameterCipher {
            id: id.to_string(),
            name: name.to_string(),
            data: encrypt(&data, password, method)?,
        }))
    }

    fn decrypt(
        &self,
        password: &str,
        method: ParameterEncryption,
    ) -> Result<Authorization, ApicizeError> {
        let Authorization::Cipher(authorization) = self else {
            return Err(ApicizeError::Encryption {
                description: "Authorization is already decrypted".to_string(),
            });
        };

        let data = serde_json::from_str::<AuthorizationEncryptedData>(&decrypt(
            &authorization.data,
            password,
            method,
        )?)
        .map_err(|err| ApicizeError::Encryption {
            description: err.to_string(),
        })?;

        Ok(match data {
            AuthorizationEncryptedData::Basic { username, password } => {
                Authorization::Plain(Box::new(AuthorizationPlain::Basic {
                    id: authorization.id.to_string(),
                    name: authorization.name.to_string(),
                    username,
                    password,
                    validation_state: ValidationState::empty(),
                    validation_warnings: None,
                    validation_errors: None,
                }))
            }
            AuthorizationEncryptedData::OAuth2Client {
                access_token_url,
                client_id,
                client_secret,
                audience,
                scope,
                selected_certificate,
                selected_proxy,
                send_credentials_in_body,
            } => Authorization::Plain(Box::new(AuthorizationPlain::OAuth2Client {
                id: authorization.id.to_string(),
                name: authorization.name.to_string(),
                access_token_url,
                client_id,
                client_secret,
                audience,
                scope,
                selected_certificate,
                selected_proxy,
                send_credentials_in_body,
                validation_state: ValidationState::empty(),
                validation_warnings: None,
                validation_errors: None,
            })),
            AuthorizationEncryptedData::OAuth2Pkce {
                authorize_url,
                access_token_url,
                client_id,
                scope,
                send_credentials_in_body,
            } => Authorization::Plain(Box::new(AuthorizationPlain::OAuth2Pkce {
                id: authorization.id.to_string(),
                name: authorization.name.to_string(),
                authorize_url,
                access_token_url,
                client_id,
                scope,
                token: None,
                refresh_token: None,
                expiration: None,
                send_credentials_in_body,
                validation_state: ValidationState::empty(),
                validation_warnings: None,
                validation_errors: None,
            })),
            AuthorizationEncryptedData::ApiKey { header, value } => {
                Authorization::Plain(Box::new(AuthorizationPlain::ApiKey {
                    id: authorization.id.to_string(),
                    name: authorization.name.to_string(),
                    header,
                    value,
                    validation_state: ValidationState::empty(),
                    validation_warnings: None,
                    validation_errors: None,
                }))
            }
        })
    }
}

impl Validated for AuthorizationPlain {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            AuthorizationPlain::Basic {
                validation_state, ..
            } => *validation_state,
            AuthorizationPlain::OAuth2Client {
                validation_state, ..
            } => *validation_state,
            AuthorizationPlain::OAuth2Pkce {
                validation_state, ..
            } => *validation_state,
            AuthorizationPlain::ApiKey {
                validation_state, ..
            } => *validation_state,
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            AuthorizationPlain::OAuth2Client {
                validation_warnings: warnings,
                ..
            } => warnings,
            _ => &None,
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        match self {
            AuthorizationPlain::Basic {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            AuthorizationPlain::OAuth2Client {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            AuthorizationPlain::OAuth2Pkce {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_warnings = warnings;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            AuthorizationPlain::ApiKey {
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
            AuthorizationPlain::Basic {
                validation_errors, ..
            } => validation_errors,
            AuthorizationPlain::OAuth2Client {
                validation_errors, ..
            } => validation_errors,
            AuthorizationPlain::OAuth2Pkce {
                validation_errors, ..
            } => validation_errors,
            AuthorizationPlain::ApiKey {
                validation_errors, ..
            } => validation_errors,
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        match self {
            AuthorizationPlain::Basic {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            AuthorizationPlain::OAuth2Client {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            AuthorizationPlain::OAuth2Pkce {
                validation_warnings,
                validation_errors,
                validation_state,
                ..
            } => {
                *validation_errors = errors;
                *validation_state = ValidationState::from(validation_warnings, validation_errors);
            }
            AuthorizationPlain::ApiKey {
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

impl Validated for Authorization {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            Authorization::Cipher(ParameterCipher { .. }) => ValidationState::empty(),
            Authorization::Plain(plain) => plain.get_validation_state(),
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            Authorization::Cipher(ParameterCipher { .. }) => &None,
            Authorization::Plain(plain) => plain.get_validation_warnings(),
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        if let Authorization::Plain(authorization) = self {
            authorization.set_validation_warnings(warnings);
        }
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            Authorization::Cipher(ParameterCipher { .. }) => &None,
            Authorization::Plain(plain) => plain.get_validation_errors(),
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        if let Authorization::Plain(authorization) = self {
            authorization.set_validation_errors(errors);
        }
    }
}

impl AuthorizationPlain {
    pub fn perform_validation(&mut self) {
        self.validate_name();
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
            AuthorizationPlain::Basic {
                name,
                validation_errors,
                validation_state,
                ..
            } => {
                perform_validation(name, validation_errors, validation_state);
            }
            AuthorizationPlain::OAuth2Client {
                name,
                validation_errors,
                validation_state,
                ..
            } => {
                perform_validation(name, validation_errors, validation_state);
            }
            AuthorizationPlain::OAuth2Pkce {
                name,
                validation_errors,
                validation_state,
                ..
            } => {
                perform_validation(name, validation_errors, validation_state);
            }
            AuthorizationPlain::ApiKey {
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

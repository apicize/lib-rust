use std::collections::HashMap;

use crate::parameters::{EncryptableParameter, ParameterCipher, ParameterEncryption};
use crate::{
    ApicizeError, Identifiable, Validated, ValidationState, add_validation_error, decrypt, encrypt,
    remove_validation_error, utility::*,
};
use serde::{Deserialize, Serialize};

use super::Variable;
use super::identifiable::CloneIdentifiable;

/// A set of variables that can be injected into templated values
/// when submitting an Apicize Request that may be encrypted
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Scenario {
    Cipher(ParameterCipher),
    Plain(Box<ScenarioPlain>),
}

/// A set of variables that can be injected into templated values
/// when submitting an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioPlain {
    /// Uniquely identifies scenario
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Name of variable to substitute (avoid using curly braces)
    pub name: String,
    /// Value of variable to substitute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<Variable>>,
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

/// Values that are stored as when encrypting a scenario
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct ScenarioEncryptedData {
    /// Value of variable to substitute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<Variable>>,
}

impl Default for ScenarioPlain {
    fn default() -> Self {
        Self {
            id: generate_uuid(),
            name: Default::default(),
            variables: Default::default(),
            validation_state: Default::default(),
            validation_warnings: Default::default(),
            validation_errors: Default::default(),
        }
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Scenario::Plain(Box::default())
    }
}

impl Identifiable for ScenarioPlain {
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

impl Identifiable for Scenario {
    fn get_id(&self) -> &str {
        match self {
            Scenario::Cipher(cipher) => cipher.get_id(),
            Scenario::Plain(plain) => plain.get_id(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Scenario::Cipher(cipher) => cipher.get_name(),
            Scenario::Plain(plain) => plain.get_name(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            Scenario::Cipher(cipher) => cipher.get_title(),
            Scenario::Plain(plain) => plain.get_title(),
        }
    }
}

impl CloneIdentifiable for Scenario {
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        match &mut cloned {
            Scenario::Cipher(cipher) => {
                cipher.id = generate_uuid();
                cipher.name = new_name;
            }
            Scenario::Plain(plain) => {
                plain.id = generate_uuid();
                plain.name = new_name;
            }
        }
        cloned
    }
}

impl EncryptableParameter for Scenario {
    fn is_encrypted(&self) -> bool {
        matches!(self, Scenario::Cipher(_))
    }

    fn encrypt(
        &self,
        password: &str,
        method: ParameterEncryption,
    ) -> Result<Scenario, ApicizeError> {
        let Scenario::Plain(scenario) = self else {
            return Err(ApicizeError::Encryption {
                description: "Encrypted scenarios cannot be re-encrypted".to_string(),
            });
        };

        let data = serde_json::to_string(&ScenarioEncryptedData {
            variables: scenario.variables.clone(),
        })
        .map_err(|err| ApicizeError::Encryption {
            description: format!("Unable to serialize scenario - {}", err),
        })?;
        Ok(Scenario::Cipher(ParameterCipher {
            id: scenario.id.to_string(),
            name: scenario.name.to_string(),
            data: encrypt(&data, password, method)?,
        }))
    }

    fn decrypt(
        &self,
        password: &str,
        method: ParameterEncryption,
    ) -> Result<Scenario, ApicizeError> {
        let Scenario::Cipher(scenario) = self else {
            return Err(ApicizeError::Encryption {
                description: "Scenario is already decrypted".to_string(),
            });
        };

        let data = serde_json::from_str::<ScenarioEncryptedData>(&decrypt(
            &scenario.data,
            password,
            method,
        )?)
        .map_err(|err| ApicizeError::Encryption {
            description: format!("Unable to deserialize scenario - {}", err),
        })?;

        Ok(Scenario::Plain(Box::new(ScenarioPlain {
            id: scenario.id.to_string(),
            name: scenario.name.to_string(),
            variables: data.variables,
            validation_state: ValidationState::empty(),
            validation_warnings: None,
            validation_errors: None,
        })))
    }
}

impl ScenarioPlain {
    pub fn perform_validation(&mut self) {
        self.validate_name();
        self.validate_variables();
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

    pub fn validate_variables(&mut self) {
        let variables_ok = match &self.variables {
            Some(vars) => !vars.iter().any(|v| v.name.trim().is_empty()),
            None => true,
        };
        if variables_ok {
            remove_validation_error(&mut self.validation_errors, "variables");
        } else {
            add_validation_error(
                &mut self.validation_errors,
                "variables",
                "Variables must be named",
            );
        }
        self.validation_state
            .set(ValidationState::ERROR, self.validation_errors.is_some());
    }
}

impl Validated for ScenarioPlain {
    fn get_validation_state(&self) -> ValidationState {
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

impl Validated for Scenario {
    fn get_validation_state(&self) -> ValidationState {
        match self {
            Scenario::Cipher(_cipher) => ValidationState::empty(),
            Scenario::Plain(plain) => plain.get_validation_state(),
        }
    }

    fn get_validation_warnings(&self) -> &Option<Vec<String>> {
        match self {
            Scenario::Cipher(_cipher) => &None,
            Scenario::Plain(plain) => plain.get_validation_warnings(),
        }
    }

    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>) {
        if let Scenario::Plain(scenario) = self {
            scenario.set_validation_warnings(warnings)
        }
    }

    fn get_validation_errors(&self) -> &Option<HashMap<String, String>> {
        match self {
            Scenario::Cipher(_cipher) => &None,
            Scenario::Plain(plain) => plain.get_validation_errors(),
        }
    }

    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>) {
        if let Scenario::Plain(scenario) = self {
            scenario.set_validation_errors(errors)
        }
    }
}

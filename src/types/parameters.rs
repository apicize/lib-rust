use std::{
    env, path::{self, Path, PathBuf}
};

use dirs::config_dir;
use serde::{Deserialize, Serialize};

use crate::{
    ApicizeError, Authorization, Certificate, Identifiable, ParameterLockStatus, Proxy, Scenario,
    SerializationSaveSuccess, Validated, delete_data_file, open_data_file, save_data_file,
};

/// Type of encyrption used to encrypt sensitive parameter info
#[derive(Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ParameterEncryption {
    #[default]
    Aes256Gcm,
}

/// Trait describing behaviors of encryptable parameters
pub trait EncryptableParameter: Sized {
    /// Returns True if the parameter is encrypted
    fn is_encrypted(&self) -> bool;
    /// Returns an encrypted copy of the parameter
    fn encrypt(&self, password: &str, method: ParameterEncryption) -> Result<Self, ApicizeError>;
    /// Returns a decrypted copy of the parameter
    fn decrypt(&self, password: &str, method: ParameterEncryption) -> Result<Self, ApicizeError>;
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ParameterCipher {
    /// Uniquely identifies entity
    pub id: String,
    /// Name of entity
    pub name: String,
    // Encrypted data of entity information
    pub data: String,
}

impl Identifiable for ParameterCipher {
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

/// Stored parameters, authorization, client certificates and proxies
/// stored outside of a workbook, either globally or alongside a workbook
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Parameters {
    /// Version of  format (should not be changed manually)
    pub version: f32,

    /// Encryption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<ParameterEncryption>,

    /// Scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenarios: Option<Vec<Scenario>>,

    /// Authorizations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizations: Option<Vec<Authorization>>,

    /// Certificates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificates: Option<Vec<Certificate>>,

    /// Proxy servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<Proxy>>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            version: 1.0,
            encryption: Default::default(),
            scenarios: Default::default(),
            authorizations: Default::default(),
            certificates: Default::default(),
            proxies: Default::default(),
        }
    }
}

impl Parameters {
    pub fn new(
        scenarios: Option<Vec<Scenario>>,
        authorizations: Option<Vec<Authorization>>,
        certificates: Option<Vec<Certificate>>,
        proxies: Option<Vec<Proxy>>,
    ) -> Self {
        Self {
            version: 1.0,
            encryption: Default::default(),
            scenarios,
            authorizations,
            certificates,
            proxies,
        }
    }

    /// Open parameters from parameters file; or, if specified, create default if parameters file does not exist
    pub fn open(
        file_name: &PathBuf,
        create_new_if_missing: bool,
    ) -> Result<Parameters, ApicizeError> {
        if Path::new(&file_name).is_file() {
            let params = open_data_file::<Parameters>(file_name)?.data;
            Ok(params)
        } else if create_new_if_missing {
            Ok(Parameters::default())
        } else {
            Err(ApicizeError::FileAccess {
                file_name: Some(file_name.to_string_lossy().to_string()),
                description: "Not found".to_string(),
            })
        }
    }

    /// Save parametesr to file if it contains entries, otherwise, delete it
    pub fn save(
        &self,
        file_name: &PathBuf,
        destination_name: &str,
        password: &Option<String>,
    ) -> Result<SerializationSaveSuccess, ApicizeError> {
        let method = ParameterEncryption::default();
        let scenarios = match &self.scenarios {
            Some(entities) => entities.iter().map(|e| e.to_owned()).collect(),
            None => vec![],
        };
        let auths = match &self.authorizations {
            Some(entities) => entities.iter().map(|e| e.to_owned()).collect(),
            None => vec![],
        };
        let certs = match &self.certificates {
            Some(entities) => entities.iter().map(|e| e.to_owned()).collect(),
            None => vec![],
        };
        let proxies = match &self.proxies {
            Some(entities) => entities.iter().map(|e| e.to_owned()).collect(),
            None => vec![],
        };

        if scenarios.iter().any(|s| s.is_encrypted())
            || auths.iter().any(|a| a.is_encrypted())
            || certs.iter().any(|c| c.is_encrypted())
            || proxies.iter().any(|s| matches!(s, Proxy::Cipher(_)))
        {
            return Err(ApicizeError::Encryption {
                description: format!("{destination_name} must be unlocked before saving"),
            });
        }

        if scenarios.is_empty() && auths.is_empty() && certs.is_empty() && proxies.is_empty() {
            delete_data_file(file_name)
        } else {
            let (encryption, scenarios, authorizations, certificates, proxies) =
                if let Some(password) = password {
                    let encrypted_scenarios = if scenarios.is_empty() {
                        None
                    } else {
                        Some(
                            scenarios
                                .iter()
                                .map(|s| s.encrypt(password, method))
                                .collect::<Result<Vec<Scenario>, _>>()?,
                        )
                    };
                    let encrypted_auths = if auths.is_empty() {
                        None
                    } else {
                        Some(
                            auths
                                .iter()
                                .map(|p| p.encrypt(password, method))
                                .collect::<Result<Vec<Authorization>, _>>()?,
                        )
                    };
                    let encrypted_certs = if certs.is_empty() {
                        None
                    } else {
                        Some(
                            certs
                                .iter()
                                .map(|p| p.encrypt(password, method))
                                .collect::<Result<Vec<Certificate>, _>>()?,
                        )
                    };
                    let encrypted_proxies = if proxies.is_empty() {
                        None
                    } else {
                        Some(
                            proxies
                                .iter()
                                .map(|p| p.encrypt(password, method))
                                .collect::<Result<Vec<Proxy>, _>>()?,
                        )
                    };
                    (
                        Some(method),
                        encrypted_scenarios,
                        encrypted_auths,
                        encrypted_certs,
                        encrypted_proxies,
                    )
                } else {
                    (
                        None,
                        (!scenarios.is_empty()).then_some(scenarios),
                        (!auths.is_empty()).then_some(auths),
                        (!certs.is_empty()).then_some(certs),
                        (!proxies.is_empty()).then_some(proxies),
                    )
                };

            save_data_file(
                file_name,
                &Parameters {
                    version: 1.0,
                    encryption,
                    scenarios,
                    authorizations,
                    certificates,
                    proxies,
                },
            )
        }
    }

    pub fn get_parameter_count(&self) -> usize {
        self.scenarios.as_ref().map_or(0, |v| v.len())
            + self.authorizations.as_ref().map_or(0, |v| v.len())
            + self.certificates.as_ref().map_or(0, |v| v.len())
            + self.proxies.as_ref().map_or(0, |v| v.len())
    }

    /// Return the file name for globals
    pub fn get_globals_filename() -> path::PathBuf {
        if let Some(directory) = config_dir() {
            directory.join("apicize").join("globals.json")
        } else {
            panic!("Operating system did not provide configuration directory")
        }
    }

    /// Return the file name for a workbook's private parameters file
    pub fn get_workbook_private_filename(workbook_path: &Path) -> path::PathBuf {
        let mut private_path = PathBuf::from(workbook_path);
        private_path.set_extension("apicize-priv");
        private_path
    }

    /// Returns true if any parameters are encrypted
    fn any_encrypted_parameters<T>(parameters: &Option<Vec<T>>) -> bool
    where
        T: EncryptableParameter,
    {
        parameters
            .as_ref()
            .is_some_and(|v| v.iter().any(|item| item.is_encrypted()))
    }

    /// Encrypt parameters with the specified password
    fn encrypt_parameters<T>(
        parameters: &Option<Vec<T>>,
        password: &str,
        method: ParameterEncryption,
    ) -> Result<Option<Vec<T>>, ApicizeError>
    where
        T: EncryptableParameter + Clone,
    {
        match parameters {
            Some(parameters) => Ok(Some(
                parameters
                    .iter()
                    .map(|p| {
                        if p.is_encrypted() {
                            p.encrypt(password, method)
                        } else {
                            Ok(p.clone())
                        }
                    })
                    .collect::<Result<Vec<T>, _>>()?,
            )),
            None => Ok(None),
        }
    }

    /// Decrypt parameters in a list, adding a warning if there is a problem decrypting.
    /// The function will decrypt any parameters it can and leave ciphers in place
    /// with warnings for any failures
    fn decrypt_parameters<T>(
        parameters: &mut Option<Vec<T>>,
        password: &str,
        method: ParameterEncryption,
    ) -> bool
    where
        T: EncryptableParameter + Validated,
    {
        let mut ok = true;
        if let Some(parameters) = parameters {
            for param in parameters.iter_mut() {
                if !param.is_encrypted() {
                    continue;
                }

                match param.decrypt(password, method) {
                    Ok(decrypted) => {
                        *param = decrypted;
                    }
                    Err(err) => {
                        ok = false;
                        param.set_validation_warnings(Some(vec![err.to_string()]));
                    }
                }
            }
        }
        ok
    }

    /// Return true if any paramerers are encrypted
    pub fn any_encyrypted(&self) -> bool {
        Self::any_encrypted_parameters(&self.scenarios)
            || Self::any_encrypted_parameters(&self.authorizations)
            || Self::any_encrypted_parameters(&self.certificates)
            || Self::any_encrypted_parameters(&self.proxies)
    }

    /// Encrypt any plain entries
    pub fn encyrpt(
        &mut self,
        password: &str,
        method: ParameterEncryption,
    ) -> Result<Parameters, ApicizeError> {
        let mut params = Parameters {
            version: self.version,
            encryption: None,
            scenarios: Self::encrypt_parameters(&self.scenarios, password, method)?,
            authorizations: Self::encrypt_parameters(&self.authorizations, password, method)?,
            certificates: Self::encrypt_parameters(&self.certificates, password, method)?,
            proxies: Self::encrypt_parameters(&self.proxies, password, method)?,
        };
        if Self::any_encyrypted(self) {
            params.encryption = Some(ParameterEncryption::Aes256Gcm);
        }
        Ok(params)
    }

    /// Decrypt the listed parameters, when encrypted, using password, if specified,
    /// or fallback env variable,if defined; returns the lock status and active password
    pub fn decrypt(
        &mut self,
        password: Option<&str>,
        fallback_env_variable: Option<&str>,
    ) -> (ParameterLockStatus, Option<String>) {
        let method = self.encryption.unwrap_or_default();

        let is_encrypted = self.any_encyrypted();
        if !is_encrypted {
            return (ParameterLockStatus::UnlockedNoPassword, None);
        }

        let env_password = if password.as_ref().is_none_or(|pw| pw.is_empty()) {
            fallback_env_variable
                .and_then(|var| env::var(var).ok())
                .filter(|pw| !pw.is_empty())
        } else {
            None
        };

        let (password, use_env_var) = if let Some(ref pw) = env_password {
            (Some(pw.as_str()), true)
        } else if password.is_some() {
            (password, false)
        } else {
            (None, false)
        };

        let unlocked = if let Some(password) = &password {
            Self::decrypt_parameters(&mut self.scenarios, password, method)
                && Self::decrypt_parameters(&mut self.authorizations, password, method)
                && Self::decrypt_parameters(&mut self.certificates, password, method)
                && Self::decrypt_parameters(&mut self.proxies, password, method)
        } else {
            false
        };

        let status = if unlocked {
            if password.is_some() {
                if use_env_var {
                    ParameterLockStatus::UnlockedWithEnvVar
                } else {
                    ParameterLockStatus::UnlockedWithPassword
                }
            } else {
                ParameterLockStatus::UnlockedNoPassword
            }
        } else if password.is_some() {
            if use_env_var {
                ParameterLockStatus::LockedInvalidEnvVar
            } else {
                ParameterLockStatus::LockedInvalidPassword
            }
        } else {
            ParameterLockStatus::Locked
        };

        (status, password.map(|p| p.to_string()))
    }
}

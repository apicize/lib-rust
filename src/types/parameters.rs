use std::path::{self, Path, PathBuf};

use dirs::config_dir;
use serde::{Deserialize, Serialize};

use crate::{
    delete_data_file, open_data_file, save_data_file, Authorization, Certificate, Proxy, Scenario,
    SerializationError, FileAccessError, SerializationSaveSuccess,
};

/// Stored parameters, authorization, client certificates and proxies
/// stored outside of a workbook, either globally or alongside a workbook
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Parameters {
    /// Version of  format (should not be changed manually)
    pub version: f32,

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
        proxies: Option<Vec<Proxy>>) -> Self {
        Self {
            version: 1.0,
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
    ) -> Result<Parameters, FileAccessError> {
        if Path::new(&file_name).is_file() {
            let params = open_data_file::<Parameters>(file_name)?.data;
            Ok(params)
        } else if create_new_if_missing {
            Ok(Parameters::default())
        } else {
            Err(FileAccessError {
                file_name: String::from(file_name.to_string_lossy()),
                error: SerializationError::IO(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("{} not found", &file_name.to_string_lossy()),
                )),
            })
        }
    }

    /// Save parametesr to file if it contains entries, otherwise, delete it
    pub fn save(
        &self,
        file_name: &PathBuf,
    ) -> Result<SerializationSaveSuccess, FileAccessError> {
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

        if scenarios.is_empty() && auths.is_empty() && certs.is_empty() && proxies.is_empty() {
            delete_data_file(file_name)
        } else {
            save_data_file(
                file_name,
                &Parameters {
                    version: 1.0,
                    scenarios: {
                        if scenarios.is_empty() {
                            None
                        } else {
                            Some(scenarios)
                        }
                    },
                    authorizations: {
                        if auths.is_empty() {
                            None
                        } else {
                            Some(auths)
                        }
                    },
                    certificates: {
                        if certs.is_empty() {
                            None
                        } else {
                            Some(certs)
                        }
                    },
                    proxies: {
                        if proxies.is_empty() {
                            None
                        } else {
                            Some(proxies)
                        }
                    },
                },
            )
        }
    }

    /// Return the file name for globals
    pub fn get_globals_filename() -> path::PathBuf {
        if let Some(directory) = config_dir() {
            directory.join("apicize").join("globals.json")
        } else {
            panic!("Operating system did not provide configuration directory")
        }
    }

    /// Return the file name for a workbook's vault
    pub fn get_workbook_vault_filename(workbook_path: &Path) -> path::PathBuf {
        let mut private_path = PathBuf::from(workbook_path);
        private_path.set_extension("apicize-priv");
        private_path
    }
}

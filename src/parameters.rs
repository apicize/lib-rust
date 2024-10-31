//! Apicize parameters
//! 
//! This module defines models used to retrieve and save parameter information from private and global parameter files

use std::path::{self, Path, PathBuf};

use dirs::config_dir;
use ::serde::{Deserialize, Serialize};

use super::{
    delete_data_file, open_data_file, save_data_file, SerializationFailure, SerializationOpenSuccess, SerializationSaveSuccess, WorkbookAuthorization, WorkbookCertificate, WorkbookProxy, WorkbookScenario, WorkspaceParameter
};

/// Persisted Apicize authorization, client certificates and proxies
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Parameters {
    /// Version of workbook format (should not be changed manually)
    pub version: f32,

    /// Workbook credential authorizations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizations: Option<Vec<WorkbookAuthorization>>,

    /// Workbook credential scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenarios: Option<Vec<WorkbookScenario>>,

    /// Workbook credential certificates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificates: Option<Vec<WorkbookCertificate>>,

    /// Workbook credential proxy servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<WorkbookProxy>>,
}

impl Parameters {
    /// Return the file name for globals
    pub fn get_globals_filename() -> path::PathBuf {
        if let Some(directory) = config_dir() {
            directory.join("apicize").join("globals.json")
        } else {
            panic!("Operating system did not provide configuration directory")
        }
    }

    // Return the file name for private workbook options
    fn get_private_options_filename(workbook_path: &Path) -> path::PathBuf {
        let mut private_path = PathBuf::from(workbook_path);
        private_path.set_extension("apicize-priv");
        private_path
    }

    /// Return global parameters information
    pub fn open_global_parameters(
        global_parameters_filename: Option<PathBuf>,
    ) -> Result<SerializationOpenSuccess<Parameters>, SerializationFailure> {
        Self::open(if let Some(filename) = global_parameters_filename {
            filename.clone()
        } else {
            Self::get_globals_filename()
        })
    }

    /// Return workbook private parameter information
    pub fn open_workbook_private_parameters(
        workbook_path: &Path,
    ) -> Result<SerializationOpenSuccess<Parameters>, SerializationFailure> {
        Self::open(Self::get_private_options_filename(workbook_path))
    }

    /// Save global parameters information
    pub fn save_global_parameters(
        scenarios: &[WorkbookScenario],
        authorizations: &[WorkbookAuthorization],
        certificates: &[WorkbookCertificate],
        proxies: &[WorkbookProxy],
    ) -> Result<SerializationSaveSuccess, SerializationFailure> {
        Self::save(
            Self::get_globals_filename(),
            scenarios,
            authorizations,
            certificates,
            proxies,
        )
    }

    /// Save workbook parameters information
    pub fn save_workbook_private_parameters(
        workbook_path: &Path,
        scenarios: &[WorkbookScenario],
        authorizations: &[WorkbookAuthorization],
        certificates: &[WorkbookCertificate],
        proxies: &[WorkbookProxy],
    ) -> Result<SerializationSaveSuccess, SerializationFailure> {
        Self::save(
            Self::get_private_options_filename(workbook_path),
            scenarios,
            authorizations,
            certificates,
            proxies,
        )
    }

    /// Return parameters information or default if parameters file does not exist
    fn open(
        credential_file_name: PathBuf,
    ) -> Result<SerializationOpenSuccess<Parameters>, SerializationFailure> {
        if Path::new(&credential_file_name).is_file() {
            open_data_file::<Parameters>(&credential_file_name)
        } else {
            Ok(SerializationOpenSuccess {
                file_name: String::from(credential_file_name.to_string_lossy()),
                data: Parameters {
                    version: 1.0,
                    scenarios: None,
                    authorizations: None,
                    certificates: None,
                    proxies: None,
                },
            })
        }
    }

    /// Save credential information
    fn save(
        file_name_to_save: PathBuf,
        scenarios: &[WorkbookScenario],
        authorizations: &[WorkbookAuthorization],
        certificates: &[WorkbookCertificate],
        proxies: &[WorkbookProxy],
    ) -> Result<SerializationSaveSuccess, SerializationFailure> {
        let mut any = false;
        let save_scenarios = if scenarios.is_empty() {
            None
        } else {
            any = true;
            Some(
                scenarios
                    .iter()
                    .map(|e| {
                        let mut cloned = e.clone();
                        cloned.clear_persistence();
                        cloned
                    })
                    .collect(),
            )
        };
        let save_authorizations = if authorizations.is_empty() {
            None
        } else {
            any = true;
            Some(
                authorizations
                    .iter()
                    .map(|e| {
                        let mut cloned = e.clone();
                        cloned.clear_persistence();
                        cloned
                    })
                    .collect(),
            )
        };
        let save_certiificates = if certificates.is_empty() {
            None
        } else {
            any = true;
            Some(
                certificates
                    .iter()
                    .map(|e| {
                        let mut cloned = e.clone();
                        cloned.clear_persistence();
                        cloned
                    })
                    .collect(),
            )
        };
        let save_proxies = if proxies.is_empty() {
            None
        } else {
            any = true;
            Some(
                proxies
                    .iter()
                    .map(|e| {
                        let mut cloned = e.clone();
                        cloned.clear_persistence();
                        cloned
                    })
                    .collect(),
            )
        };

        if any {
            let globals = Parameters {
                version: 1.0,
                scenarios: save_scenarios,
                authorizations: save_authorizations,
                certificates: save_certiificates,
                proxies: save_proxies,
            };
            save_data_file(&file_name_to_save, &globals)
        } else {
            delete_data_file(&file_name_to_save)
        }
    }
}

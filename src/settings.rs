//! Settings models submodule
//!
//! This submodule defines models used to store application settings

use std::{
    fs::create_dir_all,
    path::{self, Path},
};

use dirs::{config_dir, document_dir, home_dir};
use serde::{Deserialize, Serialize};

use super::{
    open_data_file, save_data_file, SerializationFailure, SerializationOpenSuccess,
    SerializationSaveSuccess,
};

fn default_font_size() -> i32 {
    12
}

fn default_color_scheme() -> ColorScheme {
    ColorScheme::Dark
}

fn default_pkce_listener_port() -> u16 {
    8080
}


#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
/// Color scheme for UI app
pub enum ColorScheme {
    /// Light mode
    Light,
    /// Dark mode
    Dark,
}

/// Apicize application settings
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApicizeSettings {
    /// Default directory that workbooks are stored in
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workbook_directory: Option<String>,

    /// Last opened/saved workbook name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_workbook_file_name: Option<String>,

    /// Font Size
    #[serde(default = "default_font_size")]
    pub font_size: i32,

    /// Color scheme for UI app
    #[serde(default = "default_color_scheme")]
    pub color_scheme: ColorScheme,

    #[serde(default)]
    /// Layout for editor panels (UI)
    pub editor_panels: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Recent workbook file names opened in UI
    pub recent_workbook_file_names: Option<Vec<String>>,

    /// Port for UI PKCE listener
    #[serde(default = "default_pkce_listener_port")]
    pub pkce_listener_port: u16,
}

impl ApicizeSettings {
    /// Return default workbooks directory
    pub fn get_workbooks_directory() -> path::PathBuf {
        if let Some(directory) = document_dir() {
            directory.join("apicize")
        } else if let Some(directory) = home_dir() {
            directory.join("apicize")
        } else {
            panic!("Operating system did not provide document or home directory")
        }
    }

    fn get_settings_directory() -> path::PathBuf {
        if let Some(directory) = config_dir() {
            directory.join("apicize")
        } else {
            panic!("Operating system did not provide configuration directory")
        }
    }

    /// Return the file name for settings
    fn get_settings_filename() -> path::PathBuf {
        Self::get_settings_directory().join("settings.json")
    }

    /// Open Apicize common environment from the specified name in the default path
    pub fn open() -> Result<SerializationOpenSuccess<ApicizeSettings>, SerializationFailure> {
        let file_name = &Self::get_settings_filename();
        if Path::new(&file_name).is_file() {
            open_data_file::<ApicizeSettings>(&Self::get_settings_filename())
        } else {
            // Return default settings if no existing settings file exists
            let settings = ApicizeSettings {
                last_workbook_file_name: None,
                workbook_directory: Some(String::from(Self::get_workbooks_directory().to_string_lossy())),
                font_size: 12,
                color_scheme: ColorScheme::Dark,
                editor_panels: String::from(""),
                recent_workbook_file_names: None,
                pkce_listener_port: 8080,
            };
            Ok(SerializationOpenSuccess {
                file_name: String::from(""),
                data: settings,
            })
        }
    }

    /// Save Apicize common environment to the specified name in the default path
    pub fn save(&self) -> Result<SerializationSaveSuccess, SerializationFailure> {
        let dir = Self::get_settings_directory();
        if !Path::new(&dir).is_dir() {
            if let Err(err) = create_dir_all(&dir) {
                panic!("Unable to create {} - {}", &dir.to_string_lossy(), err);
            }
        }
        save_data_file(&Self::get_settings_filename(), self)
    }
}

//! Shared models submodule
//!
//! This submodule defines information used globally

use serde::{Deserialize, Serialize};
use crate::Selection;




impl SelectableOptionType {
    /// Convert to readable string
    pub fn as_str(&self) -> &'static str {
        match self {
            SelectableOptionType::Scenario => "scenario",
            SelectableOptionType::Authorization => "authorization",
            SelectableOptionType::Certificate => "certificate",
            SelectableOptionType::Proxy => "proxy",
        }
    }
}

/// Whether a missing selectable option defaults to the parent or to None
pub enum SelectableOptionDefaultType {
    /// The request/group parent will be used as a default if no value is provided
    Parent,
    /// No default will be used if no value is provided
    None,
}

impl SelectableOptionDefaultType {
    /// Render default type as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            SelectableOptionDefaultType::Parent => "parent",
            SelectableOptionDefaultType::None => "none",
        }
    }
}




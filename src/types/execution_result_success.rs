use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExecutionResultSuccess {
    Success,
    Failure,
    Error,
}

impl Display for ExecutionResultSuccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionResultSuccess::Success => write!(f, "SUCCESS"),
            ExecutionResultSuccess::Failure => write!(f, "FAILURE"),
            ExecutionResultSuccess::Error => write!(f, "ERROR"),
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionResultSuccess {
    Success = 0,
    Failure = 1,
    Error = 2
}

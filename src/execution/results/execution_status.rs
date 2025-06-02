use serde::{Serialize, Deserialize};
use super::execution_result_summary::ExecutionResultSummary;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ExecutionStatus {
    pub request_or_group_id: String,
    pub running: bool,
    pub results: Option<Vec<ExecutionResultSummary>>
}
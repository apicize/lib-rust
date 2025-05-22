use crate::{
    ApicizeExecution, ApicizeGroupResultRow, ApicizeGroupResultRun, ApicizeRequestResultRow, ApicizeRequestResultRun, ApicizeResult
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Values active for a set if results (grouped requetss, runs, rows)
#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DataContext {
    /// Variables available from scenario
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Map<String, Value>>,

    /// Variables output from previous test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Map<String, Value>>,

    /// Row data assigned to the groups' requests (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<String, Value>>,

    /// Output variables resulting from operation to be sent to next request/group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_result: Option<Map<String, Value>>,

}

/// Trait to retrieve test contexts from enums
pub trait GetDataContext {
    fn get_data_context(&self) -> &DataContext;
}

impl GetDataContext for ApicizeResult {
    fn get_data_context(&self) -> &DataContext {
        match self {
            ApicizeResult::Request(request) => &request.data_context,
            ApicizeResult::Group(group) => &group.data_context,
        }
    }
}

/// Trait to generate test contexts for arrays of results
pub trait DataContextGenerator {
    fn generate_data_context(&self) -> DataContext;
}

impl DataContextGenerator for Vec<ApicizeGroupResultRow> {
    fn generate_data_context(&self) -> DataContext {
        if let Some(first) = self.first() {
            let first_context = &first.data_context;
            let last_context = &self.last().unwrap().data_context;
            let mut data_context = first_context.clone();
            data_context.output_result = last_context.output_result.clone();
            data_context
        } else {
            DataContext::default()
        }
    }
}

impl DataContextGenerator for Vec<ApicizeGroupResultRun> {
    fn generate_data_context(&self) -> DataContext {
        if let Some(first) = self.first() {
            let first_context = &first.data_context;
            let last_context = &self.last().unwrap().data_context;
            let mut data_context = first_context.clone();
            data_context.output_result = last_context.output_result.clone();
            data_context
        } else {
            DataContext::default()
        }
    }
}

impl DataContextGenerator for Vec<ApicizeRequestResultRow> {
    fn generate_data_context(&self) -> DataContext {
        if let Some(first) = self.first() {
            let first_context = &first.data_context;
            let last_context = &self.last().unwrap().data_context;
            let mut data_context = first_context.clone();
            data_context.output_result = last_context.output_result.clone();
            data_context
        } else {
            DataContext::default()
        }
    }
}

impl DataContextGenerator for Vec<ApicizeRequestResultRun> {
    fn generate_data_context(&self) -> DataContext {
        if let Some(first) = self.first() {
            let first_execution = &first.execution;
            let last_execution = &self.last().unwrap().execution;
            DataContext {
                variables: first_execution.test_context.variables.clone(),
                data: first_execution.test_context.data.clone(),
                output: first_execution.test_context.output.clone(),
                output_result: last_execution.output_variables.clone()
            }
        } else {
            DataContext::default()
        }
    }
}

impl DataContextGenerator for Vec<ApicizeResult> {
    fn generate_data_context(&self) -> DataContext {
        if let Some(first) = self.first() {
            let first_context = first.get_data_context();
            let last_context = self.last().unwrap().get_data_context();
            let mut data_context = first_context.clone();
            data_context.output_result = last_context.output_result.clone();
            data_context
        } else {
            DataContext::default()
        }
    }
}

impl DataContextGenerator for ApicizeExecution {
    fn generate_data_context(&self) -> DataContext {
        DataContext { variables: self.test_context.variables.clone(), output: self.test_context.output.clone(), data: self.test_context.data.clone(), output_result: self.output_variables.clone() }
    }
}
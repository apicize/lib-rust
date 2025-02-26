use serde_json::{Map, Value};

use super::{
    ApicizeExecution, ApicizeExecutionType, ApicizeGroupItem, ApicizeGroupRun, ApicizeRequest,
};

pub trait OutputVariables {
    fn get_output_variables(&self) -> Option<Map<String, Value>>;
}

impl OutputVariables for ApicizeGroupItem {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        match self {
            ApicizeGroupItem::Group(g) => g.output_variables.clone(),
            ApicizeGroupItem::Request(r) => r.output_variables.clone(),
        }
    }
}
impl OutputVariables for Vec<ApicizeGroupItem> {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.last().and_then(|l| l.get_output_variables())
    }
}

impl OutputVariables for ApicizeGroupRun {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.output_variables.clone()
    }
}

impl OutputVariables for Vec<ApicizeGroupRun> {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.last().and_then(|l| l.get_output_variables())
    }
}

impl OutputVariables for ApicizeRequest {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.output_variables.clone()
    }
}

impl OutputVariables for ApicizeExecution {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.output_variables.clone()
    }
}

impl OutputVariables for Vec<ApicizeExecution> {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.last().and_then(|e| e.output_variables.clone())
    }
}

impl OutputVariables for ApicizeExecutionType {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        match self {
            ApicizeExecutionType::None => None,
            ApicizeExecutionType::Single(execution) => execution.output_variables.clone(),
            ApicizeExecutionType::Runs(execution) => {
                execution.items.last().and_then(|e| e.output_variables.clone())
            }
            ApicizeExecutionType::Rows(execution) => {
                execution.items.last().and_then(|e| e.output_variables.clone())
            }
            ApicizeExecutionType::MultiRunRows(rows) => {
                rows.items.last().and_then(|e| e.output_variables.clone())
            }
        }
    }
}

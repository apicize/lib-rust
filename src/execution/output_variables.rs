use serde_json::{Map, Value};

use super::{ApicizeExecution, ApicizeExecutionDetail, ApicizeExecutionSummary, ApicizeItem, ApicizeList};

pub trait OutputVariables {
    fn get_output_variables(&self) -> Option<Map<String, Value>>;
}

impl OutputVariables for ApicizeItem {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        match self {
            ApicizeItem::Group(g) => g.output_variables.clone(),
            ApicizeItem::Request(r) => r.output_variables.clone(),
            ApicizeItem::ExecutedRequest(e) => e.output_variables.clone(),
            ApicizeItem::Execution(e) => match e {
                ApicizeExecution::Rows(summaries) => summaries.get_output_variables(),
                ApicizeExecution::Runs(details) => details.get_output_variables(),
                ApicizeExecution::Details(items) => items.get_output_variables(),
            },
            ApicizeItem::Items(items) => items.get_output_variables(),
            ApicizeItem::ExecutionSummaries(summaries) => summaries.get_output_variables(),
        }
    }
}
impl OutputVariables for ApicizeList<Box<ApicizeItem>> {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.items.last().and_then(|l| l.get_output_variables())
    }
}

impl OutputVariables for ApicizeList<ApicizeExecutionDetail> {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.last().and_then(|l| l.output_variables.clone())
    }
}

impl OutputVariables for ApicizeList<ApicizeExecutionSummary> {
    fn get_output_variables(&self) -> Option<Map<String, Value>> {
        self.last().and_then(|l| l.output_variables.clone())
    }
}
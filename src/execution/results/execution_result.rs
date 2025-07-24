use super::{
    ExecutionResultDetail, ExecutionResultDetailGroup, ExecutionResultSuccess,
    ExecutionResultSummary,
};
use crate::{
    ApicizeBody, ApicizeExecution, ApicizeGroupResult, ApicizeGroupResultContent,
    ApicizeGroupResultRow, ApicizeGroupResultRowContent, ApicizeGroupResultRun,
    ApicizeRequestResult, ApicizeRequestResultContent, ApicizeRequestResultRow,
    ApicizeRequestResultRowContent, ApicizeRequestResultRun, ApicizeResult,
    ExecutionResultDetailRequest, Identifiable, Tally, TestRunnerContext,
};

pub type ExecutionResult = (ExecutionResultSummary, ExecutionResultDetail);

fn success_from_tallies(tally: &dyn Tally) -> ExecutionResultSuccess {
    let tallies = tally.get_tallies();
    if tallies.request_error_count > 0 {
        ExecutionResultSuccess::Error
    } else if tallies.test_fail_count > 0 || tallies.request_failure_count > 0 {
        ExecutionResultSuccess::Failure
    } else {
        ExecutionResultSuccess::Success
    }
}

fn get_response_info(
    execution: &ApicizeExecution,
) -> (Option<u16>, Option<String>, bool, Option<usize>) {
    match &execution.test_context.response {
        Some(response) => {
            let response_body_length = match &response.body {
                Some(body) => match body {
                    ApicizeBody::Text { text } => text.len(),
                    ApicizeBody::JSON { text, .. } => text.len(),
                    ApicizeBody::XML { text, .. } => text.len(),
                    ApicizeBody::Form { text, .. } => text.len(),
                    ApicizeBody::Binary { data } => data.len(),
                },
                None => 0,
            };
            (
                Some(response.status),
                Some(response.status_text.clone()),
                response.headers.is_some(),
                Some(response_body_length),
            )
        }
        None => (None, None, false, None),
    }
}

// impl ListAppendable for ApicizeRequestResult {
//     fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//         request_or_group_id: &str,
//         request_or_group_title: &str,
//         request_or_group_tag: &Option<String>,
//     ) -> Vec<usize> {
//         let success = success_from_tallies(&self);

//         let mut add_grouped = || {
//             let index = list.len();
//             list.push((
//                 ExecutionResultSummary {
//                     request_or_group_id: self.id.to_string(),
//                     index,
//                     parent_index,
//                     child_indexes: Some(vec![]),
//                     level,
//                     name: self.name.clone(),
//                     tag: self.tag.clone(),
//                     method: None,
//                     url: None,
//                     executed_at: self.executed_at,
//                     duration: self.duration,
//                     status: None,
//                     status_text: None,
//                     has_response_headers: false,
//                     response_body_length: None,
//                     success: success.clone(),
//                     error: None,
//                     test_results: None,
//                     run_number: None,
//                     run_count: None,
//                     row_number: None,
//                     row_count: None,
//                 },
//                 ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
//                     id: self.id.to_string(),
//                     name: self.name.clone(),
//                     tag: self.tag.clone(),
//                     row_number: None,
//                     run_number: None,
//                     executed_at: self.executed_at,
//                     duration: self.duration,
//                     data_context: self.data_context.clone(),
//                     success: success.clone(),
//                     request_success_count: self.request_success_count,
//                     request_failure_count: self.request_failure_count,
//                     request_error_count: self.request_error_count,
//                     test_pass_count: self.test_pass_count,
//                     test_fail_count: self.test_fail_count,
//                 })),
//             ));
//             index
//         };

//         match self.content {
//             ApicizeRequestResultContent::Rows { rows } => {
//                 let index = add_grouped();
//                 let child_indexes = rows.append_to_list(
//                     list,
//                     level + 1,
//                     Some(index),
//                     request_or_group_id,
//                     request_or_group_title,
//                     request_or_group_tag,
//                 );
//                 if !child_indexes.is_empty() {
//                     list.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
//                 }
//                 vec![index]
//             }
//             ApicizeRequestResultContent::Runs { runs } => {
//                 let index = add_grouped();
//                 let child_indexes = runs.append_to_list(
//                     list,
//                     level + 1,
//                     Some(index),
//                     request_or_group_id,
//                     request_or_group_title,
//                     request_or_group_tag,
//                 );
//                 if !child_indexes.is_empty() {
//                     list.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
//                 }
//                 vec![index]
//             }
//             ApicizeRequestResultContent::Execution { execution } => {
//                 let (status, status_text, has_response_headers, response_body_length) =
//                     get_response_info(&execution);
//                 let index = list.len();
//                 list.push((
//                     ExecutionResultSummary {
//                         request_or_group_id: request_or_group_id.to_string(),
//                         index,
//                         parent_index,
//                         child_indexes: None,
//                         level,
//                         name: request_or_group_title.to_string(),
//                         method: execution.method.clone(),
//                         url: execution.url.clone(),
//                         tag: self.tag.clone(),
//                         executed_at: self.executed_at,
//                         duration: self.duration,
//                         status,
//                         status_text,
//                         has_response_headers,
//                         response_body_length,
//                         success: success.clone(),
//                         error: execution.error.clone(),
//                         test_results: execution.tests.clone(),
//                         run_number: None,
//                         run_count: None,
//                         row_number: None,
//                         row_count: None,
//                     },
//                     ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
//                         id: request_or_group_id.to_string(),
//                         name: request_or_group_title.to_string(),
//                         method: execution.method.clone(),
//                         url: execution.url.clone(),
//                         tag: self.tag.clone(),
//                         row_number: None,
//                         run_number: None,
//                         executed_at: self.executed_at,
//                         duration: self.duration,
//                         test_context: execution.test_context,
//                         output_variables: execution.output_variables,
//                         tests: execution.tests,
//                         error: execution.error,
//                         success,
//                         request_success_count: self.request_success_count,
//                         request_failure_count: self.request_failure_count,
//                         request_error_count: self.request_error_count,
//                         test_pass_count: self.test_pass_count,
//                         test_fail_count: self.test_fail_count,
//                     })),
//                 ));
//                 vec![index]
//             }
//         }
//     }
// }

// impl ListAppendable for Vec<ApicizeRequestResultRun> {
//     fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//         request_or_group_id: &str,
//         request_or_group_title: &str,
//         request_or_group_tag: &Option<String>,
//     ) -> Vec<usize> {
//         let mut run_number = 1;
//         let run_count = self.len();
//         let mut indexes = Vec::<usize>::with_capacity(list.len());

//         for run in self {
//             let success = success_from_tallies(&run);
//             let name = format!(
//                 "{request_or_group_title} (Run {run_number} of {run_count})"
//             );

//             let (status, status_text, has_response_headers, response_body_length) =
//                 get_response_info(&run.execution);

//             let index = list.len();
//             list.push((
//                 ExecutionResultSummary {
//                     request_or_group_id: request_or_group_id.to_string(),
//                     index,
//                     parent_index,
//                     child_indexes: None,
//                     level,
//                     name: name.clone(),
//                     tag: request_or_group_tag.clone(),
//                     method: run.execution.method.clone(),
//                     url: run.execution.url.clone(),
//                     executed_at: run.executed_at,
//                     duration: run.duration,
//                     status,
//                     status_text,
//                     has_response_headers,
//                     response_body_length,
//                     success: success.clone(),
//                     error: run.execution.error.clone(),
//                     test_results: run.execution.tests.clone(),
//                     run_number: Some(run_number),
//                     run_count: Some(run_count),
//                     row_number: None,
//                     row_count: None,
//                 },
//                 ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
//                     id: request_or_group_id.to_string(),
//                     name,
//                     tag: request_or_group_tag.clone(),
//                     method: run.execution.method.clone(),
//                     url: run.execution.url.clone(),
//                     row_number: None,
//                     run_number: Some(run_number),
//                     executed_at: run.executed_at,
//                     duration: run.duration,
//                     test_context: run.execution.test_context,
//                     output_variables: run.execution.output_variables,
//                     tests: run.execution.tests,
//                     error: run.execution.error,
//                     success,
//                     request_success_count: run.request_success_count,
//                     request_failure_count: run.request_failure_count,
//                     request_error_count: run.request_error_count,
//                     test_pass_count: run.test_pass_count,
//                     test_fail_count: run.test_fail_count,
//                 })),
//             ));

//             indexes.push(index);

//             run_number += 1;
//         }

//         indexes
//     }
// }

// impl ListAppendable for Vec<ApicizeRequestResultRow> {
//     fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//         request_or_group_id: &str,
//         request_or_group_title: &str,
//         request_or_group_tag: &Option<String>,
//     ) -> Vec<usize> {
//         let mut row_number = 1;
//         let row_count = self.len();
//         let mut indexes = Vec::<usize>::with_capacity(list.len());

//         for row in self {
//             let success = success_from_tallies(&row);
//             let index = list.len();
//             let name = format!(
//                 "{request_or_group_title} (Row {row_number} of {row_count})"
//             );

//             match row.results {
//                 ApicizeRequestResultRowContent::Runs(runs) => {
//                     list.push((
//                         ExecutionResultSummary {
//                             request_or_group_id: request_or_group_id.to_string(),
//                             index,
//                             parent_index,
//                             child_indexes: Some(vec![]),
//                             level,
//                             name: name.clone(),
//                             method: None,
//                             url: None,
//                             tag: request_or_group_tag.clone(),
//                             executed_at: row.executed_at,
//                             duration: row.duration,
//                             status: None,
//                             status_text: None,
//                             has_response_headers: false,
//                             response_body_length: None,
//                             success: success.clone(),
//                             error: None,
//                             test_results: None,
//                             run_number: None,
//                             run_count: None,
//                             row_number: Some(row_number),
//                             row_count: Some(row_count),
//                         },
//                         ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
//                             id: request_or_group_id.to_string(),
//                             name,
//                             tag: request_or_group_tag.clone(),
//                             row_number: Some(row_number),
//                             run_number: None,
//                             executed_at: row.executed_at,
//                             duration: row.duration,
//                             data_context: row.data_context,
//                             success,
//                             request_success_count: row.request_success_count,
//                             request_failure_count: row.request_failure_count,
//                             request_error_count: row.request_error_count,
//                             test_pass_count: row.test_pass_count,
//                             test_fail_count: row.test_fail_count,
//                         })),
//                     ));

//                     let child_indexes = runs.append_to_list(
//                         list,
//                         level + 1,
//                         Some(index),
//                         request_or_group_id,
//                         request_or_group_title,
//                         request_or_group_tag,
//                     );

//                     if !child_indexes.is_empty() {
//                         list.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
//                     }
//                 }
//                 ApicizeRequestResultRowContent::Execution(execution) => {
//                     let (status, status_text, has_response_headers, response_body_length) =
//                         get_response_info(&execution);

//                     let index = list.len();
//                     list.push((
//                         ExecutionResultSummary {
//                             request_or_group_id: request_or_group_id.to_string(),
//                             index,
//                             parent_index,
//                             child_indexes: None,
//                             level,
//                             name: name.clone(),
//                             tag: request_or_group_tag.clone(),
//                             method: execution.method.clone(),
//                             url: execution.url.clone(),
//                             executed_at: row.executed_at,
//                             duration: row.duration,
//                             status,
//                             status_text,
//                             has_response_headers,
//                             response_body_length,
//                             success: success.clone(),
//                             error: execution.error.clone(),
//                             test_results: execution.tests.clone(),
//                             run_number: None,
//                             run_count: None,
//                             row_number: Some(row_number),
//                             row_count: Some(row_count),
//                         },
//                         ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
//                             id: request_or_group_id.to_string(),
//                             name,
//                             tag: request_or_group_tag.clone(),
//                             method: execution.method.clone(),
//                             url: execution.url.clone(),
//                             row_number: Some(row_number),
//                             run_number: None,
//                             executed_at: row.executed_at,
//                             duration: row.duration,
//                             test_context: execution.test_context,
//                             output_variables: execution.output_variables,
//                             tests: execution.tests,
//                             error: execution.error,
//                             success,
//                             request_success_count: row.request_success_count,
//                             request_failure_count: row.request_failure_count,
//                             request_error_count: row.request_error_count,
//                             test_pass_count: row.test_pass_count,
//                             test_fail_count: row.test_fail_count,
//                         })),
//                     ));
//                 }
//             }

//             indexes.push(index);

//             row_number += 1;
//         }

//         indexes
//     }
// }

// impl ListAppendable for ApicizeGroupResult {
//     fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//         request_or_group_id: &str,
//         request_or_group_title: &str,
//         request_or_group_tag: &Option<String>,
//     ) -> Vec<usize> {
//         let success = success_from_tallies(&self);
//         let mut indexes = Vec::<usize>::with_capacity(list.len());

//         let mut append_grouped = || {
//             let index = list.len();
//             list.push((
//                 ExecutionResultSummary {
//                     request_or_group_id: self.id.to_string(),
//                     index,
//                     parent_index,
//                     child_indexes: Some(vec![]),
//                     level,
//                     name: self.name.clone(),
//                     tag: self.tag.clone(),
//                     method: None,
//                     url: None,
//                     executed_at: self.executed_at,
//                     duration: self.duration,
//                     status: None,
//                     status_text: None,
//                     has_response_headers: false,
//                     response_body_length: None,
//                     success: success.clone(),
//                     error: None,
//                     test_results: None,
//                     run_number: None,
//                     run_count: None,
//                     row_number: None,
//                     row_count: None,
//                 },
//                 ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
//                     id: self.id.to_string(),
//                     name: self.name.clone(),
//                     tag: self.tag.clone(),
//                     row_number: None,
//                     run_number: None,
//                     executed_at: self.executed_at,
//                     duration: self.duration,
//                     data_context: self.data_context.clone(),
//                     success: success.clone(),
//                     request_success_count: self.request_success_count,
//                     request_failure_count: self.request_failure_count,
//                     request_error_count: self.request_error_count,
//                     test_pass_count: self.test_pass_count,
//                     test_fail_count: self.test_fail_count,
//                 })),
//             ));

//             indexes.push(index);
//             index
//         };

//         let parent_index = append_grouped();
//         let mut child_indexes: Vec<usize>;
//         match self.content {
//             ApicizeGroupResultContent::Rows { rows } => {
//                 child_indexes = rows.append_to_list(
//                     list,
//                     level + 1,
//                     Some(parent_index),
//                     request_or_group_id,
//                     request_or_group_title,
//                     request_or_group_tag,
//                 );
//             }
//             ApicizeGroupResultContent::Runs { runs } => {
//                 child_indexes = runs.append_to_list(
//                     list,
//                     level + 1,
//                     Some(parent_index),
//                     request_or_group_id,
//                     request_or_group_title,
//                     request_or_group_tag,
//                 );
//             }
//             ApicizeGroupResultContent::Results { results } => {
//                 child_indexes = vec![];
//                 for result in results {
//                     child_indexes.extend(result.append_to_list(
//                         list,
//                         level + 1,
//                         Some(parent_index),
//                     ));
//                 }
//             }
//         }

//         if !child_indexes.is_empty() {
//             list.get_mut(parent_index).unwrap().0.child_indexes = Some(child_indexes);
//         }

//         indexes
//     }
// }

// impl ListAppendable for Vec<ApicizeGroupResultRun> {
//     fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//         request_or_group_id: &str,
//         request_or_group_title: &str,
//         request_or_group_tag: &Option<String>,
//     ) -> Vec<usize> {
//         let mut run_number = 1;
//         let run_count = self.len();
//         let mut indexes = Vec::<usize>::with_capacity(list.len());

//         for run in self {
//             let success = success_from_tallies(&run);
//             let name = format!(
//                 "{request_or_group_title} (Run {run_number} of {run_count})"
//             );

//             let index = list.len();
//             list.push((
//                 ExecutionResultSummary {
//                     request_or_group_id: request_or_group_id.to_string(),
//                     index,
//                     parent_index,
//                     child_indexes: None,
//                     level,
//                     name: name.clone(),
//                     tag: request_or_group_tag.clone(),
//                     method: None,
//                     url: None,
//                     executed_at: run.executed_at,
//                     duration: run.duration,
//                     status: None,
//                     status_text: None,
//                     has_response_headers: false,
//                     response_body_length: None,
//                     success: success.clone(),
//                     error: None,
//                     test_results: None,
//                     run_number: Some(run_number),
//                     run_count: Some(run_count),
//                     row_number: None,
//                     row_count: None,
//                 },
//                 ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
//                     id: request_or_group_id.to_string(),
//                     name,
//                     tag: request_or_group_tag.clone(),
//                     row_number: None,
//                     run_number: Some(run_number),
//                     executed_at: run.executed_at,
//                     duration: run.duration,
//                     data_context: run.data_context,
//                     success,
//                     request_success_count: run.request_success_count,
//                     request_failure_count: run.request_failure_count,
//                     request_error_count: run.request_error_count,
//                     test_pass_count: run.test_pass_count,
//                     test_fail_count: run.test_fail_count,
//                 })),
//             ));

//             let mut child_indexes = vec![];
//             for result in run.results {
//                 child_indexes.extend(result.append_to_list(list, level + 1, Some(index)));
//             }

//             if !child_indexes.is_empty() {
//                 list.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
//             }

//             indexes.push(index);

//             run_number += 1;
//         }

//         indexes
//     }
// }

// impl ListAppendable for Vec<ApicizeGroupResultRow> {
//     fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//         request_or_group_id: &str,
//         request_or_group_title: &str,
//         request_or_group_tag: &Option<String>,
//     ) -> Vec<usize> {
//         let mut row_number = 1;
//         let row_count = self.len();
//         let mut indexes = Vec::<usize>::with_capacity(list.len());

//         for row in self {
//             let success = success_from_tallies(&row);
//             let name = format!(
//                 "{request_or_group_title} (Row {row_number} of {row_count})"
//             );

//             let index = list.len();
//             list.push((
//                 ExecutionResultSummary {
//                     request_or_group_id: request_or_group_id.to_string(),
//                     index,
//                     parent_index,
//                     child_indexes: None,
//                     level,
//                     name: name.clone(),
//                     tag: request_or_group_tag.clone(),
//                     method: None,
//                     url: None,
//                     executed_at: row.executed_at,
//                     duration: row.duration,
//                     status: None,
//                     status_text: None,
//                     has_response_headers: false,
//                     response_body_length: None,
//                     success: success.clone(),
//                     error: None,
//                     test_results: None,
//                     run_number: None,
//                     run_count: None,
//                     row_number: Some(row_number),
//                     row_count: Some(row_count),
//                 },
//                 ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
//                     id: request_or_group_id.to_string(),
//                     name,
//                     tag: request_or_group_tag.clone(),
//                     row_number: None,
//                     run_number: Some(row_number),
//                     executed_at: row.executed_at,
//                     duration: row.duration,
//                     data_context: row.data_context.clone(),
//                     success,
//                     request_success_count: row.request_success_count,
//                     request_failure_count: row.request_failure_count,
//                     request_error_count: row.request_error_count,
//                     test_pass_count: row.test_pass_count,
//                     test_fail_count: row.test_fail_count,
//                 })),
//             ));

//             let mut child_indexes: Vec<usize>;
//             match row.content {
//                 ApicizeGroupResultRowContent::Runs { runs } => {
//                     child_indexes = runs.append_to_list(
//                         list,
//                         level + 1,
//                         Some(index),
//                         request_or_group_id,
//                         request_or_group_title,
//                         request_or_group_tag,
//                     );
//                 }
//                 ApicizeGroupResultRowContent::Results { results } => {
//                     child_indexes = vec![];
//                     for result in results {
//                         child_indexes.extend(result.append_to_list(list, level + 1, Some(index)));
//                     }
//                 }
//             }

//             if !child_indexes.is_empty() {
//                 list.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
//             }

//             indexes.push(index);

//             row_number += 1;
//         }

//         indexes
//     }
// }

// impl ApicizeResult {
//     pub fn append_to_list(
//         self,
//         list: &mut Vec<ExecutionResult>,
//         level: usize,
//         parent_index: Option<usize>,
//     ) -> Vec<usize> {
//         let request_or_group_id = self.get_id().to_string();
//         let request_or_group_title = self.get_title();
//         let request_or_group_tag = match &self {
//             ApicizeResult::Request(request) => request.tag.clone(),
//             ApicizeResult::Group(group) => group.tag.clone(),
//         };
//         match self {
//             ApicizeResult::Request(request) => request.append_to_list(
//                 list,
//                 level,
//                 parent_index,
//                 &request_or_group_id,
//                 &request_or_group_title,
//                 &request_or_group_tag,
//             ),
//             ApicizeResult::Group(group) => group.append_to_list(
//                 list,
//                 level,
//                 parent_index,
//                 &request_or_group_id,
//                 &request_or_group_title,
//                 &request_or_group_tag,
//             ),
//         }
//     }
// }

pub struct ExecutionResultBuilder<'a> {
    context: &'a TestRunnerContext,
    results: Vec<ExecutionResult>,
}

impl ExecutionResultBuilder<'_> {

    pub fn new(context: &TestRunnerContext) -> ExecutionResultBuilder {
        ExecutionResultBuilder { context, results: Vec::new() }
    }

    pub fn get_results(self) -> (Vec<ExecutionResultSummary>, Vec<ExecutionResultDetail>) {
        self.results.into_iter().unzip()
    }

    pub fn assemble(&mut self, result: ApicizeResult) {
        self.append_result(result, 0, None);
    }

    fn append_result(
        &mut self,
        result: ApicizeResult,
        level: usize,
        parent_index: Option<usize>,
    ) -> Vec<usize> {
        match result {
            ApicizeResult::Request(request) => {
                self.append_request_result(*request, level, parent_index)
            }
            ApicizeResult::Group(group) => self.append_group_result(*group, level, parent_index),
        }
    }

    fn append_request_result(
        &mut self,
        result: ApicizeRequestResult,
        level: usize,
        parent_index: Option<usize>,
    ) -> Vec<usize> {
        let success = success_from_tallies(&result);

        let request_id = result.get_id().to_string();
        let key = self
            .context
            .get_request_key(&request_id)
            .unwrap_or_default();
        let identifiers = EntryIdentifiers {
            id: request_id,
            title: result.get_title(),
            key,
            tag: result.tag,
        };

        let mut add_grouped = || {
            let index = self.results.len();
            self.results.push((
                ExecutionResultSummary {
                    request_or_group_id: identifiers.id.clone(),
                    index,
                    parent_index,
                    child_indexes: Some(vec![]),
                    level,
                    name: identifiers.title.clone(),
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    method: None,
                    url: None,
                    executed_at: result.executed_at,
                    duration: result.duration,
                    status: None,
                    status_text: None,
                    has_response_headers: false,
                    response_body_length: None,
                    success: success.clone(),
                    error: None,
                    test_results: None,
                    run_number: None,
                    run_count: None,
                    row_number: None,
                    row_count: None,
                },
                ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                    id: identifiers.id.clone(),
                    name: identifiers.title.clone(),
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    row_number: None,
                    run_number: None,
                    executed_at: result.executed_at,
                    duration: result.duration,
                    data_context: result.data_context.clone(),
                    success: success.clone(),
                    request_success_count: result.request_success_count,
                    request_failure_count: result.request_failure_count,
                    request_error_count: result.request_error_count,
                    test_pass_count: result.test_pass_count,
                    test_fail_count: result.test_fail_count,
                })),
            ));
            index
        };

        match result.content {
            ApicizeRequestResultContent::Rows { rows } => {
                let index = add_grouped();
                let child_indexes =
                    self.append_request_result_rows(rows, level + 1, Some(index), &identifiers);
                if !child_indexes.is_empty() {
                    self.results.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
                }
                vec![index]
            }
            ApicizeRequestResultContent::Runs { runs } => {
                let index = add_grouped();
                let child_indexes =
                    self.append_request_result_runs(runs, level + 1, Some(index), &identifiers);
                if !child_indexes.is_empty() {
                    self.results.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
                }
                vec![index]
            }
            ApicizeRequestResultContent::Execution { execution } => {
                let (status, status_text, has_response_headers, response_body_length) =
                    get_response_info(&execution);
                let index = self.results.len();
                self.results.push((
                    ExecutionResultSummary {
                        request_or_group_id: identifiers.id.to_string(),
                        index,
                        parent_index,
                        child_indexes: None,
                        level,
                        name: identifiers.title.clone(),
                        method: execution.method.clone(),
                        url: execution.url.clone(),
                        key: identifiers.key.clone(),
                        tag: identifiers.tag.clone(),
                        executed_at: result.executed_at,
                        duration: result.duration,
                        status,
                        status_text,
                        has_response_headers,
                        response_body_length,
                        success: success.clone(),
                        error: execution.error.clone(),
                        test_results: execution.tests.clone(),
                        run_number: None,
                        run_count: None,
                        row_number: None,
                        row_count: None,
                    },
                    ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
                        id: identifiers.id.to_string(),
                        name: identifiers.title.clone(),
                        method: execution.method.clone(),
                        url: execution.url.clone(),
                        key: identifiers.key.clone(),
                        tag: identifiers.tag.clone(),
                        row_number: None,
                        run_number: None,
                        executed_at: result.executed_at,
                        duration: result.duration,
                        test_context: execution.test_context,
                        output_variables: execution.output_variables,
                        tests: execution.tests,
                        error: execution.error,
                        success,
                        request_success_count: result.request_success_count,
                        request_failure_count: result.request_failure_count,
                        request_error_count: result.request_error_count,
                        test_pass_count: result.test_pass_count,
                        test_fail_count: result.test_fail_count,
                    })),
                ));
                vec![index]
            }
        }
    }

    fn append_group_result(
        &mut self,
        result: ApicizeGroupResult,
        level: usize,
        parent_index: Option<usize>,
    ) -> Vec<usize> {
        let success = success_from_tallies(&result);

        let group_id = result.get_id().to_string();
        let key = self.context.get_request_key(&group_id).unwrap_or_default();
        let identifiers = EntryIdentifiers {
            id: group_id,
            title: result.get_title(),
            key,
            tag: result.tag,
        };

        let index = self.results.len();
        self.results.push((
            ExecutionResultSummary {
                request_or_group_id: identifiers.id.clone(),
                index,
                parent_index,
                child_indexes: Some(vec![]),
                level,
                name: identifiers.title.clone(),
                tag: identifiers.tag.clone(),
                key: identifiers.key.clone(),
                method: None,
                url: None,
                executed_at: result.executed_at,
                duration: result.duration,
                status: None,
                status_text: None,
                has_response_headers: false,
                response_body_length: None,
                success: success.clone(),
                error: None,
                test_results: None,
                run_number: None,
                run_count: None,
                row_number: None,
                row_count: None,
            },
            ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                id: identifiers.id.clone(),
                name: identifiers.title.clone(),
                key: identifiers.key.clone(),
                tag: identifiers.tag.clone(),
                row_number: None,
                run_number: None,
                executed_at: result.executed_at,
                duration: result.duration,
                data_context: result.data_context.clone(),
                success: success.clone(),
                request_success_count: result.request_success_count,
                request_failure_count: result.request_failure_count,
                request_error_count: result.request_error_count,
                test_pass_count: result.test_pass_count,
                test_fail_count: result.test_fail_count,
            })),
        ));

        let mut child_indexes: Vec<usize>;
        match result.content {
            ApicizeGroupResultContent::Rows { rows } => {
                child_indexes =
                    self.append_group_result_rows(rows, level + 1, Some(index), &identifiers);
            }
            ApicizeGroupResultContent::Runs { runs } => {
                child_indexes =
                    self.append_group_result_runs(runs, level + 1, Some(index), &identifiers);
            }
            ApicizeGroupResultContent::Results { results } => {
                child_indexes = vec![];
                for result in results {
                    child_indexes.extend(self.append_result(result, level + 1, Some(index)));
                }
            }
        }

        if !child_indexes.is_empty() {
            self.results.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
        }

        vec![index]
    }

    fn append_request_result_rows(
        &mut self,
        rows: Vec<ApicizeRequestResultRow>,
        level: usize,
        parent_index: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let mut row_number = 1;
        let row_count = rows.len();
        let mut indexes = Vec::<usize>::with_capacity(row_count);

        for row in rows {
            let success = success_from_tallies(&row);
            let index = self.results.len();
            let name = format!(
                "{} (Row {} of {})",
                identifiers.title, row_number, row_count,
            );

            match row.results {
                ApicizeRequestResultRowContent::Runs(runs) => {
                    self.results.push((
                        ExecutionResultSummary {
                            request_or_group_id: identifiers.id.clone(),
                            index,
                            parent_index,
                            child_indexes: Some(vec![]),
                            level,
                            name: name.clone(),
                            method: None,
                            url: None,
                            key: identifiers.key.clone(),
                            tag: identifiers.tag.clone(),
                            executed_at: row.executed_at,
                            duration: row.duration,
                            status: None,
                            status_text: None,
                            has_response_headers: false,
                            response_body_length: None,
                            success: success.clone(),
                            error: None,
                            test_results: None,
                            run_number: None,
                            run_count: None,
                            row_number: Some(row_number),
                            row_count: Some(row_count),
                        },
                        ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                            id: identifiers.id.clone(),
                            name,
                            key: identifiers.key.clone(),
                            tag: identifiers.tag.clone(),
                            row_number: Some(row_number),
                            run_number: None,
                            executed_at: row.executed_at,
                            duration: row.duration,
                            data_context: row.data_context,
                            success,
                            request_success_count: row.request_success_count,
                            request_failure_count: row.request_failure_count,
                            request_error_count: row.request_error_count,
                            test_pass_count: row.test_pass_count,
                            test_fail_count: row.test_fail_count,
                        })),
                    ));

                    let child_indexes =
                        self.append_request_result_runs(runs, level + 1, Some(index), identifiers);

                    if !child_indexes.is_empty() {
                        self.results.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
                    }
                }
                ApicizeRequestResultRowContent::Execution(execution) => {
                    let (status, status_text, has_response_headers, response_body_length) =
                        get_response_info(&execution);

                    let index = self.results.len();
                    self.results.push((
                        ExecutionResultSummary {
                            request_or_group_id: identifiers.id.to_string(),
                            index,
                            parent_index,
                            child_indexes: None,
                            level,
                            name: name.clone(),
                            key: identifiers.key.clone(),
                            tag: identifiers.tag.clone(),
                            method: execution.method.clone(),
                            url: execution.url.clone(),
                            executed_at: row.executed_at,
                            duration: row.duration,
                            status,
                            status_text,
                            has_response_headers,
                            response_body_length,
                            success: success.clone(),
                            error: execution.error.clone(),
                            test_results: execution.tests.clone(),
                            run_number: None,
                            run_count: None,
                            row_number: Some(row_number),
                            row_count: Some(row_count),
                        },
                        ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
                            id: identifiers.id.to_string(),
                            name,
                            key: identifiers.key.clone(),
                            tag: identifiers.tag.clone(),
                            method: execution.method.clone(),
                            url: execution.url.clone(),
                            row_number: Some(row_number),
                            run_number: None,
                            executed_at: row.executed_at,
                            duration: row.duration,
                            test_context: execution.test_context,
                            output_variables: execution.output_variables,
                            tests: execution.tests,
                            error: execution.error,
                            success,
                            request_success_count: row.request_success_count,
                            request_failure_count: row.request_failure_count,
                            request_error_count: row.request_error_count,
                            test_pass_count: row.test_pass_count,
                            test_fail_count: row.test_fail_count,
                        })),
                    ));
                }
            }

            indexes.push(index);

            row_number += 1;
        }

        indexes
    }

    fn append_request_result_runs(
        &mut self,
        runs: Vec<ApicizeRequestResultRun>,
        level: usize,
        parent_index: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let _ = runs;
        let mut run_number = 1;
        let run_count = runs.len();
        let mut indexes = Vec::<usize>::with_capacity(run_count);

        for run in runs {
            let success = success_from_tallies(&run);
            let name = format!(
                "{} (Run {} of {})",
                identifiers.title, run_number, run_count,
            );

            let (status, status_text, has_response_headers, response_body_length) =
                get_response_info(&run.execution);

            let index = self.results.len();
            self.results.push((
                ExecutionResultSummary {
                    request_or_group_id: identifiers.id.to_string(),
                    index,
                    parent_index,
                    child_indexes: None,
                    level,
                    name: name.clone(),
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    method: run.execution.method.clone(),
                    url: run.execution.url.clone(),
                    executed_at: run.executed_at,
                    duration: run.duration,
                    status,
                    status_text,
                    has_response_headers,
                    response_body_length,
                    success: success.clone(),
                    error: run.execution.error.clone(),
                    test_results: run.execution.tests.clone(),
                    run_number: Some(run_number),
                    run_count: Some(run_count),
                    row_number: None,
                    row_count: None,
                },
                ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
                    id: identifiers.id.to_string(),
                    name,
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    method: run.execution.method.clone(),
                    url: run.execution.url.clone(),
                    row_number: None,
                    run_number: Some(run_number),
                    executed_at: run.executed_at,
                    duration: run.duration,
                    test_context: run.execution.test_context,
                    output_variables: run.execution.output_variables,
                    tests: run.execution.tests,
                    error: run.execution.error,
                    success,
                    request_success_count: run.request_success_count,
                    request_failure_count: run.request_failure_count,
                    request_error_count: run.request_error_count,
                    test_pass_count: run.test_pass_count,
                    test_fail_count: run.test_fail_count,
                })),
            ));

            indexes.push(index);
            run_number += 1;
        }

        indexes
    }

    fn append_group_result_rows(
        &mut self,
        rows: Vec<ApicizeGroupResultRow>,
        level: usize,
        parent_index: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let mut row_number = 1;
        let row_count = rows.len();
        let mut indexes = Vec::<usize>::with_capacity(row_count);

        for row in rows {
            let success = success_from_tallies(&row);
            let name = format!(
                "{} (Row {} of {})",
                identifiers.title, row_number, row_count,
            );

            let index = self.results.len();
            self.results.push((
                ExecutionResultSummary {
                    request_or_group_id: identifiers.id.clone(),
                    index,
                    parent_index,
                    child_indexes: None,
                    level,
                    name: name.clone(),
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    method: None,
                    url: None,
                    executed_at: row.executed_at,
                    duration: row.duration,
                    status: None,
                    status_text: None,
                    has_response_headers: false,
                    response_body_length: None,
                    success: success.clone(),
                    error: None,
                    test_results: None,
                    run_number: None,
                    run_count: None,
                    row_number: Some(row_number),
                    row_count: Some(row_count),
                },
                ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                    id: identifiers.id.clone(),
                    name,
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    row_number: None,
                    run_number: Some(row_number),
                    executed_at: row.executed_at,
                    duration: row.duration,
                    data_context: row.data_context.clone(),
                    success,
                    request_success_count: row.request_success_count,
                    request_failure_count: row.request_failure_count,
                    request_error_count: row.request_error_count,
                    test_pass_count: row.test_pass_count,
                    test_fail_count: row.test_fail_count,
                })),
            ));

            let mut child_indexes: Vec<usize>;
            match row.content {
                ApicizeGroupResultRowContent::Runs { runs } => {
                    child_indexes =
                        self.append_group_result_runs(runs, level + 1, Some(index), identifiers);
                }
                ApicizeGroupResultRowContent::Results { results } => {
                    child_indexes = vec![];
                    for result in results {
                        child_indexes.extend(self.append_result(result, level + 1, Some(index)));
                    }
                }
            }

            if !child_indexes.is_empty() {
                self.results.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
            }

            indexes.push(index);

            row_number += 1;
        }

        indexes
    }

    fn append_group_result_runs(
        &mut self,
        runs: Vec<ApicizeGroupResultRun>,
        level: usize,
        parent_index: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let mut run_number = 1;
        let run_count = runs.len();
        let mut indexes = Vec::<usize>::with_capacity(run_count);

        for run in runs {
            let success = success_from_tallies(&run);
            let name = format!(
                "{} (Run {} of {})",
                identifiers.title, run_number, run_count
            );

            let index = self.results.len();
            self.results.push((
                ExecutionResultSummary {
                    request_or_group_id: identifiers.id.to_string(),
                    index,
                    parent_index,
                    child_indexes: None,
                    level,
                    name: name.clone(),
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    method: None,
                    url: None,
                    executed_at: run.executed_at,
                    duration: run.duration,
                    status: None,
                    status_text: None,
                    has_response_headers: false,
                    response_body_length: None,
                    success: success.clone(),
                    error: None,
                    test_results: None,
                    run_number: Some(run_number),
                    run_count: Some(run_count),
                    row_number: None,
                    row_count: None,
                },
                ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                    id: identifiers.id.to_string(),
                    name,
                    key: identifiers.key.clone(),
                    tag: identifiers.tag.clone(),
                    row_number: None,
                    run_number: Some(run_number),
                    executed_at: run.executed_at,
                    duration: run.duration,
                    data_context: run.data_context,
                    success,
                    request_success_count: run.request_success_count,
                    request_failure_count: run.request_failure_count,
                    request_error_count: run.request_error_count,
                    test_pass_count: run.test_pass_count,
                    test_fail_count: run.test_fail_count,
                })),
            ));

            let mut child_indexes = vec![];
            for result in run.results {
                child_indexes.extend(self.append_result(result, level + 1, Some(index)));
            }

            if !child_indexes.is_empty() {
                self.results.get_mut(index).unwrap().0.child_indexes = Some(child_indexes);
            }

            indexes.push(index);

            run_number += 1;
        }

        indexes
    }
}

struct EntryIdentifiers {
    pub id: String,
    pub title: String,
    pub key: Option<String>,
    pub tag: Option<String>,
}


impl ApicizeResult {
    pub fn assemble_results(self, context: &TestRunnerContext) -> (Vec<ExecutionResultSummary>, Vec<ExecutionResultDetail>) {
        let mut builder = ExecutionResultBuilder {
            context,
            results: Vec::new()
        };
        builder.assemble(self);
        builder.get_results()
    }
}

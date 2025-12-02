use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use super::{
    ExecutionResultDetail, ExecutionResultDetailGroup, ExecutionResultSuccess,
    ExecutionResultSummary,
};
use crate::{
    ApicizeBody, ApicizeError, ApicizeExecution, ApicizeGroupResult, ApicizeGroupResultContent,
    ApicizeGroupResultRow, ApicizeGroupResultRowContent, ApicizeGroupResultRun,
    ApicizeRequestResult, ApicizeRequestResultContent, ApicizeRequestResultRow,
    ApicizeRequestResultRowContent, ApicizeRequestResultRun, ApicizeResult,
    ExecutionResultDetailRequest, Identifiable, Tally, TestRunnerContext,
};

pub type ExecutionResult = (ExecutionResultSummary, ExecutionResultDetail);

#[derive(Default)]
pub struct ExecutionResultBuilder {
    /// Next execution identifier
    exec_ctr: usize,
    /// All execution results indexed by execution identifer
    results: HashMap<usize, ExecutionResult>,
    /// Executions indexed first based upon the requests are executed are for, and then based
    /// up which parent request/group executed them
    executing_request_index: HashMap<String, IndexMap<String, Vec<usize>>>,
    /// Track which parents apply to each request
    associated_parent_request_index: HashMap<String, HashSet<String>>,
}
struct EntryIdentifiers {
    pub id: String,
    pub title: String,
    pub key: Option<String>,
    pub tag: Option<String>,
}

impl ExecutionResultBuilder {
    /// Get execution summaries, grouped by executing request
    pub fn get_summaries(
        &self,
        request_or_group_id: &str,
        include_all_results: bool,
    ) -> IndexMap<String, Vec<&ExecutionResultSummary>> {
        if let Some(request_results) = self.executing_request_index.get(request_or_group_id) {
            request_results
                .iter()
                .filter(|(executing_request_id, _)| {
                    include_all_results || executing_request_id.as_str() == request_or_group_id
                })
                .map(|(executing_request_id, exec_ctrs)| {
                    (
                        executing_request_id.to_string(),
                        exec_ctrs
                            .iter()
                            .map(|exec_ctr| &self.results.get(exec_ctr).unwrap().0)
                            .collect(),
                    )
                })
                .collect::<IndexMap<String, Vec<&ExecutionResultSummary>>>()
        } else {
            IndexMap::default()
        }
    }

    /// Helper function to collect child summaries
    fn collect_summaries(
        &self,
        exec_ctr: usize,
        results: &mut IndexMap<usize, ExecutionResultSummary>,
    ) {
        if let Some((summary, ..)) = self.results.get(&exec_ctr) {
            results.insert(summary.exec_ctr, summary.clone());
            if let Some(child_exec_ctrs) = &summary.child_exec_ctrs {
                for child_exec_ctr in child_exec_ctrs {
                    if !results.contains_key(child_exec_ctr) {
                        self.collect_summaries(child_exec_ctr.to_owned(), results);
                    }
                }
            }
        }
    }

    /// Get execution summaries, grouped by executing request
    pub fn get_result_summaries(&self, exec_ctr: usize) -> IndexMap<usize, ExecutionResultSummary> {
        let mut summaries = IndexMap::<usize, ExecutionResultSummary>::new();
        self.collect_summaries(exec_ctr, &mut summaries);
        summaries
    }

    /// Get execution details
    pub fn get_detail(&self, exec_ctr: &usize) -> Result<&ExecutionResultDetail, ApicizeError> {
        match self.results.get(exec_ctr) {
            Some((_, detail)) => Ok(detail),
            None => Err(ApicizeError::InvalidId {
                description: format!("Invalid execution result counter {exec_ctr}"),
            }),
        }
    }

    /// Get execution result summary and detail
    pub fn get_result(
        &self,
        exec_ctr: &usize,
    ) -> Result<(&ExecutionResultSummary, &ExecutionResultDetail), ApicizeError> {
        match self.results.get(exec_ctr) {
            Some((summary, detail)) => Ok((summary, detail)),
            None => Err(ApicizeError::InvalidId {
                description: format!("Invalid execution result counter {exec_ctr}"),
            }),
        }
    }

    /// Return any requests which may have had content updated based upon the specified request being their parent
    pub fn get_associated_requests(&self, associated_request_or_group_id: &str) -> Vec<String> {
        self.associated_parent_request_index
            .iter()
            .filter_map(|(request_or_group_id, associated_request_ids)| {
                if associated_request_ids.contains(associated_request_or_group_id) {
                    Some(request_or_group_id)
                } else {
                    None
                }
            })
            .cloned()
            .collect::<Vec<String>>()
    }

    /// Process the result generated when executing request_or_group_id
    pub fn process_result(&mut self, context: &TestRunnerContext, result: ApicizeResult) {
        self.delete_indexed_request_results(context.get_executing_request_or_group_id());
        self.append_result(context, result, 0, None);
    }

    /// Delete out any stored request index entries for the specified request/group
    pub fn delete_indexed_request_results(&mut self, executing_request_or_group_id: &str) {
        let mut executions_to_clear: HashSet<usize> = HashSet::new();

        // Remove all results for which the specified request executed them
        for request_executions in self.executing_request_index.values_mut() {
            if request_executions.contains_key(executing_request_or_group_id) {
                // If there are results for request children, then remove those executions as well
                executions_to_clear.extend(
                    request_executions
                        .get(executing_request_or_group_id)
                        .unwrap(),
                );
                request_executions.shift_remove(executing_request_or_group_id);
            }
        }

        for parent_requests in self.associated_parent_request_index.values_mut() {
            parent_requests.remove(executing_request_or_group_id);
        }

        // Delete any stored executions identified as being associated with the request/group
        if !executions_to_clear.is_empty() {
            self.results
                .retain(|id, _| !executions_to_clear.contains(id));
        }
    }

    /// Add request index entry, storing which request the execution was returned from
    fn add_index_entries(
        &mut self,
        request_or_group_id: &str,
        executing_request_or_group_id: &str,
        exec_ctr: usize,
    ) {
        let mut append = |id: &str| {
            if let Some(existing_executions) = self.executing_request_index.get_mut(id) {
                if let Some(request_executions) =
                    existing_executions.get_mut(executing_request_or_group_id)
                {
                    // If there are existing executions, check for existing executing request and, if found, append
                    request_executions.push(exec_ctr);
                } else {
                    // Otherwise, we need to insert a new set of executions either at the top (if refering to self) or next to the top (most recent)
                    let index = if request_or_group_id == executing_request_or_group_id
                        || existing_executions.is_empty()
                    {
                        0
                    } else {
                        1
                    };
                    existing_executions.insert_before(
                        index,
                        executing_request_or_group_id.to_string(),
                        vec![exec_ctr],
                    );
                }
            } else {
                // For requests with no existing executions, just create the entry
                self.executing_request_index.insert(
                    id.to_string(),
                    IndexMap::from([(executing_request_or_group_id.to_string(), vec![exec_ctr])]),
                );
            }
        };

        if executing_request_or_group_id != request_or_group_id {
            append(executing_request_or_group_id);
        }
        append(request_or_group_id);

        if let Some(existing_parent_executions) = self
            .associated_parent_request_index
            .get_mut(request_or_group_id)
        {
            existing_parent_executions.insert(executing_request_or_group_id.to_string());
        } else {
            self.associated_parent_request_index.insert(
                request_or_group_id.to_string(),
                HashSet::from([executing_request_or_group_id.to_string()]),
            );
        }

        // println!("appending counter {exec_ctr} from {request_or_group_id} executed from {executing_request_or_group_id} ");
        // for (request_id, executions) in &self.executing_request_index {
        //     println!("   stored request {request_id}");
        //     for (executing_request_id, counters) in executions {
        //         println!(
        //             "      executed {executing_request_id}: {}",
        //             counters
        //                 .iter()
        //                 .map(|c| c.to_string())
        //                 .collect::<Vec<String>>()
        //                 .join(", ")
        //         );
        //     }
        // }
    }

    fn append_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeResult,
        level: usize,
        parent_index: Option<usize>,
    ) -> Vec<usize> {
        match result {
            ApicizeResult::Request(request) => {
                self.append_request_result(context, *request, level, parent_index)
            }
            ApicizeResult::Group(group) => {
                self.append_group_result(context, *group, level, parent_index)
            }
        }
    }

    fn append_request_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeRequestResult,
        level: usize,
        parent_exec_ctr: Option<usize>,
    ) -> Vec<usize> {
        let success = success_from_tallies(&result);

        let request_id = result.get_id().to_string();
        let key = context.get_request_key(&request_id).unwrap_or_default();
        let identifiers = EntryIdentifiers {
            id: request_id,
            title: result.get_title(),
            key,
            tag: result.tag,
        };

        let exec_ctr = self.next_counter();
        let mut summary = ExecutionResultSummary {
            exec_ctr,
            request_or_group_id: identifiers.id.clone(),
            parent_exec_ctr,
            child_exec_ctrs: Some(vec![]),
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
        };

        let detail: ExecutionResultDetail;

        self.add_index_entries(
            &result.id,
            context.get_executing_request_or_group_id(),
            exec_ctr,
        );

        match result.content {
            ApicizeRequestResultContent::Rows { rows } => {
                let child_counters = self.append_request_result_rows(
                    &result.id,
                    context,
                    rows,
                    level + 1,
                    Some(exec_ctr),
                    &identifiers,
                );
                if !child_counters.is_empty() {
                    summary.child_exec_ctrs = Some(child_counters);
                }
                detail = ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                    exec_ctr,
                    group_id: identifiers.id.clone(),
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
                }));
            }
            ApicizeRequestResultContent::Runs { runs } => {
                let child_indexes = self.append_request_result_runs(
                    &result.id,
                    context,
                    runs,
                    level + 1,
                    Some(exec_ctr),
                    &identifiers,
                );
                if !child_indexes.is_empty() {
                    summary.child_exec_ctrs = Some(child_indexes);
                }
                detail = ExecutionResultDetail::Grouped(Box::new(ExecutionResultDetailGroup {
                    exec_ctr,
                    group_id: identifiers.id.clone(),
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
                }));
            }
            ApicizeRequestResultContent::Execution { execution } => {
                let (status, status_text, has_response_headers, response_body_length) =
                    get_response_info(&execution);

                summary.method = execution.method.clone();
                summary.url = execution.url.clone();
                summary.status = status;
                summary.status_text = status_text;
                summary.has_response_headers = has_response_headers;
                summary.response_body_length = response_body_length;
                summary.error = execution.error.clone();
                summary.test_results = execution.tests.clone();

                detail = ExecutionResultDetail::Request(Box::new(ExecutionResultDetailRequest {
                    exec_ctr,
                    request_id: identifiers.id.to_string(),
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
                }));
            }
        }

        self.results.insert(exec_ctr, (summary, detail));
        vec![exec_ctr]
    }

    fn append_group_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeGroupResult,
        level: usize,
        parent_exec_ctr: Option<usize>,
    ) -> Vec<usize> {
        let success = success_from_tallies(&result);

        let group_id = result.get_id().to_string();
        let key = context.get_request_key(&group_id).unwrap_or_default();
        let identifiers = EntryIdentifiers {
            id: group_id,
            title: result.get_title(),
            key,
            tag: result.tag,
        };

        let exec_ctr = self.next_counter();
        self.add_index_entries(
            &result.id,
            context.get_executing_request_or_group_id(),
            exec_ctr,
        );
        self.results.insert(
            exec_ctr,
            (
                ExecutionResultSummary {
                    request_or_group_id: identifiers.id.clone(),
                    exec_ctr,
                    parent_exec_ctr,
                    child_exec_ctrs: Some(vec![]),
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
                    group_id: identifiers.id.clone(),
                    exec_ctr,
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
            ),
        );

        let mut child_indexes: Vec<usize>;
        match result.content {
            ApicizeGroupResultContent::Rows { rows } => {
                child_indexes = self.append_group_result_rows(
                    &result.id,
                    context,
                    rows,
                    level + 1,
                    Some(exec_ctr),
                    &identifiers,
                );
            }
            ApicizeGroupResultContent::Runs { runs } => {
                child_indexes = self.append_group_result_runs(
                    &result.id,
                    context,
                    runs,
                    level + 1,
                    Some(exec_ctr),
                    &identifiers,
                );
            }
            ApicizeGroupResultContent::Results { results } => {
                child_indexes = vec![];
                for result in results {
                    child_indexes.extend(self.append_result(
                        context,
                        result,
                        level + 1,
                        Some(exec_ctr),
                    ));
                }
            }
        }

        if !child_indexes.is_empty() {
            self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs = Some(child_indexes);
        }

        vec![exec_ctr]
    }

    fn append_request_result_rows(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        rows: Vec<ApicizeRequestResultRow>,
        level: usize,
        parent_exec_ctr: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let mut row_number = 1;
        let row_count = rows.len();
        let mut indexes = Vec::<usize>::with_capacity(row_count);

        for row in rows {
            let success = success_from_tallies(&row);
            let index = self.next_counter();
            // println!("Setting result row {} index to {index}", row.row_number);
            let name = format!(
                "{} (Row {} of {})",
                identifiers.title, row_number, row_count,
            );

            match row.results {
                ApicizeRequestResultRowContent::Runs(runs) => {
                    let exec_ctr = self.next_counter();
                    self.add_index_entries(
                        request_or_group_id,
                        context.get_executing_request_or_group_id(),
                        exec_ctr,
                    );
                    self.results.insert(
                        exec_ctr,
                        (
                            ExecutionResultSummary {
                                exec_ctr,
                                request_or_group_id: identifiers.id.clone(),
                                parent_exec_ctr,
                                child_exec_ctrs: Some(vec![]),
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
                                exec_ctr,
                                group_id: identifiers.id.clone(),
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
                        ),
                    );

                    let child_indexes = self.append_request_result_runs(
                        request_or_group_id,
                        context,
                        runs,
                        level + 1,
                        Some(index),
                        identifiers,
                    );

                    if !child_indexes.is_empty() {
                        self.results.get_mut(&index).unwrap().0.child_exec_ctrs =
                            Some(child_indexes);
                    }
                }
                ApicizeRequestResultRowContent::Execution(execution) => {
                    let (status, status_text, has_response_headers, response_body_length) =
                        get_response_info(&execution);

                    let exec_ctr = self.next_counter();
                    self.add_index_entries(
                        request_or_group_id,
                        context.get_executing_request_or_group_id(),
                        exec_ctr,
                    );
                    self.results.insert(
                        exec_ctr,
                        (
                            ExecutionResultSummary {
                                exec_ctr,
                                request_or_group_id: identifiers.id.to_string(),
                                parent_exec_ctr,
                                child_exec_ctrs: None,
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
                            ExecutionResultDetail::Request(Box::new(
                                ExecutionResultDetailRequest {
                                    exec_ctr,
                                    request_id: identifiers.id.to_string(),
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
                                },
                            )),
                        ),
                    );
                }
            }

            indexes.push(index);

            row_number += 1;
        }

        indexes
    }

    fn append_request_result_runs(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        runs: Vec<ApicizeRequestResultRun>,
        level: usize,
        parent_exec_ctr: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let mut run_number = 1;
        let run_count = runs.len();
        let mut exec_ctrs = Vec::<usize>::with_capacity(run_count);

        for run in runs {
            let success = success_from_tallies(&run);
            let name = format!(
                "{} (Run {} of {})",
                identifiers.title, run_number, run_count,
            );

            let (status, status_text, has_response_headers, response_body_length) =
                get_response_info(&run.execution);

            let exec_ctr = self.next_counter();
            self.add_index_entries(
                request_or_group_id,
                context.get_executing_request_or_group_id(),
                exec_ctr,
            );
            self.results.insert(
                exec_ctr,
                (
                    ExecutionResultSummary {
                        exec_ctr,
                        request_or_group_id: identifiers.id.to_string(),
                        parent_exec_ctr,
                        child_exec_ctrs: None,
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
                        exec_ctr,
                        request_id: identifiers.id.to_string(),
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
                ),
            );

            exec_ctrs.push(exec_ctr);
            run_number += 1;
        }

        exec_ctrs
    }

    fn append_group_result_rows(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        rows: Vec<ApicizeGroupResultRow>,
        level: usize,
        parent_exec_ctr: Option<usize>,
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

            let exec_ctr = self.next_counter();
            self.add_index_entries(
                request_or_group_id,
                context.get_executing_request_or_group_id(),
                exec_ctr,
            );
            self.results.insert(
                exec_ctr,
                (
                    ExecutionResultSummary {
                        exec_ctr,
                        request_or_group_id: identifiers.id.clone(),
                        parent_exec_ctr,
                        child_exec_ctrs: None,
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
                        exec_ctr,
                        group_id: identifiers.id.clone(),
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
                ),
            );

            let mut child_exec_ctrs: Vec<usize>;
            match row.content {
                ApicizeGroupResultRowContent::Runs { runs } => {
                    child_exec_ctrs = self.append_group_result_runs(
                        request_or_group_id,
                        context,
                        runs,
                        level + 1,
                        Some(exec_ctr),
                        identifiers,
                    );
                }
                ApicizeGroupResultRowContent::Results { results } => {
                    child_exec_ctrs = vec![];
                    for result in results {
                        child_exec_ctrs.extend(self.append_result(
                            context,
                            result,
                            level + 1,
                            Some(exec_ctr),
                        ));
                    }
                }
            }

            if !child_exec_ctrs.is_empty() {
                self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs = Some(child_exec_ctrs);
            }

            indexes.push(exec_ctr);

            row_number += 1;
        }

        indexes
    }

    fn append_group_result_runs(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        runs: Vec<ApicizeGroupResultRun>,
        level: usize,
        parent_exec_ctr: Option<usize>,
        identifiers: &EntryIdentifiers,
    ) -> Vec<usize> {
        let mut run_number = 1;
        let run_count = runs.len();
        let mut child_exec_ctrs = Vec::<usize>::with_capacity(run_count);

        for run in runs {
            let success = success_from_tallies(&run);
            let name = format!(
                "{} (Run {} of {})",
                identifiers.title, run_number, run_count
            );

            let exec_ctr = self.next_counter();
            self.add_index_entries(
                request_or_group_id,
                context.get_executing_request_or_group_id(),
                exec_ctr,
            );
            self.results.insert(
                exec_ctr,
                (
                    ExecutionResultSummary {
                        exec_ctr,
                        request_or_group_id: identifiers.id.to_string(),
                        parent_exec_ctr,
                        child_exec_ctrs: None,
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
                        exec_ctr,
                        group_id: identifiers.id.to_string(),
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
                ),
            );

            let mut child_indexes = vec![];
            for result in run.results {
                child_indexes.extend(self.append_result(
                    context,
                    result,
                    level + 1,
                    Some(exec_ctr),
                ));
            }

            if !child_indexes.is_empty() {
                self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs = Some(child_indexes);
            }

            child_exec_ctrs.push(exec_ctr);

            run_number += 1;
        }

        child_exec_ctrs
    }

    /// Return next incremented counter
    fn next_counter(&mut self) -> usize {
        self.exec_ctr = self.exec_ctr.wrapping_add(1);
        self.exec_ctr
    }
}

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

/*
#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_result(
        _id: &str,
        request_error_count: usize,
        test_fail_count: usize,
    ) -> impl Tally {
        SimpleTally {
            success: request_error_count == 0 && test_fail_count == 0,
            request_success_count: if request_error_count == 0 && test_fail_count == 0 {
                1
            } else {
                0
            },
            request_failure_count: if test_fail_count > 0 && request_error_count == 0 {
                1
            } else {
                0
            },
            request_error_count,
            test_pass_count: 0,
            test_fail_count,
        }
    }

    struct SimpleTally {
        success: bool,
        request_success_count: usize,
        request_failure_count: usize,
        request_error_count: usize,
        test_pass_count: usize,
        test_fail_count: usize,
    }

    impl Tally for SimpleTally {
        fn get_tallies(&self) -> crate::Tallies {
            crate::Tallies {
                success: self.success,
                request_success_count: self.request_success_count,
                request_failure_count: self.request_failure_count,
                request_error_count: self.request_error_count,
                test_pass_count: self.test_pass_count,
                test_fail_count: self.test_fail_count,
            }
        }
    }

    #[test]
    fn test_next_counter_increments() {
        let mut builder = ExecutionResultBuilder::default();
        assert_eq!(builder.next_counter(), 1);
        assert_eq!(builder.next_counter(), 2);
        assert_eq!(builder.next_counter(), 3);
    }

    #[test]
    fn test_add_index_request_result_creates_new_entry() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_request_result("req1", "req1", 1);

        assert!(builder.index_request_results.contains_key("req1"));
        let entries = builder.index_request_results.get("req1").unwrap();
        assert_eq!(entries.get("req1").unwrap(), &vec![1]);
    }

    #[test]
    fn test_add_index_request_result_appends_to_existing() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_request_result("req1", "req1", 1);
        builder.add_index_request_result("req1", "req1", 2);

        let entries = builder.index_request_results.get("req1").unwrap();
        assert_eq!(entries.get("req1").unwrap(), &vec![1, 2]);
    }

    #[test]
    fn test_add_index_request_result_position_zero_for_new_parent() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_request_result("req1", "child1", 1);
        builder.add_index_request_result("req1", "child2", 2);

        let entries = builder.index_request_results.get("req1").unwrap();
        assert_eq!(entries.keys().next().unwrap(), "child2");
    }

    #[test]
    fn test_add_index_request_result_position_one_when_parent_matches() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_request_result("req1", "req1", 1);
        builder.add_index_request_result("req1", "child1", 2);

        let entries = builder.index_request_results.get("req1").unwrap();
        let keys: Vec<_> = entries.keys().collect();
        // When first entry is "req1", new entries are inserted at position 1 (after req1)
        assert_eq!(keys[0], "req1");
        assert_eq!(keys[1], "child1");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_clear_indexed_request_results() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_request_result("req1", "req1", 1);
        builder.add_index_request_result("req1", "req1", 2);

        assert!(builder.index_request_results.contains_key("req1"));

        builder.clear_indexed_request_results("req1");
        let entries = builder.index_request_results.get("req1");
        assert!(entries.is_none() || entries.unwrap().is_empty());
    }

    #[test]
    fn test_success_from_tallies_error() {
        let result = create_simple_result("req1", 1, 0);
        assert_eq!(success_from_tallies(&result), ExecutionResultSuccess::Error);
    }

    #[test]
    fn test_success_from_tallies_failure() {
        let result = create_simple_result("req1", 0, 1);
        assert_eq!(
            success_from_tallies(&result),
            ExecutionResultSuccess::Failure
        );
    }

    #[test]
    fn test_success_from_tallies_success() {
        let result = create_simple_result("req1", 0, 0);
        assert_eq!(
            success_from_tallies(&result),
            ExecutionResultSuccess::Success
        );
    }
}
 */

use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;

use crate::{
    ApicizeBody, ApicizeError, ApicizeExecution, ApicizeGroupResult, ApicizeGroupResultContent,
    ApicizeGroupResultRow, ApicizeGroupResultRowContent, ApicizeGroupResultRun,
    ApicizeRequestResult, ApicizeRequestResultContent, ApicizeRequestResultRow,
    ApicizeRequestResultRowContent, ApicizeRequestResultRun, ApicizeResult, ExecutionResultDetail,
    ExecutionResultDetailGroup, ExecutionResultDetailRequest, ExecutionResultSuccess,
    ExecutionResultSummary, Identifiable, Tally, TestRunnerContext,
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
}

impl ExecutionResultBuilder {
    pub fn with_exec_ctr(exec_ctr: usize) -> Self {
        ExecutionResultBuilder {
            exec_ctr,
            results: Default::default(),
            executing_request_index: Default::default(),
        }
    }
}

struct EntryIdentifiers {
    pub id: String,
    pub title: String,
    pub key: Option<String>,
    pub tag: Option<String>,
    pub parent_exec_ctr: usize,
}

struct ResultContext<'a> {
    level: usize,
    active_request_ids: &'a IndexSet<String>,
    identifiers: &'a EntryIdentifiers,
    updated_request_ids: &'a mut IndexSet<String>,
}

impl ExecutionResultBuilder {
    /// Process the result generated when executing request_or_group_id,
    /// return the list of request IDs that have updaed executions
    pub fn process_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeResult,
    ) -> IndexSet<String> {
        let mut updated_request_ids = IndexSet::<String>::new();
        self.delete_indexed_request_results(context.get_executing_request_or_group_id());
        self.append_result(
            context,
            result,
            0,
            &IndexSet::new(),
            None,
            &mut updated_request_ids,
        );
        updated_request_ids
    }

    // pub fn dump_current_indexes(&self) {
    //     println!("*** Executed Indexes ***");
    //     for (request_or_group_id, executed_results) in &self.executing_request_index {
    //         println!("Request {request_or_group_id}");
    //         for (executing_request_id, exec_ctrs) in executed_results {
    //             println!(
    //                 "  - Executed by {executing_request_id}: {}",
    //                 exec_ctrs
    //                     .iter()
    //                     .map(|exec_ctr| exec_ctr.to_string())
    //                     .collect::<Vec<String>>()
    //                     .join(", ")
    //             );
    //         }
    //     }
    //     println!("************************");
    // }

    fn append_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeResult,
        level: usize,
        active_request_ids: &IndexSet<String>,
        parent_exec_ctr: Option<usize>,
        updated_request_ids: &mut IndexSet<String>,
    ) -> usize {
        match result {
            ApicizeResult::Request(request) => self.append_request_result(
                context,
                *request,
                level,
                parent_exec_ctr,
                active_request_ids,
                updated_request_ids,
            ),
            ApicizeResult::Group(group) => self.append_group_result(
                context,
                *group,
                level,
                parent_exec_ctr,
                active_request_ids,
                updated_request_ids,
            ),
        }
    }

    fn append_request_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeRequestResult,
        level: usize,
        parent_exec_ctr: Option<usize>,
        active_request_ids: &IndexSet<String>,
        updated_request_ids: &mut IndexSet<String>,
    ) -> usize {
        let success = success_from_tallies(&result);

        let request_id = result.get_id();
        let executing_request_or_group_id = context.get_executing_request_or_group_id();
        let exec_ctr = self.increment_counter();

        // Add the counter to list of requests/groups that we are collecting for
        let mut active_request_ids = active_request_ids.clone();
        active_request_ids.insert(request_id.to_string());
        for active_request_id in &active_request_ids {
            self.add_index_entries(active_request_id, executing_request_or_group_id, exec_ctr);
        }

        // Track that this result's request was updated
        updated_request_ids.insert(request_id.to_string());

        let key = context.get_request_key(request_id).unwrap_or_default();
        let identifiers = EntryIdentifiers {
            id: request_id.to_string(),
            title: result.get_title(),
            key,
            tag: result.tag,
            parent_exec_ctr: exec_ctr,
        };

        let mut result_context = ResultContext {
            level: level + 1,
            active_request_ids: &active_request_ids,
            identifiers: &identifiers,
            updated_request_ids,
        };

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
            logs: result.logs,
            status: None,
            status_text: None,
            has_response_headers: false,
            has_curl: false,
            response_body_length: None,
            success,
            error: None,
            request_success_count: 0,
            request_failure_count: 0,
            request_error_count: 0,
            test_results: None,
            run_number: None,
            run_count: None,
            row_number: None,
            row_count: None,
        };

        let detail: ExecutionResultDetail;

        match result.content {
            ApicizeRequestResultContent::Rows { rows } => {
                let child_exec_ctrs =
                    self.append_request_result_rows(&result.id, context, rows, &mut result_context);
                if !child_exec_ctrs.is_empty() {
                    summary.child_exec_ctrs = Some(child_exec_ctrs);
                }
                summary.request_success_count = result.request_success_count;
                summary.request_failure_count = result.request_failure_count;
                summary.request_error_count = result.request_error_count;
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
                    success,
                    request_success_count: result.request_success_count,
                    request_failure_count: result.request_failure_count,
                    request_error_count: result.request_error_count,
                    test_pass_count: result.test_pass_count,
                    test_fail_count: result.test_fail_count,
                }));
            }
            ApicizeRequestResultContent::Runs { runs } => {
                let child_exec_ctrs = self.append_request_result_runs(
                    &result.id,
                    context,
                    runs,
                    Some(exec_ctr),
                    &mut result_context,
                );
                if !child_exec_ctrs.is_empty() {
                    summary.child_exec_ctrs = Some(child_exec_ctrs);
                }
                summary.request_success_count = result.request_success_count;
                summary.request_failure_count = result.request_failure_count;
                summary.request_error_count = result.request_error_count;
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
                    success,
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
                summary.has_curl = execution.curl.is_some();
                summary.response_body_length = response_body_length;
                summary.error = execution.error.clone();
                summary.test_results = execution.tests.clone();
                summary.request_success_count = result.request_success_count;
                summary.request_failure_count = result.request_failure_count;
                summary.request_error_count = result.request_error_count;

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
                    output_variables: execution
                        .output_variables
                        .as_ref()
                        .map(|arc| (**arc).clone()),
                    tests: execution.tests,
                    curl: execution.curl,
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
        exec_ctr
    }

    fn append_group_result(
        &mut self,
        context: &TestRunnerContext,
        result: ApicizeGroupResult,
        level: usize,
        parent_exec_ctr: Option<usize>,
        active_request_ids: &IndexSet<String>,
        updated_request_ids: &mut IndexSet<String>,
    ) -> usize {
        let success = success_from_tallies(&result);

        let group_id = result.get_id();
        let executing_request_or_group_id = context.get_executing_request_or_group_id();
        let exec_ctr = self.increment_counter();

        let mut active_request_ids = active_request_ids.clone();
        active_request_ids.insert(group_id.to_string());
        for active_request_id in &active_request_ids {
            self.add_index_entries(active_request_id, executing_request_or_group_id, exec_ctr);
        }

        // Track that this result's request was updated
        updated_request_ids.insert(group_id.to_string());

        let key = context.get_request_key(group_id).unwrap_or_default();
        let identifiers = EntryIdentifiers {
            id: group_id.to_string(),
            title: result.get_title(),
            key,
            tag: result.tag,
            parent_exec_ctr: exec_ctr,
        };

        let mut result_context = ResultContext {
            level: level + 1,
            active_request_ids: &active_request_ids,
            identifiers: &identifiers,
            updated_request_ids,
        };

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
                    logs: result.logs,
                    status: None,
                    status_text: None,
                    has_response_headers: false,
                    has_curl: false,
                    response_body_length: None,
                    success,
                    error: None,
                    request_success_count: result.request_success_count,
                    request_failure_count: result.request_failure_count,
                    request_error_count: result.request_error_count,
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
                    success,
                    request_success_count: result.request_success_count,
                    request_failure_count: result.request_failure_count,
                    request_error_count: result.request_error_count,
                    test_pass_count: result.test_pass_count,
                    test_fail_count: result.test_fail_count,
                })),
            ),
        );

        let child_exec_ctrs = match result.content {
            ApicizeGroupResultContent::Rows { rows } => {
                self.append_group_result_rows(&result.id, context, rows, &mut result_context)
            }
            ApicizeGroupResultContent::Runs { runs } => {
                self.append_group_result_runs(&result.id, context, runs, &mut result_context)
            }
            ApicizeGroupResultContent::Results { results } => results
                .into_iter()
                .map(|result| {
                    self.append_result(
                        context,
                        result,
                        level + 1,
                        &active_request_ids,
                        Some(exec_ctr),
                        result_context.updated_request_ids,
                    )
                })
                .collect(),
        };

        if !child_exec_ctrs.is_empty() {
            self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs = Some(child_exec_ctrs);
        }

        exec_ctr
    }

    fn append_request_result_rows(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        rows: Vec<ApicizeRequestResultRow>,
        result_context: &mut ResultContext,
    ) -> Vec<usize> {
        let level = result_context.level;
        let active_request_ids = result_context.active_request_ids;
        let identifiers = result_context.identifiers;
        let row_count = rows.len();
        let mut indexes = Vec::<usize>::with_capacity(row_count);

        let mut active_request_ids = active_request_ids.clone();
        active_request_ids.insert(request_or_group_id.to_string());

        for (row_number, row) in (1..).zip(rows) {
            let success = success_from_tallies(&row);
            // let index = self.next_counter();
            let name = format!(
                "{} (Row {} of {})",
                identifiers.title, row_number, row_count,
            );

            match row.results {
                ApicizeRequestResultRowContent::Runs(runs) => {
                    let exec_ctr = self.increment_counter();
                    indexes.push(exec_ctr);

                    for active_request_id in &active_request_ids {
                        self.add_index_entries(
                            active_request_id,
                            context.get_executing_request_or_group_id(),
                            exec_ctr,
                        );
                    }

                    self.results.insert(
                        exec_ctr,
                        (
                            ExecutionResultSummary {
                                exec_ctr,
                                request_or_group_id: identifiers.id.clone(),
                                parent_exec_ctr: Some(identifiers.parent_exec_ctr),
                                child_exec_ctrs: Some(vec![]),
                                level,
                                name: name.clone(),
                                method: None,
                                url: None,
                                key: identifiers.key.clone(),
                                tag: identifiers.tag.clone(),
                                executed_at: row.executed_at,
                                duration: row.duration,
                                logs: None,
                                status: None,
                                status_text: None,
                                has_response_headers: false,
                                has_curl: false,
                                response_body_length: None,
                                success,
                                error: None,
                                request_success_count: row.request_success_count,
                                request_failure_count: row.request_failure_count,
                                request_error_count: row.request_error_count,
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

                    let mut nested_context = ResultContext {
                        level: level + 1,
                        active_request_ids: &active_request_ids,
                        identifiers,
                        updated_request_ids: result_context.updated_request_ids,
                    };
                    let child_indexes = self.append_request_result_runs(
                        request_or_group_id,
                        context,
                        runs,
                        Some(exec_ctr),
                        &mut nested_context,
                    );

                    if !child_indexes.is_empty() {
                        self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs =
                            Some(child_indexes);
                    }
                }
                ApicizeRequestResultRowContent::Execution(execution) => {
                    let (status, status_text, has_response_headers, response_body_length) =
                        get_response_info(&execution);

                    let exec_ctr = self.increment_counter();
                    indexes.push(exec_ctr);
                    for active_request_id in &active_request_ids {
                        self.add_index_entries(
                            active_request_id,
                            context.get_executing_request_or_group_id(),
                            exec_ctr,
                        );
                    }
                    self.results.insert(
                        exec_ctr,
                        (
                            ExecutionResultSummary {
                                exec_ctr,
                                request_or_group_id: identifiers.id.to_string(),
                                parent_exec_ctr: Some(identifiers.parent_exec_ctr),
                                child_exec_ctrs: None,
                                level,
                                name: name.clone(),
                                key: identifiers.key.clone(),
                                tag: identifiers.tag.clone(),
                                method: execution.method.clone(),
                                url: execution.url.clone(),
                                executed_at: row.executed_at,
                                duration: row.duration,
                                logs: execution.logs,
                                status,
                                status_text,
                                has_response_headers,
                                has_curl: execution.curl.is_some(),
                                response_body_length,
                                success,
                                error: execution.error.clone(),
                                request_success_count: row.request_success_count,
                                request_failure_count: row.request_failure_count,
                                request_error_count: row.request_error_count,
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
                                    output_variables: execution
                                        .output_variables
                                        .as_ref()
                                        .map(|arc| (**arc).clone()),
                                    tests: execution.tests,
                                    curl: execution.curl,
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
        }

        indexes
    }

    /// Get execution summaries, grouped by executing request
    pub fn get_summaries(
        &self,
        request_or_group_id: &str,
        include_all_results: bool,
    ) -> IndexMap<String, Vec<&ExecutionResultSummary>> {
        if let Some(request_results) = self.executing_request_index.get(request_or_group_id) {
            request_results
                .into_iter()
                .filter(|(executing_request_id, _)| {
                    include_all_results || *executing_request_id == request_or_group_id
                })
                .map(|(executing_request_id, exec_ctrs)| {
                    (
                        executing_request_id.to_string(),
                        exec_ctrs // exec_ctr_list
                            .iter()
                            .filter_map(|exec_ctr| self.results.get(exec_ctr))
                            .map(|x| &x.0)
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
        exec_ctr: &usize,
        results: &mut IndexMap<usize, ExecutionResultSummary>,
    ) {
        if let Some((summary, ..)) = self.results.get(exec_ctr) {
            results.insert(summary.exec_ctr, summary.clone());
            if let Some(child_exec_ctrs) = &summary.child_exec_ctrs {
                for child_exec_ctr in child_exec_ctrs {
                    if !results.contains_key(child_exec_ctr) {
                        self.collect_summaries(child_exec_ctr, results);
                    }
                }
            }
        }
    }

    /// Get execution summaries, grouped by executing request
    pub fn get_result_summaries(
        &self,
        exec_ctr: &usize,
    ) -> IndexMap<usize, ExecutionResultSummary> {
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

    /// Delete out any stored request index entries for the specified request/group,
    /// return request/group IDs that were impacted
    pub fn delete_indexed_request_results(
        &mut self,
        executing_request_or_group_id: &str,
    ) -> Vec<String> {
        // Remove all results for which the specified request executed them
        let mut results = Vec::<String>::with_capacity(self.executing_request_index.len());
        for (id, request_executions) in &mut self.executing_request_index {
            if request_executions
                .shift_remove(executing_request_or_group_id)
                .is_some()
            {
                results.push(id.to_string())
            }
        }
        results
    }

    /// Add request index entry, storing which request the execution was returned from
    fn add_index_entries(
        &mut self,
        request_or_group_id: &str,
        executing_request_or_group_id: &str,
        exec_ctr: usize,
    ) {
        if let Some(existing_request) = self.executing_request_index.get_mut(request_or_group_id) {
            if let Some(executing_request) = existing_request.get_mut(executing_request_or_group_id)
            {
                executing_request.push(exec_ctr);
            } else {
                let insert_at = if let Some((first, _)) = existing_request.first() {
                    if first == request_or_group_id
                        && request_or_group_id != executing_request_or_group_id
                    {
                        1
                    } else {
                        0
                    }
                } else {
                    0
                };

                existing_request.shift_insert(
                    insert_at,
                    executing_request_or_group_id.to_string(),
                    vec![exec_ctr],
                );
            }
        } else {
            self.executing_request_index.insert(
                request_or_group_id.to_string(),
                IndexMap::from([(executing_request_or_group_id.to_string(), vec![exec_ctr])]),
            );
        }
    }

    fn append_request_result_runs(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        runs: Vec<ApicizeRequestResultRun>,
        parent_exec_ctr: Option<usize>,
        result_context: &mut ResultContext,
    ) -> Vec<usize> {
        let level = result_context.level;
        let active_request_ids = result_context.active_request_ids;
        let identifiers = result_context.identifiers;
        let run_count = runs.len();
        let mut exec_ctrs = Vec::<usize>::with_capacity(run_count);

        let mut active_request_ids = active_request_ids.clone();
        active_request_ids.insert(request_or_group_id.to_string());

        for (run_number, run) in (1..).zip(runs) {
            let success = success_from_tallies(&run);
            let name = format!(
                "{} (Run {} of {})",
                identifiers.title, run_number, run_count,
            );

            let (status, status_text, has_response_headers, response_body_length) =
                get_response_info(&run.execution);

            let exec_ctr = self.increment_counter();

            for active_request_id in &active_request_ids {
                self.add_index_entries(
                    active_request_id,
                    context.get_executing_request_or_group_id(),
                    exec_ctr,
                );
            }

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
                        logs: None,
                        status,
                        status_text,
                        has_response_headers,
                        has_curl: run.execution.curl.is_some(),
                        response_body_length,
                        success,
                        error: run.execution.error.clone(),
                        request_success_count: run.request_success_count,
                        request_failure_count: run.request_failure_count,
                        request_error_count: run.request_error_count,
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
                        output_variables: run
                            .execution
                            .output_variables
                            .as_ref()
                            .map(|arc| (**arc).clone()),
                        tests: run.execution.tests,
                        curl: run.execution.curl,
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
        }

        exec_ctrs
    }

    fn append_group_result_rows(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        rows: Vec<ApicizeGroupResultRow>,
        result_context: &mut ResultContext,
    ) -> Vec<usize> {
        let level = result_context.level;
        let active_request_ids = result_context.active_request_ids;
        let identifiers = result_context.identifiers;
        let row_count = rows.len();
        let mut indexes = Vec::<usize>::with_capacity(row_count);

        let mut active_request_ids = active_request_ids.clone();
        active_request_ids.insert(request_or_group_id.to_string());

        for (row_number, row) in (1..).zip(rows) {
            let success = success_from_tallies(&row);
            let name = format!(
                "{} (Row {} of {})",
                identifiers.title, row_number, row_count,
            );

            let exec_ctr = self.increment_counter();
            for active_request_id in &active_request_ids {
                self.add_index_entries(
                    active_request_id,
                    context.get_executing_request_or_group_id(),
                    exec_ctr,
                );
            }
            self.results.insert(
                exec_ctr,
                (
                    ExecutionResultSummary {
                        exec_ctr,
                        request_or_group_id: identifiers.id.clone(),
                        parent_exec_ctr: Some(identifiers.parent_exec_ctr),
                        child_exec_ctrs: None,
                        level,
                        name: name.clone(),
                        key: identifiers.key.clone(),
                        tag: identifiers.tag.clone(),
                        method: None,
                        url: None,
                        executed_at: row.executed_at,
                        duration: row.duration,
                        logs: None,
                        status: None,
                        status_text: None,
                        has_response_headers: false,
                        has_curl: false,
                        response_body_length: None,
                        success,
                        error: None,
                        request_success_count: row.request_success_count,
                        request_failure_count: row.request_failure_count,
                        request_error_count: row.request_error_count,
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
                    let mut nested_context = ResultContext {
                        level: level + 1,
                        active_request_ids: &active_request_ids,
                        identifiers,
                        updated_request_ids: result_context.updated_request_ids,
                    };
                    child_exec_ctrs = self.append_group_result_runs(
                        request_or_group_id,
                        context,
                        runs,
                        &mut nested_context,
                    );
                }
                ApicizeGroupResultRowContent::Results { results } => {
                    child_exec_ctrs = vec![];
                    for result in results {
                        child_exec_ctrs.push(self.append_result(
                            context,
                            result,
                            level + 1,
                            &active_request_ids,
                            Some(exec_ctr),
                            result_context.updated_request_ids,
                        ));
                    }
                }
            }

            if !child_exec_ctrs.is_empty() {
                self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs = Some(child_exec_ctrs);
            }

            indexes.push(exec_ctr);
        }

        indexes
    }

    fn append_group_result_runs(
        &mut self,
        request_or_group_id: &str,
        context: &TestRunnerContext,
        runs: Vec<ApicizeGroupResultRun>,
        result_context: &mut ResultContext,
    ) -> Vec<usize> {
        let level = result_context.level;
        let active_request_ids = result_context.active_request_ids;
        let identifiers = result_context.identifiers;
        let run_count = runs.len();
        let mut child_exec_ctrs = Vec::<usize>::with_capacity(run_count);

        let mut active_request_ids = active_request_ids.clone();
        active_request_ids.insert(request_or_group_id.to_string());

        for (run_number, run) in (1..).zip(runs) {
            let success = success_from_tallies(&run);
            let name = format!(
                "{} (Run {} of {})",
                identifiers.title, run_number, run_count
            );

            let exec_ctr = self.increment_counter();

            for active_request_id in &active_request_ids {
                self.add_index_entries(
                    active_request_id,
                    context.get_executing_request_or_group_id(),
                    exec_ctr,
                );
            }
            self.results.insert(
                exec_ctr,
                (
                    ExecutionResultSummary {
                        exec_ctr,
                        request_or_group_id: identifiers.id.to_string(),
                        parent_exec_ctr: Some(identifiers.parent_exec_ctr),
                        child_exec_ctrs: None,
                        level,
                        name: name.clone(),
                        key: identifiers.key.clone(),
                        tag: identifiers.tag.clone(),
                        method: None,
                        url: None,
                        executed_at: run.executed_at,
                        duration: run.duration,
                        logs: None,
                        status: None,
                        status_text: None,
                        has_response_headers: false,
                        has_curl: false,
                        response_body_length: None,
                        success,
                        error: None,
                        request_success_count: run.request_success_count,
                        request_failure_count: run.request_failure_count,
                        request_error_count: run.request_error_count,
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

            let child_indexes = run
                .results
                .into_iter()
                .map(|result| {
                    self.append_result(
                        context,
                        result,
                        level + 1,
                        &active_request_ids,
                        Some(exec_ctr),
                        result_context.updated_request_ids,
                    )
                })
                .collect::<Vec<usize>>();

            if !child_indexes.is_empty() {
                self.results.get_mut(&exec_ctr).unwrap().0.child_exec_ctrs = Some(child_indexes);
            }

            child_exec_ctrs.push(exec_ctr);
        }

        child_exec_ctrs
    }

    /// Return next incremented counter
    fn increment_counter(&mut self) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ApicizeBody, ApicizeExecution, ApicizeExecutionTestContext, ApicizeGroupResult,
        ApicizeGroupResultContent, ApicizeHttpResponse, ApicizeRequestResult,
        ApicizeRequestResultContent, ApicizeRequestResultRow, ApicizeRequestResultRowContent,
        ApicizeRequestResultRun, ApicizeResult, DataContext, IndexedEntities, Request,
        RequestEntry, TestRunnerContext, TestRunnerContextInit, WorkbookDefaultParameters,
        Workspace, workspace::ParameterLockStatus,
    };
    use serde_json::json;
    use std::collections::HashMap;

    // ========================================================================
    // Helper functions for building test fixtures
    // ========================================================================

    fn make_test_context() -> TestRunnerContext {
        let workspace = Workspace {
            private_lock_status: ParameterLockStatus::UnlockedNoPassword,
            vault_lock_status: ParameterLockStatus::UnlockedNoPassword,
            private_password: None,
            vault_password: None,
            requests: IndexedEntities::<RequestEntry>::new(&vec![RequestEntry::Request(Request {
                id: "test-req".to_string(),
                name: "Test Request".to_string(),
                ..Default::default()
            })]),
            scenarios: IndexedEntities::default(),
            authorizations: IndexedEntities::default(),
            certificates: IndexedEntities::default(),
            proxies: IndexedEntities::default(),
            data: IndexedEntities::default(),
            defaults: WorkbookDefaultParameters::default(),
            private_encryption: None,
            vault_encryption: None,
        };

        TestRunnerContext::new(TestRunnerContextInit {
            workspace,
            cancellation: None,
            executing_request_or_group_id: "test-exec",
            single_run_no_timeout: false,
            allowed_data_path: &None,
            enable_trace: false,
            generate_curl: false,
            execution_counter_callback: None,
        })
    }

    fn make_execution(
        name: &str,
        method: Option<&str>,
        url: Option<&str>,
        status: Option<u16>,
    ) -> ApicizeExecution {
        let response = status.map(|s| ApicizeHttpResponse {
            status: s,
            status_text: "OK".to_string(),
            headers: Some(HashMap::new()),
            body: Some(ApicizeBody::Text {
                text: "response body".to_string(),
            }),
            oauth2_token: None,
        });

        ApicizeExecution {
            name: name.to_string(),
            key: None,
            method: method.map(|m| m.to_string()),
            url: url.map(|u| u.to_string()),
            test_context: ApicizeExecutionTestContext {
                merged: None,
                scenario: None,
                output: None,
                data: None,
                request: None,
                response,
            },
            output_variables: None,
            logs: None,
            tests: None,
            curl: None,
            error: None,
            success: true,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    fn make_request_result_execution(
        id: &str,
        name: &str,
        execution: ApicizeExecution,
    ) -> ApicizeRequestResult {
        ApicizeRequestResult {
            id: id.to_string(),
            name: name.to_string(),
            key: None,
            tag: None,
            url: execution.url.clone(),
            executed_at: 100,
            duration: 50,
            data_context: DataContext::default(),
            content: ApicizeRequestResultContent::Execution {
                execution: Box::new(execution),
            },
            logs: None,
            success: true,
            request_success_count: 1,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    fn make_request_result_with_runs(
        id: &str,
        name: &str,
        runs: Vec<ApicizeRequestResultRun>,
    ) -> ApicizeRequestResult {
        let request_success_count = runs.iter().filter(|r| r.success).count();
        let request_failure_count = runs.len() - request_success_count;

        ApicizeRequestResult {
            id: id.to_string(),
            name: name.to_string(),
            key: None,
            tag: None,
            url: None,
            executed_at: 100,
            duration: 150,
            data_context: DataContext::default(),
            content: ApicizeRequestResultContent::Runs { runs },
            logs: None,
            success: request_failure_count == 0,
            request_success_count,
            request_failure_count,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    fn make_request_result_with_rows(
        id: &str,
        name: &str,
        rows: Vec<ApicizeRequestResultRow>,
    ) -> ApicizeRequestResult {
        let request_success_count = rows.iter().filter(|r| r.success).count();
        let request_failure_count = rows.len() - request_success_count;

        ApicizeRequestResult {
            id: id.to_string(),
            name: name.to_string(),
            key: None,
            tag: None,
            url: None,
            executed_at: 100,
            duration: 200,
            data_context: DataContext::default(),
            content: ApicizeRequestResultContent::Rows { rows },
            logs: None,
            success: request_failure_count == 0,
            request_success_count,
            request_failure_count,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    fn make_group_result_with_results(
        id: &str,
        name: &str,
        results: Vec<ApicizeResult>,
    ) -> ApicizeGroupResult {
        ApicizeGroupResult {
            id: id.to_string(),
            name: name.to_string(),
            key: None,
            tag: None,
            executed_at: 100,
            duration: 300,
            data_context: DataContext::default(),
            content: ApicizeGroupResultContent::Results { results },
            logs: None,
            success: true,
            request_success_count: 1,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    fn make_request_run(run_number: usize, success: bool) -> ApicizeRequestResultRun {
        ApicizeRequestResultRun {
            run_number,
            executed_at: 100 + (run_number as u128 * 10),
            duration: 10,
            execution: make_execution("Test", Some("GET"), Some("http://test.com"), Some(200)),
            success,
            request_success_count: if success { 1 } else { 0 },
            request_failure_count: if success { 0 } else { 1 },
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    fn make_request_row(row_number: usize) -> ApicizeRequestResultRow {
        ApicizeRequestResultRow {
            row_number,
            executed_at: 100 + (row_number as u128 * 10),
            duration: 10,
            data_context: DataContext::default(),
            results: ApicizeRequestResultRowContent::Execution(Box::new(make_execution(
                "Test",
                Some("GET"),
                Some("http://test.com"),
                Some(200),
            ))),
            success: true,
            request_success_count: 1,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }

    // Mock implementation of Tally trait for testing
    struct MockTally {
        success: bool,
        request_success_count: usize,
        request_failure_count: usize,
        request_error_count: usize,
        test_pass_count: usize,
        test_fail_count: usize,
    }

    impl Tally for MockTally {
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

    // ========================================================================
    // Builder initialization tests
    // ========================================================================

    #[test]
    fn test_default_builder_has_zero_counter() {
        let builder = ExecutionResultBuilder::default();
        assert_eq!(builder.exec_ctr, 0);
    }

    #[test]
    fn test_with_exec_ctr_sets_initial_counter() {
        let builder = ExecutionResultBuilder::with_exec_ctr(42);
        assert_eq!(builder.exec_ctr, 42);
    }

    #[test]
    fn test_default_builder_has_empty_results() {
        let builder = ExecutionResultBuilder::default();
        assert_eq!(builder.results.len(), 0);
    }

    #[test]
    fn test_default_builder_has_empty_index() {
        let builder = ExecutionResultBuilder::default();
        assert_eq!(builder.executing_request_index.len(), 0);
    }

    // ========================================================================
    // Counter management tests
    // ========================================================================

    #[test]
    fn test_increment_counter_increments() {
        let mut builder = ExecutionResultBuilder::default();
        let first = builder.increment_counter();
        let second = builder.increment_counter();
        assert_eq!(first, 1);
        assert_eq!(second, 2);
    }

    #[test]
    fn test_increment_counter_wraps() {
        let mut builder = ExecutionResultBuilder::with_exec_ctr(usize::MAX);
        let wrapped = builder.increment_counter();
        assert_eq!(wrapped, 0);
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_success_from_tallies_with_errors_returns_error() {
        let tally = MockTally {
            success: false,
            request_success_count: 0,
            request_failure_count: 0,
            request_error_count: 1,
            test_pass_count: 0,
            test_fail_count: 0,
        };
        assert_eq!(success_from_tallies(&tally), ExecutionResultSuccess::Error);
    }

    #[test]
    fn test_success_from_tallies_with_failures_returns_failure() {
        let tally = MockTally {
            success: false,
            request_success_count: 1,
            request_failure_count: 1,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 1,
        };
        assert_eq!(
            success_from_tallies(&tally),
            ExecutionResultSuccess::Failure
        );
    }

    #[test]
    fn test_success_from_tallies_with_test_failures_returns_failure() {
        let tally = MockTally {
            success: false,
            request_success_count: 1,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 1,
        };
        assert_eq!(
            success_from_tallies(&tally),
            ExecutionResultSuccess::Failure
        );
    }

    #[test]
    fn test_success_from_tallies_with_all_success_returns_success() {
        let tally = MockTally {
            success: true,
            request_success_count: 1,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 1,
            test_fail_count: 0,
        };
        assert_eq!(
            success_from_tallies(&tally),
            ExecutionResultSuccess::Success
        );
    }

    #[test]
    fn test_get_response_info_with_response() {
        let execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        let (status, status_text, has_headers, body_length) = get_response_info(&execution);
        assert_eq!(status, Some(200));
        assert_eq!(status_text, Some("OK".to_string()));
        assert_eq!(has_headers, true);
        assert_eq!(body_length, Some("response body".len()));
    }

    #[test]
    fn test_get_response_info_without_response() {
        let mut execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        execution.test_context.response = None;
        let (status, status_text, has_headers, body_length) = get_response_info(&execution);
        assert_eq!(status, None);
        assert_eq!(status_text, None);
        assert_eq!(has_headers, false);
        assert_eq!(body_length, None);
    }

    #[test]
    fn test_get_response_info_with_json_body() {
        let mut execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        if let Some(ref mut response) = execution.test_context.response {
            response.body = Some(ApicizeBody::JSON {
                text: r#"{"key":"value"}"#.to_string(),
                data: json!({"key": "value"}),
            });
        }
        let (_, _, _, body_length) = get_response_info(&execution);
        assert_eq!(body_length, Some(r#"{"key":"value"}"#.len()));
    }

    #[test]
    fn test_get_response_info_with_binary_body() {
        let mut execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        if let Some(ref mut response) = execution.test_context.response {
            response.body = Some(ApicizeBody::Binary {
                data: vec![1, 2, 3, 4, 5],
            });
        }
        let (_, _, _, body_length) = get_response_info(&execution);
        assert_eq!(body_length, Some(5));
    }

    // ========================================================================
    // Request result processing tests
    // ========================================================================

    #[test]
    fn test_append_request_result_execution_creates_summary_and_detail() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();
        let execution = make_execution(
            "Test Request",
            Some("GET"),
            Some("http://test.com"),
            Some(200),
        );
        let request = make_request_result_execution("req-1", "Test Request", execution);

        let exec_ctr = builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        assert_eq!(exec_ctr, 1);
        let (summary, detail) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.exec_ctr, exec_ctr);
        assert_eq!(summary.request_or_group_id, "req-1");
        assert_eq!(summary.name, "Test Request");
        assert_eq!(summary.method, Some("GET".to_string()));
        assert_eq!(summary.url, Some("http://test.com".to_string()));
        assert_eq!(summary.status, Some(200));

        match detail {
            ExecutionResultDetail::Request(req_detail) => {
                assert_eq!(req_detail.request_id, "req-1");
                assert_eq!(req_detail.name, "Test Request");
            }
            _ => panic!("Expected Request detail"),
        }
    }

    #[test]
    fn test_append_request_result_with_runs_creates_child_entries() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();
        let runs = vec![make_request_run(1, true), make_request_run(2, true)];
        let request = make_request_result_with_runs("req-1", "Test Request", runs);

        let exec_ctr = builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        let (summary, _) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.child_exec_ctrs.as_ref().unwrap().len(), 2);

        // Verify child entries were created
        let child_ctrs = summary.child_exec_ctrs.as_ref().unwrap();
        for (idx, child_ctr) in child_ctrs.iter().enumerate() {
            let (child_summary, _) = builder.results.get(child_ctr).unwrap();
            assert_eq!(child_summary.run_number, Some(idx + 1));
            assert_eq!(child_summary.run_count, Some(2));
            assert_eq!(child_summary.parent_exec_ctr, Some(exec_ctr));
        }
    }

    #[test]
    fn test_append_request_result_with_rows_creates_child_entries() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();
        let rows = vec![
            make_request_row(1),
            make_request_row(2),
            make_request_row(3),
        ];
        let request = make_request_result_with_rows("req-1", "Test Request", rows);

        let exec_ctr = builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        let (summary, _) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.child_exec_ctrs.as_ref().unwrap().len(), 3);

        // Verify row numbers are set correctly
        let child_ctrs = summary.child_exec_ctrs.as_ref().unwrap();
        for (idx, child_ctr) in child_ctrs.iter().enumerate() {
            let (child_summary, _) = builder.results.get(child_ctr).unwrap();
            assert_eq!(child_summary.row_number, Some(idx + 1));
            assert_eq!(child_summary.row_count, Some(3));
        }
    }

    #[test]
    fn test_append_request_result_tracks_updated_request_ids() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();
        let execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Test", execution);

        let mut updated_request_ids = IndexSet::new();
        builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut updated_request_ids,
        );

        assert!(updated_request_ids.contains("req-1"));
        assert_eq!(updated_request_ids.len(), 1);
    }

    // ========================================================================
    // Group result processing tests
    // ========================================================================

    #[test]
    fn test_append_group_result_creates_summary_and_detail() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let execution = make_execution(
            "Child Request",
            Some("GET"),
            Some("http://test.com"),
            Some(200),
        );
        let child_request = make_request_result_execution("req-1", "Child Request", execution);
        let group = make_group_result_with_results(
            "group-1",
            "Test Group",
            vec![ApicizeResult::Request(Box::new(child_request))],
        );

        let exec_ctr = builder.append_group_result(
            &context,
            group,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        let (summary, detail) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.request_or_group_id, "group-1");
        assert_eq!(summary.name, "Test Group");
        assert_eq!(summary.child_exec_ctrs.as_ref().unwrap().len(), 1);

        match detail {
            ExecutionResultDetail::Grouped(group_detail) => {
                assert_eq!(group_detail.group_id, "group-1");
            }
            _ => panic!("Expected Grouped detail"),
        }
    }

    #[test]
    fn test_append_group_result_with_nested_groups() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        // Create nested structure: group -> child group -> request
        let execution = make_execution("Request", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Request", execution);
        let child_group = make_group_result_with_results(
            "group-2",
            "Child Group",
            vec![ApicizeResult::Request(Box::new(request))],
        );
        let parent_group = make_group_result_with_results(
            "group-1",
            "Parent Group",
            vec![ApicizeResult::Group(Box::new(child_group))],
        );

        let exec_ctr = builder.append_group_result(
            &context,
            parent_group,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        // Verify hierarchy: parent -> child group -> request
        let (summary, _) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.child_exec_ctrs.as_ref().unwrap().len(), 1);

        let child_group_ctr = summary.child_exec_ctrs.as_ref().unwrap()[0];
        let (child_summary, _) = builder.results.get(&child_group_ctr).unwrap();
        assert_eq!(child_summary.request_or_group_id, "group-2");
        assert_eq!(child_summary.child_exec_ctrs.as_ref().unwrap().len(), 1);
        assert_eq!(child_summary.parent_exec_ctr, Some(exec_ctr));

        let request_ctr = child_summary.child_exec_ctrs.as_ref().unwrap()[0];
        let (request_summary, _) = builder.results.get(&request_ctr).unwrap();
        assert_eq!(request_summary.request_or_group_id, "req-1");
        assert_eq!(request_summary.parent_exec_ctr, Some(child_group_ctr));
    }

    // ========================================================================
    // Index management tests
    // ========================================================================

    #[test]
    fn test_add_index_entries_creates_new_index_for_first_entry() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_entries("req-1", "exec-1", 1);

        assert_eq!(builder.executing_request_index.len(), 1);
        assert!(builder.executing_request_index.contains_key("req-1"));
        let exec_map = builder.executing_request_index.get("req-1").unwrap();
        assert_eq!(exec_map.len(), 1);
        assert_eq!(exec_map.get("exec-1").unwrap(), &vec![1]);
    }

    #[test]
    fn test_add_index_entries_appends_to_existing_executor() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_entries("req-1", "exec-1", 1);
        builder.add_index_entries("req-1", "exec-1", 2);
        builder.add_index_entries("req-1", "exec-1", 3);

        let exec_map = builder.executing_request_index.get("req-1").unwrap();
        assert_eq!(exec_map.get("exec-1").unwrap(), &vec![1, 2, 3]);
    }

    #[test]
    fn test_add_index_entries_tracks_multiple_executors() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_entries("req-1", "exec-1", 1);
        builder.add_index_entries("req-1", "exec-2", 2);

        let exec_map = builder.executing_request_index.get("req-1").unwrap();
        assert_eq!(exec_map.len(), 2);
        assert_eq!(exec_map.get("exec-1").unwrap(), &vec![1]);
        assert_eq!(exec_map.get("exec-2").unwrap(), &vec![2]);
    }

    #[test]
    fn test_delete_indexed_request_results_removes_executor_entries() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_entries("req-1", "exec-1", 1);
        builder.add_index_entries("req-2", "exec-1", 2);
        builder.add_index_entries("req-1", "exec-2", 3);

        let deleted = builder.delete_indexed_request_results("exec-1");

        assert_eq!(deleted.len(), 2);
        assert!(deleted.contains(&"req-1".to_string()));
        assert!(deleted.contains(&"req-2".to_string()));

        // Verify exec-1 entries are gone
        let req1_map = builder.executing_request_index.get("req-1").unwrap();
        assert!(!req1_map.contains_key("exec-1"));
        assert!(req1_map.contains_key("exec-2"));

        assert_eq!(
            builder.executing_request_index.get("req-2").unwrap().len(),
            0
        );
    }

    #[test]
    fn test_delete_indexed_request_results_handles_non_existent_executor() {
        let mut builder = ExecutionResultBuilder::default();
        builder.add_index_entries("req-1", "exec-1", 1);

        let deleted = builder.delete_indexed_request_results("non-existent");

        assert_eq!(deleted.len(), 0);
        assert_eq!(
            builder.executing_request_index.get("req-1").unwrap().len(),
            1
        );
    }

    // ========================================================================
    // Retrieval method tests
    // ========================================================================

    #[test]
    fn test_get_summaries_returns_empty_for_non_existent_request() {
        let builder = ExecutionResultBuilder::default();
        let summaries = builder.get_summaries("non-existent", false);
        assert_eq!(summaries.len(), 0);
    }

    #[test]
    fn test_get_summaries_filters_by_executing_request() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Test", execution);
        builder.process_result(&context, ApicizeResult::Request(Box::new(request)));

        // Get all results regardless of who executed them
        let summaries = builder.get_summaries("req-1", true);
        assert_eq!(summaries.len(), 1);
        assert!(summaries.contains_key("test-exec"));
    }

    #[test]
    fn test_get_detail_returns_detail_for_valid_exec_ctr() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Test", execution);
        builder.process_result(&context, ApicizeResult::Request(Box::new(request)));

        let summaries = builder.get_summaries("req-1", true);
        let first_summary = summaries.values().next().unwrap().first().unwrap();

        let result = builder.get_detail(&first_summary.exec_ctr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_detail_returns_error_for_invalid_exec_ctr() {
        let builder = ExecutionResultBuilder::default();
        let result = builder.get_detail(&999);

        assert!(result.is_err());
        match result.err().unwrap() {
            ApicizeError::InvalidId { description } => {
                assert!(description.contains("999"));
            }
            _ => panic!("Expected InvalidId error"),
        }
    }

    #[test]
    fn test_get_result_returns_both_summary_and_detail() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Test", execution);
        builder.process_result(&context, ApicizeResult::Request(Box::new(request)));

        let summaries = builder.get_summaries("req-1", true);
        let first_summary = summaries.values().next().unwrap().first().unwrap();

        let result = builder.get_result(&first_summary.exec_ctr);
        assert!(result.is_ok());

        let (summary, detail) = result.unwrap();
        assert_eq!(summary.exec_ctr, first_summary.exec_ctr);
        match detail {
            ExecutionResultDetail::Request(_) => {}
            _ => panic!("Expected Request detail"),
        }
    }

    #[test]
    fn test_get_result_summaries_collects_hierarchy() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let runs = vec![make_request_run(1, true), make_request_run(2, true)];
        let request = make_request_result_with_runs("req-1", "Test", runs);
        builder.process_result(&context, ApicizeResult::Request(Box::new(request)));

        let summaries_map = builder.get_summaries("req-1", true);
        let first_summary = summaries_map.values().next().unwrap().first().unwrap();

        let all_summaries = builder.get_result_summaries(&first_summary.exec_ctr);

        // Should have parent + 2 child runs = 3 total
        assert_eq!(all_summaries.len(), 3);
    }

    // ========================================================================
    // Integration scenario tests
    // ========================================================================

    #[test]
    fn test_process_result_deletes_previous_results() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        // First execution
        let execution1 = make_execution("Test 1", Some("GET"), Some("http://test.com"), Some(200));
        let request1 = make_request_result_execution("req-1", "Test 1", execution1);
        builder.process_result(&context, ApicizeResult::Request(Box::new(request1)));

        // Second execution (should replace first)
        let execution2 = make_execution("Test 2", Some("GET"), Some("http://test.com"), Some(200));
        let request2 = make_request_result_execution("req-1", "Test 2", execution2);
        builder.process_result(&context, ApicizeResult::Request(Box::new(request2)));

        // Should only have results from second execution
        let summaries = builder.get_summaries("req-1", true);
        let test_exec_summaries = summaries.get("test-exec").unwrap();
        assert_eq!(test_exec_summaries.len(), 1);
        assert_eq!(test_exec_summaries[0].name, "Test 2");
    }

    #[test]
    fn test_process_result_returns_updated_request_ids() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let execution = make_execution("Test", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Test", execution);
        let updated = builder.process_result(&context, ApicizeResult::Request(Box::new(request)));

        assert_eq!(updated.len(), 1);
        assert!(updated.contains("req-1"));
    }

    #[test]
    fn test_process_result_with_nested_results_updates_multiple_ids() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let execution = make_execution("Request", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Request", execution);
        let group = make_group_result_with_results(
            "group-1",
            "Group",
            vec![ApicizeResult::Request(Box::new(request))],
        );

        let updated = builder.process_result(&context, ApicizeResult::Group(Box::new(group)));

        assert_eq!(updated.len(), 2);
        assert!(updated.contains("group-1"));
        assert!(updated.contains("req-1"));
    }

    // ========================================================================
    // Edge case tests
    // ========================================================================

    #[test]
    fn test_empty_runs_creates_parent_with_empty_children() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let request = make_request_result_with_runs("req-1", "Test", vec![]);
        let exec_ctr = builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        let (summary, _) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.child_exec_ctrs.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_large_number_of_runs() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let runs: Vec<_> = (1..=100).map(|i| make_request_run(i, true)).collect();
        let request = make_request_result_with_runs("req-1", "Test", runs);

        let exec_ctr = builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        let (summary, _) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.child_exec_ctrs.as_ref().unwrap().len(), 100);
    }

    #[test]
    fn test_deeply_nested_groups() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        // Create 5 levels of nesting
        let execution = make_execution("Request", Some("GET"), Some("http://test.com"), Some(200));
        let request = make_request_result_execution("req-1", "Request", execution);
        let mut current: ApicizeResult = ApicizeResult::Request(Box::new(request));

        for i in (1..=5).rev() {
            let group = make_group_result_with_results(
                &format!("group-{}", i),
                &format!("Group {}", i),
                vec![current],
            );
            current = ApicizeResult::Group(Box::new(group));
        }

        let exec_ctr = match current {
            ApicizeResult::Group(group) => builder.append_group_result(
                &context,
                *group,
                0,
                None,
                &IndexSet::new(),
                &mut IndexSet::new(),
            ),
            _ => panic!("Expected group"),
        };

        // Verify we can traverse the full hierarchy
        let all_summaries = builder.get_result_summaries(&exec_ctr);
        assert_eq!(all_summaries.len(), 6); // 5 groups + 1 request
    }

    #[test]
    fn test_mixed_success_failure_counts() {
        let mut builder = ExecutionResultBuilder::default();
        let context = make_test_context();

        let runs = vec![
            make_request_run(1, true),
            make_request_run(2, false),
            make_request_run(3, true),
        ];
        let request = make_request_result_with_runs("req-1", "Test", runs);

        let exec_ctr = builder.append_request_result(
            &context,
            request,
            0,
            None,
            &IndexSet::new(),
            &mut IndexSet::new(),
        );

        let (summary, _) = builder.results.get(&exec_ctr).unwrap();
        assert_eq!(summary.request_success_count, 2);
        assert_eq!(summary.request_failure_count, 1);
    }
}

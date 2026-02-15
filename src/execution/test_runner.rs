//! Apicize test execution.
//!
//! This library supports dispatching Apicize functional web tests
use regex::Regex;
use reqwest::redirect::Policy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, Once};
use std::time::{Duration, Instant};
use xmltojson::to_json;

use async_recursion::async_recursion;
use encoding_rs::{Encoding, UTF_8};
use mime::Mime;
use reqwest::{Body, Client, Response};
use serde_json::{Map, Value};
use tokio::select;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::{
    ApicizeBody, ApicizeExecution, ApicizeExecutionTestContext, ApicizeGroupResult,
    ApicizeGroupResultContent, ApicizeGroupResultRow, ApicizeGroupResultRun, ApicizeHttpRequest,
    ApicizeHttpResponse, ApicizeRequestResult, ApicizeRequestResultRun, ApicizeResult,
    ApicizeTestBehavior, ApicizeTestResponse, ApicizeTestResult, DataContext, DataContextGenerator,
    GetDataContext, Tally,
};
use crate::oauth2_client_tokens::TokenResult;
use crate::types::workspace::RequestExecutionParameters;
use crate::workspace::RequestExecutionState;
use crate::{
    ApicizeError, ApicizeGroupResultRowContent, ApicizeRequestResultContent,
    ApicizeRequestResultRow, ApicizeRequestResultRowContent, Authorization, ExecutionConcurrency,
    Identifiable, Request, RequestBody, RequestEntry, RequestGroup, RequestMethod, VariableCache,
    Workspace, get_oauth2_client_credentials, retrieve_oauth2_token_from_cache,
};

// #[cfg(test)]
// use crate::oauth2_client_tokens::tests::MockOAuth2ClientTokens as oauth2;

static V8_INIT: Once = Once::new();

static PORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".+:(\d{1,5})(?:\?.*)?$").unwrap());

pub trait ApicizeRunner {
    fn run(
        &self,
        request_ids: Vec<String>,
    ) -> impl std::future::Future<Output = Vec<Result<ApicizeResult, ApicizeError>>> + Send;
}

/// Information about a test run
pub struct TestRunnerContext {
    /// The workspace the test is being run against
    workspace: Workspace,
    /// Token to cancel asynchronous execution
    cancellation: CancellationToken,
    /// The request which is being executed (used to track execution results)
    executing_request_or_group_id: String,
    /// Current values of variables (scenario, test-defined)
    value_cache: Mutex<VariableCache>,
    /// When test execution started
    tests_started: Instant,
    /// Used for interactive UI runs where a single execution is requested with no timeout
    single_run_no_timeout: bool,
    /// If true, reqwest trace will be enabled (for I/O logging)
    enable_trace: bool,
}

impl TestRunnerContext {
    pub fn new(
        workspace: Workspace,
        cancellation: Option<CancellationToken>,
        executing_request_or_group_id: &str,
        single_run_no_timeout: bool,
        allowed_data_path: &Option<PathBuf>,
        enable_trace: bool,
    ) -> Self {
        // Ensure V8 is initialized
        V8_INIT.call_once(|| {
            let platform = v8::new_unprotected_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        TestRunnerContext {
            workspace,
            cancellation: cancellation.unwrap_or_default(),
            executing_request_or_group_id: executing_request_or_group_id.to_string(),
            value_cache: Mutex::new(VariableCache::new(allowed_data_path)),
            tests_started: Instant::now(),
            single_run_no_timeout,
            enable_trace,
        }
    }

    pub fn ellapsed_in_ms(&self) -> u128 {
        self.tests_started.elapsed().as_millis()
    }

    pub fn get_executing_request_or_group_id(&self) -> &str {
        self.executing_request_or_group_id.as_str()
    }

    pub fn get_request_entry(
        &self,
        request_or_group_id: &str,
    ) -> Result<&RequestEntry, ApicizeError> {
        match self.workspace.requests.entities.get(request_or_group_id) {
            Some(entry) => Ok(entry),
            None => Err(ApicizeError::InvalidId {
                description: format!("Invalid Request or Group ID {request_or_group_id}"),
            }),
        }
    }

    pub fn get_request(&self, request_id: &str) -> Result<&Request, ApicizeError> {
        match self.workspace.requests.entities.get(request_id) {
            Some(RequestEntry::Request(request)) => Ok(request),
            _ => Err(ApicizeError::InvalidId {
                description: format!("Invalid Request ID {request_id}"),
            }),
        }
    }

    /// Return key of the request/group with inheritance
    pub fn get_request_key(
        &self,
        request_or_group_id: &str,
    ) -> Result<Option<String>, ApicizeError> {
        match self.workspace.requests.entities.get(request_or_group_id) {
            Some(RequestEntry::Request(request)) => {
                if let Some(key) = &request.key {
                    Ok(Some(key.clone()))
                } else {
                    for (parent_id, child_ids) in &self.workspace.requests.child_ids {
                        if child_ids.contains(&request_or_group_id.to_string()) {
                            return self.get_request_key(parent_id);
                        }
                    }
                    Ok(None)
                }
            }
            Some(RequestEntry::Group(group)) => {
                if let Some(key) = &group.key {
                    Ok(Some(key.clone()))
                } else {
                    for (parent_id, child_ids) in &self.workspace.requests.child_ids {
                        if child_ids.contains(&request_or_group_id.to_string()) {
                            return self.get_request_key(parent_id);
                        }
                    }
                    Ok(None)
                }
            }
            _ => Err(ApicizeError::InvalidId {
                description: format!("Invalid Request ID {request_or_group_id}"),
            }),
        }
    }

    pub fn get_group(&self, group_id: &str) -> Result<&RequestGroup, ApicizeError> {
        match self.workspace.requests.entities.get(group_id) {
            Some(RequestEntry::Group(group)) => Ok(group),
            _ => Err(ApicizeError::InvalidId {
                description: format!("Invalid Group ID {group_id}"),
            }),
        }
    }

    pub fn get_group_children(&self, group_id: &str) -> &[String] {
        match self.workspace.requests.child_ids.get(group_id) {
            Some(child_ids) => child_ids.as_slice(),
            None => &[],
        }
    }
}

impl ApicizeRunner for Arc<TestRunnerContext> {
    async fn run(
        &self,
        request_entry_ids: Vec<String>,
    ) -> Vec<Result<ApicizeResult, ApicizeError>> {
        let mut results =
            Vec::<Result<ApicizeResult, ApicizeError>>::with_capacity(request_entry_ids.len());
        for request_entry_id in request_entry_ids {
            let result = Box::pin(run_request_entry(
                self.clone(),
                request_entry_id,
                Arc::new(RequestExecutionParameters::default()),
                Arc::new(RequestExecutionState::default()),
            ))
            .await;
            match result {
                Ok(Some(r)) => {
                    results.push(Ok(r));
                }
                Ok(None) => {}
                Err(err) => results.push(Err(err)),
            }
        }
        results
    }
}

async fn run_request_entry(
    context: Arc<TestRunnerContext>,
    request_or_group_id: String,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Option<ApicizeResult>, ApicizeError> {
    let entry = context.get_request_entry(&request_or_group_id)?;

    let new_params =
        context
            .workspace
            .retrieve_request_parameters(entry, &context.value_cache, &params)?;

    let new_state = if state.row.is_some() && !new_params.data_enabled {
        // If a row is active but we are not using a data set now (because the request/group data parameter
        // has been set to off or is different than what is in use) then we need to not use the
        // active row anymore
        Arc::new(RequestExecutionState {
            output_variables: state.output_variables.clone(),
            row: None,
        })
    } else {
        state.clone()
    };

    match entry {
        RequestEntry::Request(..) => match run_request(
            context.clone(),
            &request_or_group_id,
            Arc::new(new_params),
            new_state,
        )
        .await?
        {
            Some(result) => Ok(Some(ApicizeResult::Request(result))),
            None => Ok(None),
        },
        RequestEntry::Group(..) => match run_group(
            context.clone(),
            &request_or_group_id,
            Arc::new(new_params),
            new_state,
        )
        .await?
        {
            Some(result) => Ok(Some(ApicizeResult::Group(result))),
            None => Ok(None),
        },
    }
}

async fn run_request(
    context: Arc<TestRunnerContext>,
    request_id: &str,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Option<Box<ApicizeRequestResult>>, ApicizeError> {
    let key = context.get_request_key(request_id)?;
    let executed_at = context.ellapsed_in_ms();
    let request = context.get_request(request_id)?;
    let multi_run = request.runs > 1 && (!context.single_run_no_timeout);

    if request.runs < 1 {
        return Ok(None);
    }

    let (content, data_context, tallies) = if params.data_set.is_some() && state.row.is_none() {
        let rows =
            run_request_rows(context.clone(), request_id, params.clone(), state.clone()).await?;
        let data_context = rows.generate_data_context();
        let tallies = rows.get_tallies();
        (
            ApicizeRequestResultContent::Rows { rows },
            data_context,
            tallies,
        )
    } else if multi_run {
        let runs =
            run_request_runs(context.clone(), request_id, params.clone(), state.clone()).await?;
        let data_context = runs.generate_data_context();
        let tallies = runs.get_tallies();
        (
            ApicizeRequestResultContent::Runs { runs },
            data_context,
            tallies,
        )
    } else {
        let execution =
            dispatch_request_and_test(context.clone(), request_id.to_string(), params, state)
                .await?;
        let data_context = execution.generate_data_context();
        let tallies = execution.get_tallies();
        (
            ApicizeRequestResultContent::Execution {
                execution: Box::new(execution),
            },
            data_context,
            tallies,
        )
    };

    Ok(Some(Box::new(ApicizeRequestResult {
        id: request.id.clone(),
        name: request.get_title(),
        key,
        tag: None,
        url: None,
        executed_at,
        duration: context.ellapsed_in_ms() - executed_at,
        data_context,
        content,
        success: tallies.success,
        request_success_count: tallies.request_success_count,
        request_failure_count: tallies.request_failure_count,
        request_error_count: tallies.request_error_count,
        test_pass_count: tallies.test_pass_count,
        test_fail_count: tallies.test_fail_count,
    })))
}

async fn run_request_rows(
    context: Arc<TestRunnerContext>,
    request_id: &str,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Vec<ApicizeRequestResultRow>, ApicizeError> {
    let request = context.get_request(request_id)?;
    let mut row_number = 1;
    match params.data_set.as_ref() {
        None => Err(ApicizeError::Error {
            description: "run_request_rows called with no rows defined".to_string(),
        }),
        Some(data_set) => {
            let mut rows = Vec::<ApicizeRequestResultRow>::with_capacity(data_set.data.len());

            for row in &data_set.data {
                // Generate row params updating inbound variable values with external data row values

                let row_executed_at = context.ellapsed_in_ms();

                let row_state = Arc::new(RequestExecutionState {
                    row: Some(Arc::new(row.clone())),
                    output_variables: state.output_variables.clone(),
                });

                if request.runs == 1 {
                    let execution = dispatch_request_and_test(
                        context.clone(),
                        request_id.to_string(),
                        params.clone(),
                        row_state.clone(),
                    )
                    .await?;

                    let data_context = DataContext {
                        scenario: params.variables.clone(),
                        data: row_state.row.clone(),
                        output: row_state.output_variables.clone(),
                        output_result: execution.output_variables.clone(),
                    };

                    let taliles = execution.get_tallies();

                    rows.push(ApicizeRequestResultRow {
                        row_number,
                        executed_at: row_executed_at,
                        duration: context.ellapsed_in_ms() - row_executed_at,
                        data_context,
                        results: ApicizeRequestResultRowContent::Execution(Box::new(execution)),
                        success: taliles.success,
                        request_success_count: taliles.request_success_count,
                        request_failure_count: taliles.request_failure_count,
                        request_error_count: taliles.request_error_count,
                        test_pass_count: taliles.test_pass_count,
                        test_fail_count: taliles.test_fail_count,
                    });
                } else {
                    let runs = run_request_runs(
                        context.clone(),
                        request_id,
                        params.clone(),
                        state.clone(),
                    )
                    .await?;

                    let row_tallies = runs.get_tallies();

                    rows.push(ApicizeRequestResultRow {
                        row_number,
                        executed_at: row_executed_at,
                        duration: context.ellapsed_in_ms() - row_executed_at,
                        data_context: runs.generate_data_context(),
                        results: ApicizeRequestResultRowContent::Runs(runs),
                        success: row_tallies.success,
                        request_success_count: row_tallies.request_success_count,
                        request_failure_count: row_tallies.request_failure_count,
                        request_error_count: row_tallies.request_error_count,
                        test_pass_count: row_tallies.test_pass_count,
                        test_fail_count: row_tallies.test_fail_count,
                    });
                }

                row_number += 1;
            }

            Ok(rows)
        }
    }
}

async fn run_request_runs(
    context: Arc<TestRunnerContext>,
    request_id: &str,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Vec<ApicizeRequestResultRun>, ApicizeError> {
    let mut runs = Vec::<ApicizeRequestResultRun>::new();
    let request = context.get_request(request_id)?;
    let number_of_runs = if context.single_run_no_timeout {
        1
    } else {
        request.runs
    };

    match request.multi_run_execution {
        ExecutionConcurrency::Sequential => {
            for run_number in 1..number_of_runs + 1 {
                let run_executed_at = context.ellapsed_in_ms();
                let execution = dispatch_request_and_test(
                    context.clone(),
                    request_id.to_string(),
                    params.clone(),
                    state.clone(),
                )
                .await?;

                let success = execution.success;
                let request_failure_count = if execution.test_fail_count > 0 { 1 } else { 0 };
                let request_error_count = if execution.error.is_some() { 1 } else { 0 };
                let request_success_count = if request_failure_count > 0 || request_error_count > 0
                {
                    0
                } else {
                    1
                };
                let test_pass_count = execution.test_pass_count;
                let test_fail_count = execution.test_fail_count;

                let run = ApicizeRequestResultRun {
                    run_number,
                    executed_at: run_executed_at,
                    duration: context.ellapsed_in_ms() - run_executed_at,
                    execution,
                    success,
                    request_success_count,
                    request_failure_count,
                    request_error_count,
                    test_pass_count,
                    test_fail_count,
                };

                runs.push(run);
            }
        }
        ExecutionConcurrency::Concurrent => {
            let runs_executed_at = context.ellapsed_in_ms();
            let mut executing_runs: JoinSet<Result<ApicizeRequestResultRun, ApicizeError>> =
                JoinSet::new();

            for run_number in 1..number_of_runs + 1 {
                let context = context.clone();
                let request_id = request_id.to_string();
                let params = params.clone();
                let state = state.clone();

                executing_runs.spawn(async move {
                    select! {
                        _ = context.cancellation.cancelled() => Err(ApicizeError::Cancelled),
                        result = dispatch_request_and_test(
                            context.clone(),
                            request_id,
                            params,
                            state,
                        ) => {
                            match result {
                                Ok(execution) => {
                                    let success = execution.success;
                                    let request_failure_count = if execution.test_fail_count > 0 { 1 } else { 0 };
                                    let request_error_count  = if execution.error.is_some() { 1 } else { 0 };
                                    let request_success_count = if request_failure_count > 0 || request_error_count > 0 { 0 } else { 1 };
                                    let test_pass_count = execution.test_pass_count;
                                    let test_fail_count = execution.test_fail_count;

                                    Ok(ApicizeRequestResultRun {
                                        run_number,
                                        executed_at: runs_executed_at,
                                        duration: context.ellapsed_in_ms() - runs_executed_at,
                                        execution,
                                        success,
                                        request_success_count,
                                        request_failure_count,
                                        request_error_count,
                                        test_pass_count,
                                        test_fail_count,
                                    })
                                },
                                Err(err) => {
                                    Err(err)
                                }
                            }

                        }
                    }
                });
            }

            runs = executing_runs.join_all().await.into_iter().try_fold(
                vec![],
                |mut unrolled, result| -> Result<Vec<ApicizeRequestResultRun>, ApicizeError> {
                    unrolled.push(result?);
                    Ok(unrolled)
                },
            )?;

            runs.sort_by_key(|run| run.run_number);
        }
    }

    Ok(runs)
}

async fn run_group(
    context: Arc<TestRunnerContext>,
    group_id: &str,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Option<Box<ApicizeGroupResult>>, ApicizeError> {
    let executed_at = context.ellapsed_in_ms();

    let group = context.get_group(group_id)?;
    let key = context.get_request_key(group_id)?;
    let multi_run = group.runs > 1 && (!context.single_run_no_timeout);

    if group.runs < 1 {
        return Ok(None);
    }

    let child_ids = context.get_group_children(group_id);

    let (content, data_context, tallies) = if params.data_set.is_some() && !child_ids.is_empty() {
        // Apply all data rows to each child of a group
        let rows = run_group_rows(context.clone(), group_id, params.clone(), state.clone()).await?;
        let data_context = rows.generate_data_context();
        let tallies = rows.get_tallies();
        (
            ApicizeGroupResultContent::Rows { rows },
            data_context,
            tallies,
        )
    } else if multi_run {
        let runs = run_group_runs(context.clone(), group_id, params.clone(), state.clone()).await?;
        let data_context = runs.generate_data_context();
        let tallies = runs.get_tallies();
        (
            ApicizeGroupResultContent::Runs { runs },
            data_context,
            tallies,
        )
    } else {
        let entries = run_group_children(
            context.clone(),
            child_ids,
            params.clone(),
            state.clone(),
            &group.execution,
        )
        .await?;
        let data_context = entries.generate_data_context();
        let tallies = entries.get_tallies();
        (
            ApicizeGroupResultContent::Results { results: entries },
            data_context,
            tallies,
        )
    };

    Ok(Some(Box::new(ApicizeGroupResult {
        id: group.id.clone(),
        name: group.get_title(),
        key,
        tag: None,
        executed_at,
        duration: context.ellapsed_in_ms() - executed_at,
        data_context,
        content,
        success: tallies.success,
        request_success_count: tallies.request_success_count,
        request_failure_count: tallies.request_failure_count,
        request_error_count: tallies.request_error_count,
        test_pass_count: tallies.test_pass_count,
        test_fail_count: tallies.test_fail_count,
    })))
}

#[async_recursion]
async fn run_group_children(
    context: Arc<TestRunnerContext>,
    child_ids: &[String],
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
    concurrency: &ExecutionConcurrency,
) -> Result<Vec<ApicizeResult>, ApicizeError> {
    if !child_ids.is_empty() {
        match concurrency {
            ExecutionConcurrency::Sequential => {
                let mut results = Vec::<ApicizeResult>::with_capacity(child_ids.len());
                let mut group_state = state.clone();
                for child_id in child_ids {
                    let result = run_request_entry(
                        context.clone(),
                        child_id.clone(),
                        params.clone(),
                        group_state.clone(),
                    )
                    .await?;

                    if let Some(r) = result {
                        let result_output_variables = &r.get_data_context().output_result;
                        if result_output_variables.is_some() {
                            group_state = Arc::new(RequestExecutionState {
                                row: state.row.clone(),
                                output_variables: result_output_variables.clone(),
                            });
                        }
                        results.push(r);
                    }
                }
                Ok(results)
            }
            ExecutionConcurrency::Concurrent => {
                let mut executing_children: JoinSet<Result<Option<ApicizeResult>, ApicizeError>> =
                    JoinSet::new();

                for child_id in child_ids {
                    let context = context.clone();
                    let child_id = child_id.clone();
                    let params = params.clone();
                    let state = state.clone();

                    executing_children.spawn(async move {
                        select! {
                            _ = context.cancellation.cancelled() => Err(ApicizeError::Cancelled),
                            result = run_request_entry(
                                context.clone(),
                                child_id,
                                params,
                                state,
                            ) => {
                                result
                            }
                        }
                    });
                }

                let mut results = executing_children.join_all().await.into_iter().try_fold(
                    vec![],
                    |mut unrolled, result| {
                        match result {
                            Ok(Some(r)) => {
                                unrolled.push(r);
                            }
                            Ok(None) => {}
                            Err(err) => {
                                return Err(err);
                            }
                        }
                        Ok(unrolled)
                    },
                )?;

                results.sort_by(|a, b| {
                    let id1 = a.get_id();
                    let id2 = b.get_id();
                    let pos1 = &child_ids
                        .iter()
                        .position(|id| id == id1)
                        .unwrap_or(usize::MAX);
                    let pos2 = &child_ids
                        .iter()
                        .position(|id| id == id2)
                        .unwrap_or(usize::MAX);
                    pos1.cmp(pos2)
                });

                Ok(results)
            }
        }
    } else {
        Ok(vec![])
    }
}

async fn run_group_rows(
    context: Arc<TestRunnerContext>,
    group_id: &str,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Vec<ApicizeGroupResultRow>, ApicizeError> {
    let group = context.get_group(group_id)?;
    let child_ids = context.get_group_children(group_id);
    let active_data = match &params.data_set.as_ref() {
        Some(d) => &d.data,
        None => &vec![],
    };

    if child_ids.is_empty() || active_data.is_empty() {
        Ok(vec![])
    } else {
        let mut row_number = 1;
        let mut rows = Vec::<ApicizeGroupResultRow>::with_capacity(active_data.len());

        let mut row_state = RequestExecutionState {
            row: None,
            output_variables: state.output_variables.clone(),
        };

        for row in active_data {
            let row_executed_at = context.ellapsed_in_ms();

            row_state.row = Some(Arc::new(row.clone()));

            let (content, tallies, data_context) = if group.runs == 1 {
                let entries = run_group_children(
                    context.clone(),
                    child_ids,
                    params.clone(),
                    Arc::new(row_state.clone()),
                    &group.execution,
                )
                .await?;
                let tallies = entries.get_tallies();
                let data_context = entries.generate_data_context();
                (
                    ApicizeGroupResultRowContent::Results { results: entries },
                    tallies,
                    data_context,
                )
            } else {
                let runs = run_group_runs(
                    context.clone(),
                    group_id,
                    params.clone(),
                    Arc::new(row_state.clone()),
                )
                .await?;
                let tallies = runs.get_tallies();
                let data_context = runs.generate_data_context();
                (
                    ApicizeGroupResultRowContent::Runs { runs },
                    tallies,
                    data_context,
                )
            };

            if data_context.output_result.is_some() {
                row_state.output_variables = data_context.output_result.clone();
            }

            rows.push(ApicizeGroupResultRow {
                row_number,
                executed_at: row_executed_at,
                duration: context.ellapsed_in_ms() - row_executed_at,
                data_context,
                content,
                success: tallies.success,
                request_success_count: tallies.request_success_count,
                request_failure_count: tallies.request_failure_count,
                request_error_count: tallies.request_error_count,
                test_pass_count: tallies.test_pass_count,
                test_fail_count: tallies.test_fail_count,
            });

            row_number += 1;
        }

        Ok(rows)
    }
}

async fn run_group_runs(
    context: Arc<TestRunnerContext>,
    group_id: &str,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<Vec<ApicizeGroupResultRun>, ApicizeError> {
    let group = context.get_group(group_id)?;
    let number_of_runs = if context.single_run_no_timeout {
        1
    } else {
        group.runs
    };
    let child_ids = context.get_group_children(group_id);
    let mut runs: Vec<ApicizeGroupResultRun> =
        Vec::<ApicizeGroupResultRun>::with_capacity(group.runs);

    match group.multi_run_execution {
        ExecutionConcurrency::Sequential => {
            for run_number in 1..number_of_runs + 1 {
                let run_executed_at = context.ellapsed_in_ms();
                let results = run_group_children(
                    context.clone(),
                    child_ids,
                    params.clone(),
                    state.clone(),
                    &group.execution,
                )
                .await?;

                let tallies = results.get_tallies();
                let data_context = results.generate_data_context();

                runs.push(ApicizeGroupResultRun {
                    run_number,
                    executed_at: run_executed_at,
                    duration: context.ellapsed_in_ms() - run_executed_at,
                    data_context,
                    results,
                    success: tallies.success,
                    request_success_count: tallies.request_success_count,
                    request_failure_count: tallies.request_failure_count,
                    request_error_count: tallies.request_error_count,
                    test_pass_count: tallies.test_pass_count,
                    test_fail_count: tallies.test_fail_count,
                });
            }
        }
        ExecutionConcurrency::Concurrent => {
            let mut executing_runs: JoinSet<Result<ApicizeGroupResultRun, ApicizeError>> =
                JoinSet::new();

            for run_number in 1..number_of_runs + 1 {
                let context = context.clone();
                let child_ids = child_ids.to_vec();
                let params = params.clone();
                let state = state.clone();
                let execution = group.execution.clone();

                let run_executed_at = context.ellapsed_in_ms();

                executing_runs.spawn(async move {
                    select! {
                        _ = context.cancellation.cancelled() => Err(ApicizeError::Cancelled),
                        executed_results = run_group_children(
                            context.clone(),
                            &child_ids,
                            params,
                            state,
                            &execution,
                        ) => {
                            match executed_results {
                                Ok(results) => {
                                    let tallies = results.get_tallies();
                                    Ok(ApicizeGroupResultRun {
                                        run_number,
                                        executed_at: run_executed_at,
                                        duration: context.ellapsed_in_ms() - run_executed_at,
                                        data_context: results.generate_data_context(),
                                        results,
                                        success: tallies.success,
                                        request_success_count: tallies.request_success_count,
                                        request_failure_count: tallies.request_failure_count,
                                        request_error_count: tallies.request_error_count,
                                        test_pass_count: tallies.test_pass_count,
                                        test_fail_count: tallies.test_fail_count,
                                    })
                                },
                                Err(err) => Err(err),
                            }
                        }
                    }
                });
            }
            runs = executing_runs.join_all().await.into_iter().try_fold(
                vec![],
                |mut unrolled, result| -> Result<Vec<ApicizeGroupResultRun>, ApicizeError> {
                    unrolled.push(result?);
                    Ok(unrolled)
                },
            )?;

            runs.sort_by_key(|run| run.run_number);
        }
    }

    Ok(runs)
}

#[async_recursion]
async fn dispatch_request_and_test(
    context: Arc<TestRunnerContext>,
    request_id: String,
    params: Arc<RequestExecutionParameters>,
    state: Arc<RequestExecutionState>,
) -> Result<ApicizeExecution, ApicizeError> {
    let mut execution_request: Option<ApicizeHttpRequest> = None;
    let mut execution_response: Option<ApicizeHttpResponse> = None;
    let mut output_variables: Option<Arc<Map<String, Value>>> = None;
    let mut tests: Option<Vec<ApicizeTestBehavior>> = None;
    let mut error: Option<ApicizeError> = None;

    let name: String;
    let key = context.get_request_key(&request_id)?;

    let request = context.get_request(&request_id)?;

    let mut merged_vars = match &params.variables {
        Some(vars) => (**vars).clone(),
        None => Map::new(),
    };
    if let Some(r) = state.output_variables.as_ref() {
        merged_vars.extend((**r).clone());
    }
    if let Some(r) = state.row.as_ref() {
        merged_vars.extend((**r).clone());
    }

    let merged = match merged_vars.is_empty() {
        true => None,
        false => Some(Arc::new(merged_vars)),
    };

    let subs = match &merged {
        Some(m) => {
            let mut subs = HashMap::with_capacity(m.len());
            for (name, value) in m.iter() {
                let v = if let Some(s) = value.as_str() {
                    s.to_owned()
                } else {
                    value.to_string()
                };
                subs.insert(format!("{{{{{name}}}}}"), v);
            }
            subs
        }
        None => HashMap::new(),
    };

    let mut test_count: usize = 0;
    let mut test_fail_count: usize = 0;
    let mut method: Option<String> = None;
    let url: Option<String>;

    match dispatch_request(context.clone(), &request_id, &params, &subs).await {
        Ok((name_with_subs, url_called, http_request, http_response, _)) => {
            name = name_with_subs;
            url = Some(url_called);
            method = Some(http_request.method.clone());

            execution_request = Some(http_request);
            execution_response = Some(http_response);

            match &request.test {
                Some(t) => {
                    match execute_request_test(
                        RequestEntry::clone_and_sub(t, &subs).as_str(),
                        &execution_request,
                        &execution_response,
                        &params.variables,
                        &state.row,
                        &state.output_variables,
                        &context.tests_started,
                    ) {
                        Ok(test_response) => {
                            (tests, output_variables) = match test_response {
                                Some(response) => {
                                    let output = if response.output.is_empty() {
                                        None
                                    } else {
                                        Some(Arc::new(response.output))
                                    };

                                    // Flatten test nested responses into behaviors
                                    let mut behaviors = Vec::<ApicizeTestBehavior>::new();
                                    flatten_test_results(&response.results, &mut behaviors, &[]);
                                    let test_results = if behaviors.is_empty() {
                                        None
                                    } else {
                                        for b in &behaviors {
                                            test_count += 1;
                                            if !b.success {
                                                test_fail_count += 1;
                                            }
                                        }
                                        Some(behaviors)
                                    };

                                    (test_results, output)
                                }
                                None => (None, None),
                            };
                        }
                        Err(err) => {
                            error = Some(err);
                        }
                    }
                }
                None => {
                    tests = None;
                    output_variables = None;
                }
            }
        }
        Err(err) => {
            name = request.get_name().to_string();
            url = None;
            error = Some(err);
        }
    }

    // If there was a cancellation, return a cancellation error instead of recording the error in the execution
    if let Some(ApicizeError::Cancelled) = error {
        return Err(ApicizeError::Cancelled);
    }

    let success = error.is_none() && (test_count == 0 || test_fail_count == 0);

    Ok(ApicizeExecution {
        name,
        key,
        method,
        url,
        test_context: ApicizeExecutionTestContext {
            merged,
            scenario: params.variables.clone(),
            output: state.output_variables.clone(),
            data: state.row.clone(),
            request: execution_request,
            response: execution_response,
        },
        output_variables,
        tests,
        error,
        success,
        test_pass_count: test_count - test_fail_count,
        test_fail_count,
    })
}

/// Dispatch the specified request (via reqwest), returning either the repsonse or error
async fn dispatch_request(
    context: Arc<TestRunnerContext>,
    request_id: &str,
    params: &RequestExecutionParameters,
    subs: &HashMap<String, String>,
) -> Result<
    (
        String,
        String,
        ApicizeHttpRequest,
        ApicizeHttpResponse,
        Option<Map<String, Value>>,
    ),
    ApicizeError,
> {
    let request = context.get_request(request_id)?;

    let method = match &request.method {
        Some(RequestMethod::Get) => reqwest::Method::GET,
        Some(RequestMethod::Post) => reqwest::Method::POST,
        Some(RequestMethod::Patch) => reqwest::Method::PATCH,
        Some(RequestMethod::Put) => reqwest::Method::PUT,
        Some(RequestMethod::Delete) => reqwest::Method::DELETE,
        Some(RequestMethod::Head) => reqwest::Method::HEAD,
        Some(RequestMethod::Options) => reqwest::Method::OPTIONS,
        None => reqwest::Method::GET,
    };

    let timeout = if context.single_run_no_timeout {
        None
    } else if let Some(t) = request.timeout {
        if t == 0 {
            None
        } else {
            Some(Duration::from_millis(t as u64))
        }
    } else {
        Some(Duration::from_secs(30))
    };

    // Build the reqwest client and request
    let mut reqwest_builder = Client::builder()
        .http2_keep_alive_while_idle(request.keep_alive)
        .danger_accept_invalid_certs(request.accept_invalid_certs)
        .redirect(if request.number_of_redirects == 0 {
            Policy::none()
        } else {
            Policy::limited(request.number_of_redirects)
        })
        .connection_verbose(context.enable_trace);

    if let Some(t) = timeout {
        reqwest_builder = reqwest_builder.timeout(t);
    } else {
        let max = Duration::from_mins(5);
        reqwest_builder = reqwest_builder
            .connect_timeout(max)
            .read_timeout(max)
            .pool_idle_timeout(max);
        #[cfg(target_os = "linux")]
        {
            reqwest_builder = reqwest_builder.tcp_user_timeout(max);
        }
        reqwest_builder = reqwest_builder.http2_keep_alive_timeout(max);
    }

    // Add certificate to builder if configured
    if let Some(certificate) = context
        .workspace
        .certificates
        .get_optional(&params.certificate_id)
    {
        match certificate.append_to_builder(reqwest_builder) {
            Ok(updated_builder) => reqwest_builder = updated_builder,
            Err(err) => return Err(err),
        }
    }

    // Add proxy to builder if configured
    if let Some(proxy) = context.workspace.proxies.get_optional(&params.proxy_id) {
        match proxy.append_to_builder(reqwest_builder) {
            Ok(updated_builder) => reqwest_builder = updated_builder,
            Err(err) => return Err(ApicizeError::from_reqwest(err, None)),
        }
    }

    let builder_result = reqwest_builder.build();
    let mut oauth2_token: Option<TokenResult> = None;

    let name = RequestEntry::clone_and_sub(&request.name, subs);

    let client = builder_result.map_err(|err| ApicizeError::from_reqwest(err, None))?;

    let mut url = RequestEntry::clone_and_sub(request.url.as_str(), subs)
        .trim()
        .to_string();

    if url.is_empty() {
        return Err(ApicizeError::Http {
            context: None,
            description: "Missing URL".to_string(),
            url: None,
        });
    }

    if !(url.starts_with("https://") || url.starts_with("http://")) {
        // If no prefix, check port 443.  If that's responding then assume https
        let mut https = false;
        if let Some(result) = PORT_REGEX.captures(&url)
            && let Some(m) = result.get(1)
            && let Ok(port) = m.as_str().parse::<u32>()
        {
            https = (port % 1000) == 443;
        }

        if !https
            && let Ok(Ok(_)) = tokio::time::timeout(
                Duration::from_secs(2),
                tokio::net::TcpStream::connect(format!("{url}:443")),
            )
            .await
        {
            https = true;
        }
        url = format!("{}://{}", if https { "https" } else { "http" }, url);
    }

    let mut request_builder = client.request(method, &url);

    // Add headers, including authorization if applicable
    let mut headers = match &request.headers {
        Some(h) => {
            let capacity = h.iter().filter(|nvp| nvp.disabled != Some(true)).count();
            reqwest::header::HeaderMap::with_capacity(capacity)
        }
        None => reqwest::header::HeaderMap::new(),
    };

    if let Some(h) = &request.headers {
        for nvp in h {
            if nvp.disabled != Some(true) {
                let name_str = RequestEntry::clone_and_sub(&nvp.name, subs);
                let value_str = RequestEntry::clone_and_sub(&nvp.value, subs);
                headers.insert(
                    reqwest::header::HeaderName::try_from(name_str).unwrap(),
                    reqwest::header::HeaderValue::try_from(value_str).unwrap(),
                );
            }
        }
    }

    match context
        .workspace
        .authorizations
        .get_optional(&params.authorization_id)
    {
        Some(Authorization::Basic {
            username, password, ..
        }) => {
            request_builder = request_builder.basic_auth(username, Some(password));
        }
        Some(Authorization::ApiKey { header, value, .. }) => {
            headers.append(
                reqwest::header::HeaderName::try_from(header).unwrap(),
                reqwest::header::HeaderValue::try_from(value).unwrap(),
            );
        }
        Some(Authorization::OAuth2Client {
            id,
            access_token_url,
            client_id,
            client_secret,
            audience,
            scope,
            send_credentials_in_body,
            selected_certificate,
            selected_proxy,
            ..
        }) => {
            match get_oauth2_client_credentials(
                id.as_str(),
                access_token_url.as_str(),
                client_id.as_str(),
                client_secret.as_str(),
                send_credentials_in_body.unwrap_or(false),
                scope,
                audience,
                context
                    .workspace
                    .certificates
                    .get_optional(&selected_certificate.as_ref().map(|c| c.id.clone())),
                context
                    .workspace
                    .proxies
                    .get_optional(&selected_proxy.as_ref().map(|p| p.id.clone())),
                context.enable_trace,
            )
            .await
            {
                Ok(token_result) => {
                    request_builder = request_builder.bearer_auth(token_result.token.clone());
                    oauth2_token = Some(token_result);
                }
                Err(err) => return Err(err),
            }
        }
        Some(Authorization::OAuth2Pkce { id, .. }) => {
            match retrieve_oauth2_token_from_cache(id).await {
                Some(t) => {
                    request_builder = request_builder.bearer_auth(t.access_token.clone());
                }
                None => {
                    return Err(ApicizeError::Error {
                        description: String::from("PKCE access token is not available"),
                    });
                }
            }
        }
        None => {}
    }

    if !headers.is_empty() {
        request_builder = request_builder.headers(headers);
    }

    // Add query string parameters, if applicable
    if let Some(q) = &request.query_string_params {
        let mut query: Vec<(String, String)> = vec![];
        for nvp in q {
            if nvp.disabled != Some(true) {
                query.push((
                    RequestEntry::clone_and_sub(&nvp.name, subs),
                    RequestEntry::clone_and_sub(&nvp.value, subs),
                ));
            }
        }
        request_builder = request_builder.query(&query);
    }

    // Add body, if applicable -xxxx
    let mut request_body: Option<ApicizeBody>;
    match &request.body {
        Some(RequestBody::Text { data }) => {
            let s = RequestEntry::clone_and_sub(data, subs);
            request_body = Some(ApicizeBody::Text {
                text: "".to_string(),
            });
            request_builder = request_builder.body(Body::from(s.clone()));
        }
        Some(RequestBody::JSON { data, .. }) => {
            let escaped_subs = subs
                .iter()
                .map(|(n, v)| (n.to_string(), v.replace("\"", "\\\"")))
                .collect::<HashMap<String, String>>();

            let s = RequestEntry::clone_and_sub(data, &escaped_subs);
            request_body = match serde_json::from_str::<Value>(&s) {
                Ok(data) => Some(ApicizeBody::JSON {
                    text: "".to_string(),
                    data,
                }),
                Err(_) => Some(ApicizeBody::Text { text: s.clone() }),
            };
            request_builder = request_builder.body(Body::from(s));
        }
        Some(RequestBody::XML { data }) => {
            let s = RequestEntry::clone_and_sub(data, subs);
            request_body = match to_json(data) {
                Ok(data) => Some(ApicizeBody::XML {
                    text: "".to_string(),
                    data,
                }),
                Err(_) => Some(ApicizeBody::Text { text: s.clone() }),
            };
            request_builder = request_builder.body(Body::from(s));
        }
        Some(RequestBody::Form { data }) => {
            let form_data = data
                .iter()
                .map(|pair| {
                    (
                        RequestEntry::clone_and_sub(&pair.name, subs),
                        RequestEntry::clone_and_sub(&pair.value, subs),
                    )
                })
                .collect::<HashMap<String, String>>();
            request_body = Some(ApicizeBody::Form {
                text: "".to_string(),
                data: form_data.clone(),
            });
            request_builder = request_builder.form(&form_data);
        }
        Some(RequestBody::Raw { data }) => {
            request_body = Some(ApicizeBody::Binary { data: data.clone() });
            request_builder = request_builder.body(Body::from(data.clone()));
        }
        None => {
            request_body = None;
        }
    }

    let mut web_request = request_builder
        .build()
        .map_err(|err| ApicizeError::from_reqwest(err, None))?;
    // Copy value generated for the request so that we can include in the function results
    let request_url = web_request.url().to_string();
    let request_headers = web_request
        .headers()
        .iter()
        .map(|(h, v)| {
            (
                String::from(h.as_str()),
                String::from(v.to_str().unwrap_or("(Header Contains Non-ASCII Data)")),
            )
        })
        .collect::<HashMap<String, String>>();
    let ref_body = web_request.body_mut();
    if let Some(data) = ref_body {
        let bytes = data.as_bytes().unwrap();
        if !bytes.is_empty() {
            let request_encoding = UTF_8;
            let data = bytes.to_vec();
            match request_body.as_mut() {
                None => {}
                Some(ApicizeBody::Binary { .. }) => {}
                Some(ApicizeBody::Form { text, .. })
                | Some(ApicizeBody::JSON { text, .. })
                | Some(ApicizeBody::XML { text, .. })
                | Some(ApicizeBody::Text { text, .. }) => {
                    let (decoded, _, malformed) = request_encoding.decode(&data);
                    *text = if malformed {
                        "Malformed UTF8".to_string()
                    } else {
                        decoded.to_string()
                    }
                }
            }
        }
    }

    let client_response: Result<Response, ApicizeError> = select! {
        _ = context.cancellation.cancelled() => Err(ApicizeError::Cancelled),
        result = client.execute(web_request) => {
            match result {
                Ok(response) => Ok(response),
                Err(error) => Err(ApicizeError::from_reqwest(error, None)),
            }
        }
    };

    // Execute the request
    match client_response {
        Err(error) => Err(error),
        Ok(response) => {
            // Collect headers for response
            let response_headers = response.headers();
            let mut may_have_json = false;
            let headers = if response_headers.is_empty() {
                None
            } else {
                Some(HashMap::from_iter(
                    response_headers
                        .iter()
                        .map(|(h, v)| {
                            let name = h.as_str();
                            let value = v.to_str().unwrap_or("(Header Contains Non-ASCII Data)");
                            if name.eq_ignore_ascii_case("content-type")
                                && value.to_ascii_lowercase().contains("json")
                            {
                                may_have_json = true;
                            }
                            (name.to_string(), value.to_string())
                        })
                        .collect::<HashMap<String, String>>(),
                ))
            };

            // Determine the default text encoding
            let response_content_type = response_headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<Mime>().ok());

            let response_encoding_name = response_content_type
                .as_ref()
                .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
                .unwrap_or("utf-8");

            let response_encoding =
                Encoding::for_label(response_encoding_name.as_bytes()).unwrap_or(UTF_8);

            // Collect status for response
            let status = response.status();
            let status_text = String::from(status.canonical_reason().unwrap_or("Unknown"));

            // Retrieve response bytes and convert raw data to string
            match response.bytes().await {
                Ok(bytes) => {
                    let mut output_variables: Option<Map<String, Value>> = None;
                    let response_body = if bytes.is_empty() {
                        None
                    } else {
                        let data = Vec::from(bytes.as_ref());
                        let (decoded, _, malformed) = response_encoding.decode(&data);

                        if malformed {
                            Some(ApicizeBody::Binary { data })
                        } else {
                            let text = decoded.to_string();
                            if may_have_json {
                                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                                    if let Some(obj) = parsed.as_object() {
                                        output_variables = Some(obj.clone());
                                    }

                                    Some(ApicizeBody::JSON {
                                        text: decoded.to_string(),
                                        data: parsed,
                                    })
                                } else {
                                    Some(ApicizeBody::Text { text })
                                }
                            } else {
                                Some(ApicizeBody::Text { text })
                            }
                        }
                    };

                    let response = (
                        name,
                        url,
                        ApicizeHttpRequest {
                            url: request_url,
                            method: request
                                .method
                                .as_ref()
                                .unwrap_or(&RequestMethod::Get)
                                .as_str()
                                .to_string(),
                            headers: request_headers,
                            body: request_body,
                        },
                        ApicizeHttpResponse {
                            status: status.as_u16(),
                            status_text,
                            headers,
                            body: response_body,
                            oauth2_token,
                        },
                        output_variables,
                    );

                    Ok(response)
                }
                Err(err) => Err(ApicizeError::from_reqwest(err, None)),
            }
        }
    }
}

/// Execute the specified request's tests
fn execute_request_test(
    test: &str,
    request: &Option<ApicizeHttpRequest>,
    response: &Option<ApicizeHttpResponse>,
    variables: &Option<Arc<Map<String, Value>>>,
    data: &Option<Arc<Map<String, Value>>>,
    output: &Option<Arc<Map<String, Value>>>,
    tests_started: &Instant,
) -> Result<Option<ApicizeTestResponse>, ApicizeError> {
    // Ensure V8 is initialized
    V8_INIT.call_once(|| {
        let platform = v8::new_unprotected_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });

    // Create a new Isolate and make it the current one.
    let isolate = &mut v8::Isolate::new(Default::default());
    // let scope = &mut v8::HandleScope::new(isolate);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // Use the included framework directly without extra allocation
    let framework_code = include_str!(concat!(env!("OUT_DIR"), "/framework.min.js"));
    let v8_code = v8::String::new(scope, framework_code).unwrap();
    let script = v8::Script::compile(scope, v8_code, None).unwrap();
    script.run(scope).unwrap();

    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = scope.init();

    let cloned_tests_started = tests_started;

    let init_code = format!(
        "runTestSuite({}, {}, {}, {}, {}, {}, () => {{{}}})",
        serde_json::to_string(request).unwrap(),
        serde_json::to_string(response).unwrap(),
        serde_json::to_string(&variables).unwrap(),
        serde_json::to_string(&data).unwrap(),
        serde_json::to_string(&output).unwrap(),
        std::time::UNIX_EPOCH.elapsed().unwrap().as_millis()
            - cloned_tests_started.elapsed().as_millis()
            + 1,
        test,
    );

    let v8_code = v8::String::new(&scope, &init_code).unwrap();

    let Some(script) = v8::Script::compile(&scope, v8_code, None) else {
        let message = scope.message().unwrap();
        let message = message.get(&scope).to_rust_string_lossy(&scope);
        return Err(ApicizeError::from_failed_test(message));
    };

    let Some(value) = script.run(&scope) else {
        let message = scope.message().unwrap();
        let message = message.get(&scope).to_rust_string_lossy(&scope);
        return Err(ApicizeError::from_failed_test(message));
    };

    let result = value.to_string(&scope);
    let s = result.unwrap().to_rust_string_lossy(&scope);

    let test_response: ApicizeTestResponse = serde_json::from_str(&s).unwrap();

    Ok(Some(test_response))
}

fn flatten_test_results(
    results: &Option<Vec<ApicizeTestResult>>,
    behaviors: &mut Vec<ApicizeTestBehavior>,
    parents: &[String],
) {
    if let Some(r) = results {
        for result in r {
            match &result {
                ApicizeTestResult::Scenario(scenario) => {
                    if scenario.children.is_some() {
                        let mut new_parents = Vec::with_capacity(parents.len() + 1);
                        new_parents.extend_from_slice(parents);
                        new_parents.push(scenario.name.clone());
                        flatten_test_results(&scenario.children, behaviors, &new_parents);
                    }
                }
                ApicizeTestResult::Behavior(behavior) => {
                    let mut name = Vec::with_capacity(parents.len() + 1);
                    name.extend_from_slice(parents);
                    name.push(behavior.name.clone());
                    behaviors.push(ApicizeTestBehavior {
                        name: name.join(" "),
                        tag: behavior.tag.clone(),
                        success: behavior.success,
                        error: behavior.error.clone(),
                        logs: behavior.logs.clone(),
                    });
                }
            }
        }
    }
}

/// Cleanup V8 platform, should only be called once at end of application
pub fn cleanup_v8() {
    unsafe {
        v8::V8::dispose();
    }
    v8::V8::dispose_platform();
}

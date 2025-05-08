//! Apicize test execution.
//!
//! This library supports dispatching Apicize functional web tests
use regex::Regex;
use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use async_recursion::async_recursion;
use encoding_rs::{Encoding, UTF_8};
use mime::Mime;
use reqwest::{Body, Client};
use serde_json::{Map, Value};
use tokio::select;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::{
    ApicizeBody, ApicizeExecution, ApicizeExecutionType, ApicizeGroup, ApicizeGroupChildren,
    ApicizeGroupRun, ApicizeHttpRequest, ApicizeHttpResponse, ApicizeList, ApicizeRequest,
    ApicizeResult, ApicizeRow, ApicizeRowSummary, ApicizeTestResponse, ApicizeTestResult,
    OutputVariables, Tally,
};
use crate::oauth2_client_tokens::TokenResult;
use crate::types::workspace::RequestParameters;
use crate::{
    ApicizeError, ApicizeGroupItem, Authorization, ExecutionConcurrency, Request, RequestBody,
    RequestEntry, RequestGroup, RequestMethod, VariableCache, Workspace,
};

#[cfg(test)]
use crate::oauth2_client_tokens::tests::MockOAuth2ClientTokens as oauth2;

#[cfg(not(test))]
use crate::oauth2_client_tokens as oauth2;

static V8_INIT: Once = Once::new();

pub trait ApicizeRunner {
    fn run(
        &self,
        request_ids: &[String],
    ) -> impl std::future::Future<Output = Result<ApicizeResult, ApicizeError>> + Send;
}

pub struct TestRunnerContext {
    workspace: Workspace,
    cancellation: CancellationToken,
    value_cache: Mutex<VariableCache>,
    tests_started: Instant,
    override_number_of_runs: Option<usize>,
    enable_trace: bool,
}

impl TestRunnerContext {
    pub fn new(
        workspace: Workspace,
        cancellation: Option<CancellationToken>,
        override_number_of_runs: Option<usize>,
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
            value_cache: Mutex::new(VariableCache::new(allowed_data_path)),
            tests_started: Instant::now(),
            override_number_of_runs,
            enable_trace,
        }
    }
}

impl ApicizeRunner for Arc<TestRunnerContext> {
    /// Dispatch requests/groups in the specified workspace
    async fn run(&self, request_ids: &[String]) -> Result<ApicizeResult, ApicizeError> {
        let row_count = match &self.workspace.defaults.selected_data {
            Some(selected_data) => match self
                .workspace
                .data
                .iter()
                .find(|d| d.id == selected_data.id)
            {
                Some(data) => {
                    let mut locked_cache = self.value_cache.lock().unwrap();
                    match locked_cache.get_external_data(data) {
                        Ok(data) => data.len(),
                        Err(err) => return Err(err.clone()),
                    }
                }
                _ => 0,
            },
            None => 0,
        };

        if row_count == 0 {
            let mut executing_items: JoinSet<Result<ApicizeGroupItem, ApicizeError>> =
                JoinSet::new();

            for request_id in request_ids {
                let cloned_context = self.clone();
                let cloned_id = request_id.clone();

                executing_items.spawn(async move {
                    select! {
                        _ = cloned_context.cancellation.cancelled() => Err(ApicizeError::Cancelled {
                            source: None
                        }),
                        result = run_request_item(
                            cloned_context.clone(),
                            cloned_id,
                            None,
                            None,
                        ) => {
                            result
                        }
                    }
                });
            }

            let completed_items = executing_items.join_all().await;
            let mut items = Vec::<ApicizeGroupItem>::with_capacity(completed_items.len());

            for completed_item in completed_items {
                match completed_item {
                    Ok(item) => {
                        items.push(item);
                    }
                    Err(err) => {
                        return Err(err);
                    }
                }
            }

            Ok(ApicizeResult::Items(ApicizeList { items }))
        } else {
            let mut rows = Vec::<ApicizeRow>::with_capacity(row_count);
            let started_at = Instant::now();
            let executed_at = self.tests_started.elapsed().as_millis();

            for row_number in 1..=row_count {
                let row_started_at = Instant::now();
                let row_executed_at = self.tests_started.elapsed().as_millis();

                let mut executing_items: JoinSet<Result<ApicizeGroupItem, ApicizeError>> =
                    JoinSet::new();

                for request_id in request_ids {
                    let cloned_context = self.clone();
                    let cloned_id = request_id.clone();

                    executing_items.spawn(async move {
                        select! {
                            _ = cloned_context.cancellation.cancelled() => Err(ApicizeError::Cancelled {
                                source: None
                            }),
                            result = run_request_item(
                                cloned_context.clone(),
                                cloned_id,
                                Some(row_number),
                                None,
                            ) => {
                                result
                            }
                        }
                    });
                }

                let completed_items = executing_items.join_all().await;
                let mut items = Vec::<ApicizeGroupItem>::with_capacity(completed_items.len());

                for completed_item in completed_items {
                    match completed_item {
                        Ok(item) => {
                            items.push(item);
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }

                let row_tallies = items.get_tallies();
                rows.push(ApicizeRow {
                    row_number,
                    items,
                    executed_at: row_executed_at,
                    duration: row_started_at.elapsed().as_millis(),
                    success: row_tallies.success,
                    request_success_count: row_tallies.request_success_count,
                    request_failure_count: row_tallies.request_failure_count,
                    request_error_count: row_tallies.request_error_count,
                    test_pass_count: row_tallies.test_pass_count,
                    test_fail_count: row_tallies.test_fail_count,
                })
            }

            let tallies = rows.get_tallies();

            Ok(ApicizeResult::Rows(ApicizeRowSummary {
                rows,
                executed_at,
                duration: started_at.elapsed().as_millis(),
                success: tallies.success,
                request_success_count: tallies.request_success_count,
                request_failure_count: tallies.request_failure_count,
                request_error_count: tallies.request_error_count,
                test_pass_count: tallies.test_pass_count,
                test_fail_count: tallies.test_fail_count,
            }))
        }
    }
}

// async fn launch_run_item(
//     context: Arc<TestRunnerContext>,
//     id: String,
// ) -> Result<ApicizeGroupItem, ApicizeError> {
//     match context.workspace.requests.get(&id) {
//         Some(entry) => {
//             let params = Arc::new(
//                 context
//                     .workspace
//                     .retrieve_request_parameters(entry, &context.value_cache)?,
//             );

//             match &params.data {
//                 Some(data) => {
//                     let row_count = data.len();
//                     let mut rows = Vec::<ApicizeGroupItem>::with_capacity(row_count);

//                     let started_at = Instant::now();
//                     let executed_at = context.tests_started.elapsed().as_millis();

//                     for row_number in 1..=row_count {
//                         match run_request_item(
//                             context.clone(),
//                             Arc::new(entry),
//                             params.clone(),
//                             Some(row_number),
//                         )
//                         .await
//                         {
//                             Ok(item) => {
//                                 rows.push(item);
//                             }
//                             Err(err) => return Err(err),
//                         }
//                     }

//                     let tallies = rows.get_tallies();

//                     Ok(ApicizeResult::Rows(ApicizeRowSummary {
//                         executed_at,
//                         duration: started_at.elapsed().as_millis(),
//                         rows,
//                         success: tallies.success,
//                         request_success_count: tallies.request_success_count,
//                         request_failure_count: tallies.request_failure_count,
//                         request_error_count: tallies.request_error_count,
//                         test_pass_count: tallies.test_pass_count,
//                         test_fail_count: tallies.test_fail_count,
//                     }))
//                 }
//                 None => {
//                     match run_request_item(context.clone(), Arc::new(entry), params.clone(), None)
//                         .await
//                     {
//                         Ok(item) => match item {
//                             ApicizeGroupItem::Group(group) => Ok(ApicizeResult::Group(group)),
//                             ApicizeGroupItem::Request(request) => {
//                                 Ok(ApicizeResult::Request(request))
//                             }
//                         },
//                         Err(err) => return Err(err),
//                     }
//                 }
//             }
//         }
//         None => Err(ApicizeError::Error {
//             description: format!("Invalid request ID \"{}\"", id),
//             source: None,
//         }),
//     }
// }

#[async_recursion]
async fn run_request_item(
    context: Arc<TestRunnerContext>,
    id: String,
    row_number: Option<usize>,
    active_variables: Option<Arc<Map<String, Value>>>,
) -> Result<ApicizeGroupItem, ApicizeError> {
    match context.workspace.requests.get(&id) {
        Some(entry) => {
            let params = Arc::new(context.workspace.retrieve_request_parameters(
                entry,
                &context.value_cache,
                active_variables,
            )?);

            match entry {
                RequestEntry::Request(request) => {
                    return run_request(
                        context.clone(),
                        Arc::new(request.clone()),
                        params.clone(),
                        row_number,
                    )
                    .await
                }
                RequestEntry::Group(group) => {
                    return run_group(
                        context.clone(),
                        Arc::new(group.clone()),
                        params.clone(),
                        row_number,
                    )
                    .await
                }
            }
        }
        None => {
            return Err(ApicizeError::Error {
                description: format!("Invalid request ID \"{}\"", id),
                source: None,
            })
        }
    }
}

/// Dispatch a request and execute its tests, optionally for a specific data row
async fn run_request(
    context: Arc<TestRunnerContext>,
    request: Arc<Request>,
    params: Arc<RequestParameters>,
    row_number: Option<usize>,
) -> Result<ApicizeGroupItem, ApicizeError> {
    let number_of_runs = context.override_number_of_runs.unwrap_or(request.runs);
    let started_at = Instant::now();
    let executed_at = context.tests_started.elapsed().as_millis();

    match number_of_runs {
        0 => Ok(ApicizeGroupItem::Request(Box::new(ApicizeRequest {
            id: request.id.clone(),
            name: request.name.clone(),
            row_number,
            executed_at,
            duration: 0,
            input_variables: None,
            output_variables: None,
            execution: super::ApicizeExecutionType::None,
            success: false,
            request_success_count: 0,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }))),
        1 => {
            let execution = dispatch_request_and_test(
                context.clone(),
                request.clone(),
                params.clone(),
                row_number,
                row_number,
            )
            .await;

            let mut response = ApicizeRequest {
                id: request.id.clone(),
                name: request.name.clone(),
                row_number,
                executed_at,
                duration: started_at.elapsed().as_millis(),
                execution: ApicizeExecutionType::None,
                input_variables: execution.input_variables.clone(),
                output_variables: execution.output_variables.clone(),
                success: execution.success,
                request_success_count: if execution.success { 1 } else { 0 },
                request_failure_count: if execution.success {
                    0
                } else if execution.error.is_none() {
                    1
                } else {
                    0
                },
                request_error_count: if execution.success {
                    0
                } else if execution.error.is_some() {
                    1
                } else {
                    0
                },
                test_pass_count: execution.test_pass_count,
                test_fail_count: execution.test_fail_count,
            };
            response.execution = ApicizeExecutionType::Single(execution);
            Ok(ApicizeGroupItem::Request(Box::new(response)))
        }
        _ => {
            let mut runs =
                Vec::<Result<ApicizeExecution, ApicizeError>>::with_capacity(number_of_runs);
            match request.multi_run_execution {
                ExecutionConcurrency::Sequential => {
                    for run_number in 1..=number_of_runs {
                        runs.push(Ok(dispatch_request_and_test(
                            context.clone(),
                            request.clone(),
                            params.clone(),
                            Some(run_number),
                            row_number,
                        )
                        .await));
                    }
                }
                ExecutionConcurrency::Concurrent => {
                    let mut spawned_runs: JoinSet<Result<ApicizeExecution, ApicizeError>> =
                        JoinSet::new();

                    for run_number in 1..=number_of_runs {
                        let ccl = context.cancellation.clone();
                        let ctx = context.clone();
                        let req = request.clone();
                        let prm = params.clone();

                        spawned_runs.spawn(async move {
                            select! {
                                _ = ccl.cancelled() => Err(ApicizeError::Cancelled {
                                    source: None
                                }),
                                result =  dispatch_request_and_test(ctx.clone(), req, prm, Some(run_number), row_number) => {
                                    Ok(result)
                                }
                            }
                        });
                    }

                    for spawned_run in spawned_runs.join_all().await {
                        match spawned_run {
                            Ok(run) => runs.push(Ok(run)),
                            Err(err) => return Err(err),
                        }
                    }
                }
            }

            let mut result_runs = Vec::<ApicizeExecution>::with_capacity(number_of_runs);
            for run in runs {
                match run {
                    Ok(r) => result_runs.push(r),
                    Err(err) => return Err(err),
                }
            }

            let tallies = result_runs.get_tallies();
            let output_variables = result_runs.get_output_variables();

            Ok(ApicizeGroupItem::Request(Box::new(ApicizeRequest {
                id: request.id.clone(),
                name: request.name.clone(),
                row_number,
                executed_at,
                duration: started_at.elapsed().as_millis(),
                execution: ApicizeExecutionType::Runs(ApicizeList { items: result_runs }),
                input_variables: None,
                output_variables,
                success: tallies.success,
                request_success_count: tallies.request_success_count,
                request_failure_count: tallies.request_failure_count,
                request_error_count: tallies.request_error_count,
                test_pass_count: tallies.test_pass_count,
                test_fail_count: tallies.test_fail_count,
            })))
        }
    }
}

async fn run_group_iteration(
    context: Arc<TestRunnerContext>,
    group: Arc<RequestGroup>,
    params: Arc<RequestParameters>,
    run_number: usize,
    row_number: Option<usize>,
) -> Result<ApicizeGroupRun, ApicizeError> {
    let run_executed_at = context.tests_started.elapsed().as_millis();
    let run_started_at = Instant::now();

    // We have to copy child IDs here because context goes out of scope when we spawn
    let child_ids = context.workspace.requests.child_ids.get(&group.id).cloned();

    let run_results = match child_ids {
        Some(child_ids) => {
            let mut results: Vec<Result<ApicizeGroupItem, ApicizeError>>;

            // if params.data.map_or(false, |d| !d.is_empty()) {
            //     Vec::new()
            // } else {
            match &group.execution {
                ExecutionConcurrency::Sequential => {
                    let mut active_vars =
                        params.variables.as_ref().map(|vars| Arc::new(vars.clone()));

                    results = Vec::with_capacity(child_ids.len());
                    for child_id in &child_ids {
                        let c = run_request_item(
                            context.clone(),
                            child_id.clone(),
                            row_number,
                            active_vars.clone(),
                        )
                        .await;
                        if let Ok(c_ok) = &c {
                            active_vars = c_ok.get_output_variables().map(Arc::new);
                        }
                        results.push(c);
                    }
                    results
                }
                ExecutionConcurrency::Concurrent => {
                    let mut spawned_items: JoinSet<Result<ApicizeGroupItem, ApicizeError>> =
                        JoinSet::new();

                    let active_vars = params.variables.as_ref().map(|vars| Arc::new(vars.clone()));

                    for child_id in child_ids {
                        let ccl = context.cancellation.clone();
                        let ctx = context.clone();
                        let vars = active_vars.clone();

                        spawned_items.spawn(async move {
                            select! {
                                _ = ccl.cancelled() => Err(ApicizeError::Cancelled {
                                    source: None
                                }),
                                result = run_request_item(
                                    ctx.clone(),
                                    child_id.clone(),
                                    row_number,
                                    vars,
                                ) => {
                                    result
                                }
                            }
                        });
                    }

                    (spawned_items.join_all().await).into_iter().collect()
                }
            }
            // }
        }
        None => Vec::new(),
    };

    let mut run_items = Vec::with_capacity(run_results.len());
    for run_result in run_results {
        match run_result {
            Ok(item) => run_items.push(item),
            Err(err) => return Err(err),
        }
    }

    let run_tallies = run_items.get_tallies();
    let output_variables = run_items.get_output_variables();

    Ok(ApicizeGroupRun {
        run_number,
        children: run_items,
        executed_at: run_executed_at,
        duration: run_started_at.elapsed().as_millis(),
        output_variables,
        success: run_tallies.success,
        request_success_count: run_tallies.request_success_count,
        request_failure_count: run_tallies.request_failure_count,
        request_error_count: run_tallies.request_error_count,
        test_pass_count: run_tallies.test_pass_count,
        test_fail_count: run_tallies.test_fail_count,
    })
}

/// Dispatch the request and execute its tests
#[async_recursion]
async fn run_group(
    context: Arc<TestRunnerContext>,
    group: Arc<RequestGroup>,
    params: Arc<RequestParameters>,
    row_number: Option<usize>,
) -> Result<ApicizeGroupItem, ApicizeError> {
    let number_of_runs = context.override_number_of_runs.unwrap_or(group.runs);
    let started_at = Instant::now();
    let executed_at = context.tests_started.elapsed().as_millis();

    match number_of_runs {
        0 => Ok(ApicizeGroupItem::Group(Box::new(ApicizeGroup {
            id: group.id.clone(),
            name: group.name.clone(),
            row_number,
            executed_at,
            duration: 0,
            output_variables: None,
            children: None,
            success: false,
            request_success_count: 0,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }))),
        1 => match run_group_iteration(
            context.clone(),
            group.clone(),
            params.clone(),
            1,
            row_number,
        )
        .await
        {
            Ok(run) => Ok(ApicizeGroupItem::Group(Box::new(ApicizeGroup {
                id: group.id.clone(),
                name: group.name.clone(),
                row_number,
                executed_at,
                duration: started_at.elapsed().as_millis(),
                output_variables: run.get_output_variables(),
                children: Some(ApicizeGroupChildren::Items(ApicizeList {
                    items: run.children,
                })),
                success: run.success,
                request_success_count: run.request_success_count,
                request_failure_count: run.request_failure_count,
                request_error_count: run.request_error_count,
                test_pass_count: run.test_pass_count,
                test_fail_count: run.test_fail_count,
            }))),
            Err(err) => Err(err),
        },
        _ => {
            let mut runs = Vec::with_capacity(number_of_runs);

            match &group.multi_run_execution {
                ExecutionConcurrency::Sequential => {
                    for run_number in 1..=number_of_runs {
                        match run_group_iteration(
                            context.clone(),
                            group.clone(),
                            params.clone(),
                            run_number,
                            row_number,
                        )
                        .await
                        {
                            Ok(run) => runs.push(run),
                            Err(err) => return Err(err),
                        }
                    }
                }
                ExecutionConcurrency::Concurrent => {
                    let mut spawned_runs: JoinSet<Result<ApicizeGroupRun, ApicizeError>> =
                        JoinSet::new();

                    for run_number in 1..=number_of_runs {
                        let cc = context.clone();
                        let cg = group.clone();
                        let cp = params.clone();
                        let ccl = context.cancellation.clone();

                        spawned_runs.spawn(async move {
                        select! {
                            _ = ccl.cancelled() => Err(ApicizeError::Cancelled {
                                source: None
                            }),
                            result = run_group_iteration(cc.clone(), cg.clone(), cp.clone(), run_number, row_number) => {
                                result
                            }
                        }
                    });
                    }

                    for spawned_run in spawned_runs.join_all().await {
                        match spawned_run {
                            Ok(run) => runs.push(run),
                            Err(err) => return Err(err),
                        }
                    }

                    runs.sort_by_key(|r| r.run_number);
                }
            }

            let tallies = runs.get_tallies();
            let output_variables = runs.get_output_variables();

            Ok(ApicizeGroupItem::Group(Box::new(ApicizeGroup {
                id: group.id.clone(),
                name: group.name.clone(),
                row_number,
                executed_at,
                duration: started_at.elapsed().as_millis(),
                children: Some(ApicizeGroupChildren::Runs(ApicizeList { items: runs })),
                output_variables,
                success: tallies.success,
                request_success_count: tallies.request_success_count,
                request_failure_count: tallies.request_failure_count,
                request_error_count: tallies.request_error_count,
                test_pass_count: tallies.test_pass_count,
                test_fail_count: tallies.test_fail_count,
            })))
        }
    }
}

#[async_recursion]
async fn dispatch_request_and_test(
    context: Arc<TestRunnerContext>,
    request: Arc<Request>,
    params: Arc<RequestParameters>,
    run_number: Option<usize>,
    row_number: Option<usize>,
) -> ApicizeExecution {
    let mut data: Option<Map<String, Value>> = None;
    let mut execution_request: Option<ApicizeHttpRequest> = None;
    let mut execution_response: Option<ApicizeHttpResponse> = None;
    let mut execution_variables: Option<Map<String, Value>> = None;
    let mut output_variables: Option<Map<String, Value>> = None;
    let mut tests: Option<Vec<ApicizeTestResult>> = None;
    let mut test_count = 0;
    let mut test_fail_count = 0;
    let mut error: Option<ApicizeError> = None;

    let started_at = Instant::now();
    let executed_at = context.tests_started.elapsed().as_millis();

    match dispatch_request(context.clone(), &request, &params, &row_number).await {
        Ok((http_request, http_response, variables)) => {
            // url = Some(http_request.url);
            // method = Some(http_request.method);
            // headers = Some(http_request.headers);
            // body = http_request.body;

            // Get row seed data, if applicable
            if let Some(active_data) = &params.data {
                if let Some(n) = row_number {
                    match active_data.get(n - 1) {
                        Some(row) => data = Some(row.clone()),
                        None => {
                            error = Some(ApicizeError::Error {
                                description: "Invalid data row index".to_string(),
                                source: None,
                            });
                        }
                    }
                }
            }

            match execute_request_test(
                &request.test,
                &http_request,
                &http_response,
                &params.variables,
                &data,
                &context.tests_started,
            ) {
                Ok(test_response) => {
                    tests = match test_response {
                        Some(response) => {
                            output_variables = Some(response.variables.clone());
                            if let Some(test_result) = &response.results {
                                test_count = test_result.len();
                                test_fail_count +=
                                    test_result.iter().filter(|r| !r.success).count();
                            }
                            response.results
                        }
                        None => {
                            output_variables = None;
                            None
                        }
                    };
                }
                Err(err) => {
                    error = Some(err);
                }
            }

            execution_request = Some(http_request);
            execution_response = Some(http_response);
            execution_variables = variables;
        }
        Err(err) => {
            error = Some(err);
        }
    }

    let duration = started_at.elapsed().as_millis();
    let success = error.is_none() && (test_count == 0 || test_fail_count == 0);

    ApicizeExecution {
        row_number,
        run_number,
        executed_at,
        duration,
        input_variables: execution_variables,
        data,
        output_variables,
        request: execution_request,
        response: execution_response,
        tests,
        error,
        success,
        test_pass_count: test_count - test_fail_count,
        test_fail_count,
    }
}

/// Dispatch the specified request (via reqwest), returning either the repsonse or error
async fn dispatch_request(
    context: Arc<TestRunnerContext>,
    request: &Request,
    params: &RequestParameters,
    row_number: &Option<usize>,
) -> Result<
    (
        ApicizeHttpRequest,
        ApicizeHttpResponse,
        Option<Map<String, Value>>,
    ),
    ApicizeError,
> {
    let method = match request.method {
        Some(RequestMethod::Get) => reqwest::Method::GET,
        Some(RequestMethod::Post) => reqwest::Method::POST,
        Some(RequestMethod::Put) => reqwest::Method::PUT,
        Some(RequestMethod::Delete) => reqwest::Method::DELETE,
        Some(RequestMethod::Head) => reqwest::Method::HEAD,
        Some(RequestMethod::Options) => reqwest::Method::OPTIONS,
        None => reqwest::Method::GET,
        _ => panic!("Invalid method"),
    };

    let timeout: Duration;
    if let Some(t) = request.timeout {
        timeout = if t == 0 {
            Duration::MAX
        } else {
            Duration::from_millis(t as u64)
        }
    } else {
        timeout = Duration::from_secs(30);
    }

    // let keep_alive: bool;
    // if let Some(b) = request.keep_alive {
    //     keep_alive = b;
    // } else {
    //     keep_alive = true;
    // }

    let mut subs: HashMap<String, String> = match &params.variables {
        Some(variables) => HashMap::from_iter(variables.iter().map(|(name, value)| {
            let v = if let Some(s) = value.as_str() {
                String::from(s)
            } else {
                format!("{}", value)
            };
            (format!("{{{{{}}}}}", name), v)
        })),
        None => HashMap::new(),
    };

    if let Some(row) = row_number {
        if let Some(data) = &params.data {
            match data.get(*row - 1) {
                Some(row_data) => {
                    for (name, value) in row_data {
                        let v = if let Some(s) = value.as_str() {
                            String::from(s)
                        } else {
                            format!("{}", value)
                        };
                        subs.insert(format!("{{{{{}}}}}", name), v);
                    }
                }
                None => {
                    return Err(ApicizeError::Error {
                        description: format!("Invalid row #{}", row),
                        source: None,
                    })
                }
            }
        }
    }

    // Build the reqwest client and request
    let mut reqwest_builder = Client::builder()
        // .http2_keep_alive_while_idle(keep_alive)
        .timeout(timeout)
        .connection_verbose(context.enable_trace);

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
            Err(err) => return Err(ApicizeError::from_reqwest(err)),
        }
    }

    let builder_result = reqwest_builder.build();
    let mut oauth2_token: Option<TokenResult> = None;

    match builder_result {
        Ok(client) => {
            let mut url = RequestEntry::clone_and_sub(request.url.as_str(), &subs)
                .trim()
                .to_string();
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                // If no prefix, check port 443.  If that's responding then assume https
                let mut https = false;
                let regex = Regex::new(r".+:(\d{1,5})(?:\?.*)?$").unwrap();
                if let Some(result) = regex.captures(&url) {
                    if let Some(m) = result.get(1) {
                        if let Ok(port) = m.as_str().parse::<u32>() {
                            https = (port % 1000) == 443;
                        }
                    }
                }

                if !https {
                    if let Ok(addrs) = &mut format!("{}:443", url).to_socket_addrs() {
                        if let Some(addr) = addrs.next() {
                            https =
                                TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok();
                        }
                    }
                }
                url = format!("{}://{}", if https { "https" } else { "http" }, url);
            }

            let mut request_builder = client.request(method, url);

            // Add headers, including authorization if applicable
            let mut headers = reqwest::header::HeaderMap::new();
            if let Some(h) = &request.headers {
                for nvp in h {
                    if nvp.disabled != Some(true) {
                        headers.insert(
                            reqwest::header::HeaderName::try_from(RequestEntry::clone_and_sub(
                                &nvp.name, &subs,
                            ))
                            .unwrap(),
                            reqwest::header::HeaderValue::try_from(RequestEntry::clone_and_sub(
                                &nvp.value, &subs,
                            ))
                            .unwrap(),
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
                    ..
                }) => {
                    match oauth2::get_oauth2_client_credentials(
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
                            .get_optional(&params.auth_certificate_id),
                        context
                            .workspace
                            .proxies
                            .get_optional(&params.auth_proxy_id),
                        context.enable_trace,
                    )
                    .await
                    {
                        Ok(token_result) => {
                            request_builder =
                                request_builder.bearer_auth(token_result.token.clone());
                            oauth2_token = Some(token_result);
                        }
                        Err(err) => return Err(err),
                    }
                }
                Some(Authorization::OAuth2Pkce { token, .. }) => match token {
                    Some(t) => {
                        request_builder = request_builder.bearer_auth(t.clone());
                    }
                    None => {
                        return Err(ApicizeError::Error {
                            description: String::from("PKCE access token is not available"),
                            source: None,
                        });
                    }
                },
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
                            RequestEntry::clone_and_sub(&nvp.name, &subs),
                            RequestEntry::clone_and_sub(&nvp.value, &subs),
                        ));
                    }
                }
                request_builder = request_builder.query(&query);
            }

            // Add body, if applicable
            match &request.body {
                Some(RequestBody::Text { data }) => {
                    let s = RequestEntry::clone_and_sub(data, &subs);
                    request_builder = request_builder.body(Body::from(s.clone()));
                }
                Some(RequestBody::JSON { data, .. }) => {
                    let s = RequestEntry::clone_and_sub(data, &subs);
                    request_builder = request_builder.body(Body::from(s));
                }
                Some(RequestBody::XML { data }) => {
                    let s = RequestEntry::clone_and_sub(data, &subs);
                    request_builder = request_builder.body(Body::from(s));
                }
                Some(RequestBody::Form { data }) => {
                    let form_data = data
                        .iter()
                        .map(|pair| {
                            (
                                String::from(pair.name.as_str()),
                                String::from(pair.value.as_str()),
                            )
                        })
                        .collect::<HashMap<String, String>>();
                    request_builder = request_builder.form(&form_data);
                }
                Some(RequestBody::Raw { data }) => {
                    request_builder = request_builder.body(Body::from(data.clone()));
                }
                None => {}
            }

            // let mut web_request = request_builder.build()?;
            match request_builder.build() {
                Ok(mut web_request) => {
                    // Copy value generated for the request so that we can include in the function results
                    let request_url = web_request.url().to_string();
                    let request_headers = web_request
                        .headers()
                        .iter()
                        .map(|(h, v)| {
                            (
                                String::from(h.as_str()),
                                String::from(
                                    v.to_str().unwrap_or("(Header Contains Non-ASCII Data)"),
                                ),
                            )
                        })
                        .collect::<HashMap<String, String>>();
                    let ref_body = web_request.body_mut();
                    let request_body = match ref_body {
                        Some(data) => {
                            let bytes = data.as_bytes().unwrap();
                            if bytes.is_empty() {
                                None
                            } else {
                                let request_encoding = UTF_8;

                                let data = bytes.to_vec();
                                if data.is_empty() {
                                    None
                                } else {
                                    let (decoded, _, malformed) = request_encoding.decode(&data);
                                    if malformed {
                                        Some(ApicizeBody::Binary { data })
                                    } else {
                                        Some(ApicizeBody::Text {
                                            data: decoded.to_string(),
                                        })
                                    }
                                }
                            }
                        }
                        None => None,
                    };

                    // Execute the request
                    let client_response = client.execute(web_request).await;
                    match client_response {
                        Err(error) => Err(ApicizeError::from_reqwest(error)),
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
                                            let value = v
                                                .to_str()
                                                .unwrap_or("(Header Contains Non-ASCII Data)");
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
                                .and_then(|mime| {
                                    mime.get_param("charset").map(|charset| charset.as_str())
                                })
                                .unwrap_or("utf-8");

                            let response_encoding =
                                Encoding::for_label(response_encoding_name.as_bytes())
                                    .unwrap_or(UTF_8);

                            // Collect status for response
                            let status = response.status();
                            let status_text =
                                String::from(status.canonical_reason().unwrap_or("Unknown"));

                            // Retrieve response bytes and convert raw data to string
                            match response.bytes().await {
                                Ok(bytes) => {
                                    let response_body = if bytes.is_empty() {
                                        None
                                    } else {
                                        let data = Vec::from(bytes.as_ref());
                                        let (decoded, _, malformed) =
                                            response_encoding.decode(&data);

                                        if malformed {
                                            Some(ApicizeBody::Binary { data })
                                        } else {
                                            let text = decoded.to_string();
                                            if may_have_json {
                                                if let Ok(parsed) =
                                                    serde_json::from_str::<Value>(&text)
                                                {
                                                    Some(ApicizeBody::JSON {
                                                        text: decoded.to_string(),
                                                        data: parsed,
                                                    })
                                                } else {
                                                    Some(ApicizeBody::Text { data: text })
                                                }
                                            } else {
                                                Some(ApicizeBody::Text { data: text })
                                            }
                                        }
                                    };

                                    let response = (
                                        ApicizeHttpRequest {
                                            url: request_url,
                                            method: request
                                                .method
                                                .as_ref()
                                                .unwrap()
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
                                        if params.variables.as_ref().map_or(true, |v| v.is_empty())
                                        {
                                            None
                                        } else {
                                            params.variables.clone()
                                        },
                                    );

                                    Ok(response)
                                }
                                Err(err) => Err(ApicizeError::from_reqwest(err)),
                            }
                        }
                    }
                }
                Err(err) => Err(ApicizeError::from_reqwest(err)),
            }
        }
        Err(err) => Err(ApicizeError::from_reqwest(err)),
    }
}

// /// Run the specified request entry recursively.
// #[async_recursion]
// async fn run_request_item(
//     workspace: Arc<Workspace>,
//     cancellation_token: Arc<CancellationToken>,
//     tests_started: Arc<Instant>,
//     request_id: String,
//     variables: Arc<HashMap<String, Value>>,
//     override_number_of_runs: Option<usize>,
//     value_cache: Arc<Mutex<VariableCache>>,
//     enable_trace: bool,
// ) -> Result<ApicizeExecutionItem, ApicizeError> {
//     let request_as_entry = workspace.requests.entities.get(&request_id).unwrap();
//     let name = request_as_entry.get_name().as_str();

//     let executed_at = tests_started.elapsed().as_millis();
//     let start_instant = Instant::now();
//     let number_of_runs = override_number_of_runs.unwrap_or(request_as_entry.get_runs());

//     match request_as_entry {
//         RequestEntry::Info(request) => {
//             let mut runs: Vec<ApicizeExecutionRequestRun> = vec![];

//             // todo!("It would be nice not to clone these, but with recursion it may be necessary evil");
//             let shared_entity = Arc::new(request_as_entry.clone());
//             let shared_request = Arc::new(request.clone());

//             if request.multi_run_execution == ExecutionConcurrency::Sequential
//                 || number_of_runs < 2
//             {
//                 for ctr in 1..=number_of_runs {
//                     let mut executed_runs = execute_request_run(
//                         workspace.clone(),
//                         tests_started.clone(),
//                         ctr,
//                         number_of_runs,
//                         shared_request.clone(),
//                         shared_entity.clone(),
//                         variables.clone(),
//                         enable_trace,
//                         value_cache.clone(),
//                     )
//                     .await;
//                     executed_runs.drain(..).for_each(|r| runs.push(r));
//                 }
//             } else {
//                 let mut child_runs: JoinSet<Option<Vec<ApicizeExecutionRequestRun>>> =
//                     JoinSet::new();

//                 for ctr in 1..=number_of_runs {
//                     let cloned_cancellation = cancellation_token.clone();
//                     let executed_request_runs = execute_request_run(
//                         workspace.clone(),
//                         tests_started.clone(),
//                         ctr,
//                         number_of_runs.clone(),
//                         shared_request.clone(),
//                         shared_entity.clone(),
//                         variables.clone(),
//                         enable_trace,
//                         value_cache.clone(),
//                     );
//                     child_runs.spawn(async move {
//                         select! {
//                             _ = cloned_cancellation.cancelled() => None,
//                             result =  executed_request_runs => {
//                                 Some(result)
//                             }
//                         }
//                     });
//                 }

//                 let mut executed_runs = child_runs
//                     .join_all()
//                     .await
//                     .into_iter()
//                     .flatten()
//                     .flatten()
//                     .collect::<Vec<ApicizeExecutionRequestRun>>();
//                 executed_runs.drain(..).for_each(|r| runs.push(r));
//             }

//             let mut executed_request = ApicizeExecutionRequest {
//                 id: request_id,
//                 name: String::from(name),
//                 executed_at,
//                 duration: start_instant.elapsed().as_millis(),
//                 runs: vec![],
//                 variables: None,
//                 success: true,
//                 requests_with_passed_tests_count: 0,
//                 requests_with_failed_tests_count: 0,
//                 requests_with_errors: 0,
//                 test_pass_count: 0,
//                 test_fail_count: 0,
//             };

//             for run in &runs {
//                 executed_request.add_totals(run);
//             }
//             executed_request.runs = runs;

//             Ok(ApicizeExecutionItem::Request(Box::pin(executed_request)))
//         }
//         RequestEntry::Group(group) => {
//             let group_child_ids =
//                 if let Some(group_child_ids) = workspace.requests.child_ids.get(&group.id) {
//                     group_child_ids.clone()
//                 } else {
//                     vec![]
//                 };

//             let mut runs: Vec<Result<ApicizeExecutionGroupRun, ApicizeError>> = vec![];

//             match group.multi_run_execution {
//                 ExecutionConcurrency::Sequential => {
//                     for ctr in 1..=number_of_runs {
//                         runs.push(
//                             run_request_group(
//                                 workspace.clone(),
//                                 cancellation_token.clone(),
//                                 tests_started.clone(),
//                                 group.execution.clone(),
//                                 group_child_ids.clone(),
//                                 ctr,
//                                 number_of_runs,
//                                 variables.clone(),
//                                 value_cache.clone(),
//                                 enable_trace,
//                             )
//                             .await,
//                         )
//                     }
//                 }
//                 ExecutionConcurrency::Concurrent => {
//                     let mut executing_runs: JoinSet<
//                         Option<Result<ApicizeExecutionGroupRun, ApicizeError>>,
//                     > = JoinSet::new();
//                     for ctr in 1..=number_of_runs {
//                         let cloned_cancellation = cancellation_token.clone();
//                         let executed_group = run_request_group(
//                             workspace.clone(),
//                             cancellation_token.clone(),
//                             tests_started.clone(),
//                             group.execution.clone(),
//                             group_child_ids.clone(),
//                             ctr,
//                             number_of_runs,
//                             variables.clone(),
//                             value_cache.clone(),
//                             enable_trace,
//                         );
//                         executing_runs.spawn(async move {
//                             select! {
//                                 _ = cloned_cancellation.cancelled() => None,
//                                 result = executed_group => {
//                                     Some(result)
//                                 }
//                             }
//                         });
//                     }
//                     for completed_run in executing_runs.join_all().await {
//                         match completed_run {
//                             Some(run) => {
//                                 runs.push(run);
//                             }
//                             None => {}
//                         }
//                     }

//                     runs.sort_by_key(|run| match run {
//                         Ok(r) => r.run_number,
//                         Err(_) => 0,
//                     });
//                 }
//             }

//             let mut executed_group = ApicizeExecutionGroup {
//                 id: request_id,
//                 name: String::from(name),
//                 executed_at,
//                 duration: start_instant.elapsed().as_millis(),
//                 runs: vec![],
//                 success: true,
//                 requests_with_passed_tests_count: 0,
//                 requests_with_failed_tests_count: 0,
//                 requests_with_errors: 0,
//                 test_pass_count: 0,
//                 test_fail_count: 0,
//             };

//             executed_group.runs = vec![];
//             for run in runs {
//                 match run {
//                     Ok(successful_run) => {
//                         executed_group.add_totals(&successful_run);
//                         executed_group.runs.push(successful_run);
//                     }
//                     Err(err) => {
//                         return Err(err);
//                     }
//                 }
//             }

//             Ok(ApicizeExecutionItem::Group(Box::pin(executed_group)))
//         }
//     }
// }

// #[allow(clippy::too_many_arguments)]
// #[async_recursion]
// async fn run_group_test(
//     workspace: Arc<Workspace>,
//     value_cache: &Mutex<VariableCache>,
//     cancellation_token: Arc<CancellationToken>,
//     group: &mut RequestGroup,
//     params: &RequestParameters,
//     tests_started: Arc<Instant>,
//     run_number: Option<usize>,
//     row_number: Option<usize>,
//     enable_trace: bool,
// ) -> Result<ApicizeRequestExecutionInfo, ApicizeError> {
//     let (start_at, variables) = match params.variables {
//         Some(v) => (1, v),
//         None => (0, HashMap::new()),
//     };

//     if let Some(children) = &group.children {
//         for child in children {
//             let child_params =
//                 workspace.retrieve_request_parameters(&child, &variables, &value_cache)?;
//         }
//     }

//     for run_number in [1..=group.runs] {
//         for row_number in start_at..=variables.len() {
//             let row_variables = variables.get(&row_number);
//         }
//     }

//     let mut items: Vec<ApicizeExecutionItem> = vec![];
//     let number_of_children = group_child_ids.len();
//     let executed_at = tests_started.elapsed().as_millis();
//     let start_instant = Instant::now();

//     if execution == ExecutionConcurrency::Sequential || number_of_children < 2 {
//         for child_id in group_child_ids {
//             let executed_child = run_request_item(
//                 workspace.clone(),
//                 cancellation_token.clone(),
//                 tests_started.clone(),
//                 child_id.clone(),
//                 variables.clone(),
//                 None,
//                 value_cache.clone(),
//                 enable_trace,
//             )
//             .await;

//             match executed_child {
//                 Ok(execution) => {
//                     if let Some(updated_variables) = execution.get_variables() {
//                         variables = Arc::new(updated_variables.clone());
//                     }
//                     items.push(execution);
//                 }
//                 Err(err) => {
//                     return Err(err);
//                 }
//             }
//         }
//     } else {
//         let mut child_items: JoinSet<Option<Result<ApicizeExecutionItem, ApicizeError>>> =
//             JoinSet::new();
//         for id in group_child_ids {
//             let cloned_cancellation = cancellation_token.clone();
//             let executed_item = run_request_item(
//                 workspace.clone(),
//                 cancellation_token.clone(),
//                 tests_started.clone(),
//                 id,
//                 variables.clone(),
//                 None,
//                 value_cache.clone(),
//                 enable_trace,
//             );
//             child_items.spawn(async move {
//                 select! {
//                     _ = cloned_cancellation.cancelled() => None,
//                     result =  executed_item => {
//                         Some(result)
//                     }
//                 }
//             });
//         }

//         while let Some(child_results) = child_items.join_next().await {
//             match child_results {
//                 Ok(execution) => {
//                     if let Some(execution_item) = execution {
//                         match execution_item {
//                             Ok(result) => {
//                                 items.push(result);
//                             }
//                             Err(err) => {
//                                 return Err(err);
//                             }
//                         }
//                     }
//                 }
//                 Err(err) => {
//                     return Err(ApicizeError::from_async(err));
//                 }
//             }
//         }
//     }

//     let mut executed_run = ApicizeExecutionGroupRun {
//         run_number,
//         number_of_runs,
//         executed_at,
//         duration: start_instant.elapsed().as_millis(),
//         items: vec![], // placeholder
//         variables: None,
//         success: true,
//         requests_with_passed_tests_count: 0,
//         requests_with_failed_tests_count: 0,
//         requests_with_errors: 0,
//         test_pass_count: 0,
//         test_fail_count: 0,
//     };

//     for item in &items {
//         executed_run.add_totals(item);
//         Clone::clone_from(&mut executed_run.variables, item.get_variables());
//     }
//     executed_run.items = items;
//     return Ok(executed_run);
// }

/// Execute the specified request's tests
fn execute_request_test(
    execute_test: &Option<String>,
    request: &ApicizeHttpRequest,
    response: &ApicizeHttpResponse,
    variables: &Option<Map<String, Value>>,
    data: &Option<Map<String, Value>>,
    tests_started: &Instant,
) -> Result<Option<ApicizeTestResponse>, ApicizeError> {
    // Return empty test results if no test
    match execute_test {
        None => Ok(None),
        Some(test) => {
            // Ensure V8 is initialized
            V8_INIT.call_once(|| {
                let platform = v8::new_unprotected_default_platform(0, false).make_shared();
                v8::V8::initialize_platform(platform);
                v8::V8::initialize();
            });

            // Create a new Isolate and make it the current one.
            let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

            // Create a stack-allocated handle scope.
            let scope = &mut v8::HandleScope::new(isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);

            let mut init_code = String::new();
            init_code.push_str(include_str!(concat!(env!("OUT_DIR"), "/framework.min.js")));

            // Compile the source code
            let v8_code = v8::String::new(scope, &init_code).unwrap();
            let script = v8::Script::compile(scope, v8_code, None).unwrap();
            script.run(scope).unwrap();

            let tc = &mut v8::TryCatch::new(scope);

            let cloned_tests_started = tests_started;

            let mut merged_variables: Map<String, Value> = match variables {
                Some(data) => data.clone(),
                None => Map::new(),
            };

            if let Some(v) = data {
                merged_variables.extend(v.iter().map(|(key, value)| (key.clone(), value.clone())));
            }

            let mut init_code = String::new();
            init_code.push_str(&format!(
                "runTestSuite({}, {}, {}, {}, () => {{{}}})",
                serde_json::to_string(request).unwrap(),
                serde_json::to_string(response).unwrap(),
                serde_json::to_string(&merged_variables).unwrap(),
                std::time::UNIX_EPOCH.elapsed().unwrap().as_millis()
                    - cloned_tests_started.elapsed().as_millis()
                    + 1,
                test,
            ));

            let v8_code = v8::String::new(tc, &init_code).unwrap();

            let Some(script) = v8::Script::compile(tc, v8_code, None) else {
                let message = tc.message().unwrap();
                let message = message.get(tc).to_rust_string_lossy(tc);
                return Err(ApicizeError::from_failed_test(message));
            };

            let Some(value) = script.run(tc) else {
                let message = tc.message().unwrap();
                let message = message.get(tc).to_rust_string_lossy(tc);
                return Err(ApicizeError::from_failed_test(message));
            };

            let result = value.to_string(tc);
            let s = result.unwrap().to_rust_string_lossy(tc);
            let test_response: ApicizeTestResponse = serde_json::from_str(&s).unwrap();

            Ok(Some(test_response))
        }
    }
}

// /// Dispatch the specified request and execute its tests
// async fn execute_request_run(
//     workspace: Arc<Workspace>,
//     tests_started: Arc<Instant>,
//     run_number: usize,
//     number_of_runs: usize,
//     request: Arc<Request>,
//     request_as_entry: Arc<RequestEntry>,
//     variables: Arc<HashMap<String, Value>>,
//     enable_trace: bool,
//     value_cache: Arc<Mutex<VariableCache>>,
// ) -> Vec<ApicizeExecutionRequestRun> {
//     let shared_workspace = workspace.clone();
//     let shared_test_started = tests_started.clone();

//     let executed_at = shared_test_started.elapsed().as_millis();
//     let start_instant = Instant::now();

//     let mut runs = Vec::<ApicizeExecutionRequestRun>::new();

//     let mut current_row_number = 0;

//     loop {
//         let params = match shared_workspace.retrieve_request_parameters(
//             &request_as_entry,
//             &variables,
//             &value_cache,
//             current_row_number,
//         ) {
//             Ok(valid) => valid,
//             Err(err) => {
//                 return vec![ApicizeExecutionRequestRun {
//                     run_number,
//                     number_of_runs,
//                     row_number: None,
//                     total_rows: None,
//                     executed_at,
//                     duration: start_instant.elapsed().as_millis(),
//                     request: None,
//                     response: None,
//                     success: false,
//                     error: Some(err),
//                     tests: None,
//                     input_variables: None,
//                     variables: None,
//                     requests_with_passed_tests_count: 0,
//                     requests_with_failed_tests_count: 0,
//                     requests_with_errors: 0,
//                     test_pass_count: 0,
//                     test_fail_count: 0,
//                 }];
//             }
//         };

//         current_row_number = params.row_number;
//         let dispatch_response = dispatch_request(&request, &workspace, &params, enable_trace).await;

//         match dispatch_response {
//             Ok((packaged_request, response)) => {
//                 let test_result = execute_request_test(
//                     &request.clone(),
//                     &response,
//                     &params.variables,
//                     &shared_test_started,
//                 );
//                 match test_result {
//                     Ok(test_response) => {
//                         let mut test_count = 0;
//                         let mut test_fail_count = 0;
//                         let result_variables: Option<HashMap<String, Value>>;
//                         let test_results = match test_response {
//                             Some(response) => {
//                                 result_variables = Some(response.variables.clone());
//                                 if let Some(test_results) = &response.results {
//                                     test_count = test_results.len();
//                                     test_fail_count +=
//                                         test_results.iter().filter(|r| !r.success).count();
//                                 }
//                                 response.results
//                             }
//                             None => {
//                                 result_variables = None;
//                                 None
//                             }
//                         };

//                         runs.push(ApicizeExecutionRequestRun {
//                             run_number,
//                             number_of_runs,
//                             row_number: if params.row_number > 0 {
//                                 Some(params.row_number)
//                             } else {
//                                 None
//                             },
//                             total_rows: if params.total_rows > 0 {
//                                 Some(params.total_rows)
//                             } else {
//                                 None
//                             },
//                             executed_at,
//                             duration: start_instant.elapsed().as_millis(),
//                             request: Some(packaged_request.clone()),
//                             response: Some(response.clone()),
//                             success: test_count == 0 || test_fail_count == 0,
//                             error: None,
//                             tests: test_results,
//                             input_variables: if params.variables.is_empty() {
//                                 None
//                             } else {
//                                 Some(params.variables.clone())
//                             },
//                             variables: result_variables,
//                             requests_with_passed_tests_count: if test_count == 0
//                                 && test_fail_count == 0
//                             {
//                                 1
//                             } else {
//                                 0
//                             },
//                             requests_with_failed_tests_count: if test_fail_count > 0 {
//                                 1
//                             } else {
//                                 0
//                             },
//                             requests_with_errors: 0,
//                             test_pass_count: test_count - test_fail_count,
//                             test_fail_count,
//                         })
//                     }
//                     Err(err) => runs.push(ApicizeExecutionRequestRun {
//                         run_number,
//                         number_of_runs,
//                         row_number: None,
//                         total_rows: None,
//                         executed_at,
//                         duration: start_instant.elapsed().as_millis(),
//                         request: Some(packaged_request.clone()),
//                         response: Some(response.clone()),
//                         success: false,
//                         error: Some(err),
//                         tests: None,
//                         input_variables: None,
//                         variables: None,
//                         requests_with_passed_tests_count: 0,
//                         requests_with_failed_tests_count: 0,
//                         requests_with_errors: 1,
//                         test_pass_count: 0,
//                         test_fail_count: 0,
//                     }),
//                 }
//             }
//             Err(err) => runs.push(ApicizeExecutionRequestRun {
//                 run_number,
//                 number_of_runs,
//                 row_number: None,
//                 total_rows: None,
//                 executed_at,
//                 duration: start_instant.elapsed().as_millis(),
//                 request: None,
//                 response: None,
//                 success: false,
//                 error: Some(err),
//                 tests: None,
//                 input_variables: None,
//                 variables: None,
//                 requests_with_passed_tests_count: 0,
//                 requests_with_failed_tests_count: 0,
//                 requests_with_errors: 1,
//                 test_pass_count: 0,
//                 test_fail_count: 0,
//             }),
//         }

//         if params.row_number >= params.total_rows {
//             break;
//         }
//     }

//     runs.sort_by(|a, b| {
//         let mut cmp = a.run_number.cmp(&b.run_number);
//         if cmp == Ordering::Equal {
//             cmp = a.row_number.cmp(&b.row_number)
//         }
//         cmp
//     });

//     runs
// }

/// Cleanup V8 platform, should only be called once at end of application
pub fn cleanup_v8() {
    unsafe {
        v8::V8::dispose();
    }
    v8::V8::dispose_platform();
}

// #[cfg(test)]
// mod tests {
//     use mockito::Matcher;
//     use serde_json::Value;
//     use std::{
//         collections::HashMap,
//         sync::Arc,
//         thread::sleep,
//         time::{Duration, Instant},
//     };
//     use tokio::task::JoinSet;
//     use tokio_util::sync::CancellationToken;

//     use super::{ApicizeExecution, ApicizeExecutionItem, ApicizeResponse};
//     use crate::{
//         execution::test_runner::{dispatch_request, execute_request_test}, oauth2_client_tokens::TokenResult, ApicizeError, Certificate, IndexedEntities, IndexedRequests, NameValuePair, Proxy, Request, RequestEntry, RequestMethod, Workspace
//     };

//     use crate::oauth2_client_tokens::tests::MockOAuth2ClientTokens;

//     #[tokio::test]
//     async fn dispatch_requests_and_handles_bad_domain() {
//         let request = Request {
//             id: String::from(""),
//             name: String::from("test"),
//             url: String::from("https://foofooxxxxxx/"),
//             method: Some(RequestMethod::Post),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: None,
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };
//         let response =
//             dispatch_request(&request, &HashMap::new(), None, None, None, None, None).await;
//         match &response {
//             Ok(_) => {}
//             Err(err) => {
//                 println!("{}: {}", err.get_label(), err);
//             }
//         }
//         assert!(response.is_err());
//     }

//     #[tokio::test]
//     async fn dispatch_requests_and_handles_timeout() {
//         let mut server = mockito::Server::new_async().await;
//         let mock = server
//             .mock("GET", "/")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_chunked_body(|_| {
//                 sleep(Duration::from_secs(1));
//                 Ok({})
//             })
//             .create();

//         let request = Request {
//             id: String::from(""),
//             name: String::from("test"),
//             url: server.url(),
//             method: Some(RequestMethod::Get),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: Some(1),
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };
//         let response =
//             dispatch_request(&request, &HashMap::new(), None, None, None, None, None).await;
//         match &response {
//             Ok(_) => {}
//             Err(err) => {
//                 println!("{}: {}", err.get_label(), err);
//             }
//         }
//         assert!(response.is_err());
//         mock.assert();
//     }

//     #[tokio::test]
//     async fn dispatch_requests_with_substituted_variables() {
//         let mut server = mockito::Server::new_async().await;
//         let mock = server
//             .mock("POST", "/test")
//             .match_query(Matcher::AllOf(vec![Matcher::UrlEncoded(
//                 "abc".into(),
//                 "123".into(),
//             )]))
//             .match_header("xxx", "zzz")
//             .match_body("foo")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_body("ok")
//             .create();

//         let request = Request {
//             id: String::from(""),
//             name: String::from("test"),
//             url: server.url() + "/{{page}}",
//             method: Some(RequestMethod::Post),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: None,
//             keep_alive: None,
//             runs: 1,
//             headers: Some(vec![NameValuePair {
//                 name: String::from("xxx"),
//                 value: String::from("{{xxx}}"),
//                 disabled: None,
//             }]),
//             query_string_params: Some(vec![NameValuePair {
//                 name: String::from("abc"),
//                 value: String::from("{{abc}}"),
//                 disabled: None,
//             }]),
//             body: Some(crate::RequestBody::Text {
//                 data: String::from("{{stuff}}"),
//             }),
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let variables = HashMap::from([
//             (String::from("page"), Value::from("test")),
//             (String::from("abc"), Value::from("123")),
//             (String::from("xxx"), Value::from("zzz")),
//             (String::from("stuff"), Value::from("foo")),
//         ]);
//         let response = dispatch_request(&request, &variables, None, None, None, None, None).await;
//         mock.assert();
//         assert_eq!(response.unwrap().1.status, 200);
//     }

//     #[tokio::test]
//     async fn dispatch_requests_with_basic_auth() {
//         let mut server = mockito::Server::new_async().await;
//         let mock = server
//             .mock("POST", "/test")
//             .match_header("Authorization", "Basic bmFtZTpzaGho")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_body("ok")
//             .create();

//         let request = Request {
//             id: String::from(""),
//             name: String::from("test"),
//             url: server.url() + "/test",
//             method: Some(RequestMethod::Post),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: None,
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let response = dispatch_request(
//             &request,
//             &HashMap::new(),
//             Some(&crate::Authorization::Basic {
//                 id: String::from(""),
//                 name: String::from(""),
//                 username: String::from("name"),
//                 password: String::from("shhh"),
//             }),
//             None,
//             None,
//             None,
//             None,
//         )
//         .await;
//         mock.assert();
//         assert_eq!(response.unwrap().1.status, 200);
//     }

//     #[tokio::test]
//     async fn dispatch_requests_with_api_key_auth() {
//         let mut server = mockito::Server::new_async().await;
//         let mock = server
//             .mock("POST", "/test")
//             .match_header("x-api-key", "abc")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_body("ok")
//             .create();

//         let request = Request {
//             id: String::from(""),
//             name: String::from("test"),
//             url: server.url() + "/test",
//             method: Some(RequestMethod::Post),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: None,
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let response = dispatch_request(
//             &request,
//             &HashMap::new(),
//             Some(&crate::Authorization::ApiKey {
//                 id: String::from(""),
//                 name: String::from(""),
//                 header: String::from("x-api-key"),
//                 value: String::from("abc"),
//             }),
//             None,
//             None,
//             None,
//             None,
//         )
//         .await;
//         mock.assert();
//         assert_eq!(response.unwrap().1.status, 200);
//     }

//     #[tokio::test]
//     async fn dispatch_requests_with_oauth2_auth() {
//         let mut server = mockito::Server::new_async().await;
//         let mock = server
//             .mock("POST", "/test")
//             .match_header("authorization", "Bearer ***TOKEN***")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_body("ok")
//             .create();

//         let request = Request {
//             id: String::from(""),
//             name: String::from("test"),
//             url: server.url() + "/test",
//             method: Some(RequestMethod::Post),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: None,
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let oauth2_context = MockOAuth2ClientTokens::get_oauth2_client_credentials_context();
//         oauth2_context
//             .expect()
//             .withf(
//                 |id, url, _client_id, _client_secret, _scope, _certificaite, _proxy| {
//                     id == String::from("11111") && url == String::from("https://server")
//                 },
//             )
//             .returning(
//                 |_id: &str,
//                  _token_url: &str,
//                  _client_id: &str,
//                  _client_secret: &str,
//                  _scope: &Option<String>,
//                  _certificate: Option<&Certificate>,
//                  _proxy: Option<&Proxy>| {
//                     Ok(TokenResult {
//                         token: String::from("***TOKEN***"),
//                         cached: true,
//                         url: None,
//                         certificate: None,
//                         proxy: None,
//                     })
//                 },
//             );

//         let response = dispatch_request(
//             &request,
//             &HashMap::new(),
//             Some(&crate::Authorization::OAuth2Client {
//                 id: String::from("11111"),
//                 name: String::from("My Token"),
//                 access_token_url: String::from("https://server"),
//                 client_id: String::from("me"),
//                 client_secret: String::from("shhh"),
//                 scope: Some(String::from("x")),
//                 selected_certificate: None,
//                 selected_proxy: None,
//             }),
//             None,
//             None,
//             None,
//             None,
//         )
//         .await;
//         mock.assert();
//         assert_eq!(response.unwrap().1.status, 200);
//     }

//     #[tokio::test]
//     async fn execute_request_test_runs_test() {
//         let request = Request {
//             id: String::from("xxx"),
//             name: String::from("xxx"),
//             test: Some(String::from("describe('test', () => { it('runs', () => { expect(response.status).to.equal(200) }) })")),
//             url: String::from("http://foo"),
//             method: Some(RequestMethod::Get),
//             timeout: Some(5000),
//             headers: None,
//             query_string_params: None,
//             body: None,
//             keep_alive: None,
//             runs: 1,
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let response = ApicizeResponse {
//             status: 200,
//             status_text: String::from("Ok"),
//             headers: None,
//             body: None,
//             oauth2_token: None,
//         };

//         let variables: HashMap<String, Value> = HashMap::new();

//         let tests_started = Arc::new(Instant::now());

//         let result = execute_request_test(&request, &response, &variables, &tests_started);

//         let mut successes = 0;
//         let mut failures = 0;
//         for test_result in result.unwrap().unwrap().results.unwrap().iter() {
//             // if let Some(logs) = &test_result.logs {
//             //     println!("Logs: {}", logs.join("; "));
//             // }
//             // if let Some(error) = &test_result.error {
//             //     println!("Error: {}", error);
//             // }

//             if test_result.success {
//                 successes += 1;
//             } else {
//                 failures += 1;
//             }
//         }

//         assert_eq!(successes, 1);
//         assert_eq!(failures, 0);
//     }

//     #[tokio::test]
//     async fn execute_request_test_includes_jsonpath() {
//         let request = Request {
//             id: String::from("xxx"),
//             name: String::from("xxx"),
//             test: Some(String::from("describe('test', () => { it('works', () => { var foo = { \"abc\": 123 }; expect(jsonpath('$.abc', foo)[0]).to.equal(123) }) })")),
//             url: String::from("http://foo"),
//             method: Some(RequestMethod::Get),
//             timeout: Some(5000),
//             headers: None,
//             query_string_params: None,
//             body: None,
//             keep_alive: None,
//             runs: 1,
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let response = ApicizeResponse {
//             status: 200,
//             status_text: String::from("Ok"),
//             headers: None,
//             body: None,
//             oauth2_token: None,
//         };

//         let variables: HashMap<String, Value> = HashMap::new();

//         let tests_started = Arc::new(Instant::now());

//         let result = execute_request_test(&request, &response, &variables, &tests_started);

//         let mut successes = 0;
//         let mut failures = 0;
//         for test_result in result.unwrap().unwrap().results.unwrap().iter() {
//             // if let Some(logs) = &test_result.logs {
//             //     println!("Logs: {}", logs.join("; "));
//             // }
//             // if let Some(error) = &test_result.error {
//             //     println!("Error: {}", error);
//             // }
//             if test_result.success {
//                 successes += 1;
//             } else {
//                 failures += 1;
//             }
//         }

//         assert_eq!(successes, 1);
//         assert_eq!(failures, 0);
//     }

//     #[tokio::test]
//     async fn execute_request_test_includes_xpath() {
//         let request = Request {
//             id: String::from("xxx"),
//             name: String::from("xxx"),
//             test: Some(String::from("describe('test', () => { it('works', () => { const xml = \"<foo><bar>test</bar></foo>\"; const doc = new dom().parseFromString(xml, 'text/xml'); expect(xpath.select('//bar', doc)[0].firstChild.data).to.equal('test') }) })")),
//             url: String::from("http://foo"),
//             method: Some(RequestMethod::Get),
//             timeout: Some(5000),
//             headers: None,
//             query_string_params: None,
//             body: None,
//             keep_alive: None,
//             runs: 1,
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         };

//         let response = ApicizeResponse {
//             status: 200,
//             status_text: String::from("Ok"),
//             headers: None,
//             body: None,
//             oauth2_token: None,
//         };

//         let variables: HashMap<String, Value> = HashMap::new();

//         let tests_started = Arc::new(Instant::now());

//         let result = execute_request_test(&request, &response, &variables, &tests_started);

//         let mut successes = 0;
//         let mut failures = 0;
//         for test_result in result.unwrap().unwrap().results.unwrap().iter() {
//             // if let Some(logs) = &test_result.logs {
//             //     println!("Logs: {}", logs.join("; "));
//             // }
//             // if let Some(error) = &test_result.error {
//             //     println!("Error: {}", error);
//             // }
//             if test_result.success {
//                 successes += 1;
//             } else {
//                 failures += 1;
//             }
//         }

//         assert_eq!(successes, 1);
//         assert_eq!(failures, 0);
//     }

//     async fn wait_and_cancel(
//         cancellation: CancellationToken,
//     ) -> Result<ApicizeExecution, ApicizeError> {
//         sleep(Duration::from_millis(10));
//         cancellation.cancel();
//         Ok(ApicizeExecution {
//             duration: 0,
//             items: vec![],
//             success: false,
//             requests_with_passed_tests_count: 0,
//             requests_with_failed_tests_count: 0,
//             requests_with_errors: 0,
//             test_pass_count: 0,
//             test_fail_count: 0,
//         })
//     }

//     #[tokio::test]
//     async fn run_honors_override_number_of_runs() {
//         let mut server = mockito::Server::new_async().await;
//         server
//             .mock("GET", "/")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_body("Ok")
//             .create();

//         let request = RequestEntry::Info(Request {
//             id: String::from("123"),
//             name: String::from("test"),
//             url: server.url(),
//             method: Some(RequestMethod::Get),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: Some(500),
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         });

//         let workspace = Workspace {
//             requests: IndexedRequests {
//                 top_level_ids: vec![String::from("123")],
//                 entities: HashMap::from([(String::from("123"), request)]),
//                 child_ids: None,
//             },
//             scenarios: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             authorizations: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             certificates: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             proxies: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             defaults: None,
//             warnings: None,
//         };

//         let tests_started = Arc::new(Instant::now());
//         let cancellation = CancellationToken::new();

//         let attempt = super::run(
//             Arc::new(workspace),
//             Some(vec![String::from("123")]),
//             Some(cancellation.clone()),
//             tests_started,
//             Some(4),
//         )
//         .await;

//         let runs = if let ApicizeExecutionItem::Request(result) =
//             attempt.unwrap().items.first().unwrap()
//         {
//             result.runs.len()
//         } else {
//             0
//         };
//         assert_eq!(runs, 4)
//     }

//     #[tokio::test]
//     async fn run_honors_cancel() {
//         let mut server = mockito::Server::new_async().await;
//         server
//             .mock("GET", "/")
//             .with_status(200)
//             .with_header("Content-Type", "text/plain")
//             .with_chunked_body(|_| {
//                 sleep(Duration::from_secs(5000));
//                 Ok({})
//             })
//             .create();

//         let request = RequestEntry::Info(Request {
//             id: String::from("123"),
//             name: String::from("test"),
//             url: server.url(),
//             method: Some(RequestMethod::Get),
//             multi_run_execution: crate::ExecutionConcurrency::Sequential,
//             timeout: Some(60000),
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//             warnings: None,
//         });

//         let workspace = Workspace {
//             requests: IndexedRequests {
//                 top_level_ids: vec![String::from("123")],
//                 entities: HashMap::from([(String::from("123"), request)]),
//                 child_ids: None,
//             },
//             scenarios: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             authorizations: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             certificates: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             proxies: IndexedEntities {
//                 top_level_ids: vec![],
//                 entities: HashMap::new(),
//             },
//             defaults: None,
//             warnings: None,
//         };

//         let tests_started = Arc::new(Instant::now());
//         let cancellation = CancellationToken::new();

//         let mut results: JoinSet<Result<ApicizeExecution, ApicizeError>> = JoinSet::new();

//         let attempt = super::run(
//             Arc::new(workspace),
//             Some(vec![String::from("123")]),
//             Some(cancellation.clone()),
//             tests_started,
//             None,
//         );

//         results.spawn(attempt);
//         let cloned_cancellation = cancellation.clone();
//         results.spawn(wait_and_cancel(cloned_cancellation));

//         let completed_results = results.join_all().await;
//         let has_cancelled_result = completed_results
//             .iter()
//             .any(|r| r.as_ref().is_err_and(|err| err.get_label() == "Cancelled"));
//         assert!(has_cancelled_result);
//     }
// }

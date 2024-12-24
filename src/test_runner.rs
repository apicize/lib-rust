//! Apicize test execution.
//!
//! This library supports dispatching Apicize functional web tests
use std::collections::HashMap;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use async_recursion::async_recursion;
use encoding_rs::{Encoding, UTF_8};
use mime::Mime;
use reqwest::{Body, Client};
use serde_json::Value;
use tokio::select;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::apicize::{
    ApicizeBody, ApicizeExecution, ApicizeExecutionGroup, ApicizeExecutionGroupRun,
    ApicizeExecutionItem, ApicizeExecutionRequest, ApicizeExecutionRequestRun, ApicizeRequest,
    ApicizeTestResponse, ExecutionTotals, ExecutionTotalsSource,
};
use crate::oauth2_client_tokens::TokenResult;
use crate::{apicize::ApicizeResponse, WorkbookRequest};
use crate::{
    ApicizeError, WorkbookAuthorization, WorkbookCertificate, WorkbookExecution, WorkbookProxy,
    WorkbookRequestBody, WorkbookRequestEntry, WorkbookRequestMethod, Workspace,
};

#[cfg(test)]
use crate::oauth2_client_tokens::tests::MockOAuth2ClientTokens as oauth2;

#[cfg(not(test))]
use crate::oauth2_client_tokens as oauth2;

static V8_INIT: Once = Once::new();

/// Dispatch requests/groups in the specified workspace, optionally forcing the number of runs
pub async fn run(
    workspace: Arc<Workspace>,
    request_ids: Option<Vec<String>>,
    cancellation_token: Option<CancellationToken>,
    tests_started: Arc<Instant>,
    override_number_of_runs: Option<usize>,
) -> Result<ApicizeExecution, ApicizeError> {
    // Ensure V8 is initialized
    V8_INIT.call_once(|| {
        let platform = v8::new_unprotected_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });

    let cancellation = match cancellation_token {
        Some(t) => t,
        None => CancellationToken::new(),
    };

    let request_ids_to_execute = request_ids.unwrap_or(workspace.requests.top_level_ids.clone());

    let mut executing_items: JoinSet<Result<ApicizeExecutionItem, ApicizeError>> = JoinSet::new();
    for request_id in request_ids_to_execute {
        let cloned_workspace = workspace.clone();
        let cloned_tests_started = tests_started.clone();
        let cloned_token = cancellation.clone();

        executing_items.spawn(async move {
            select! {
                _ = cloned_token.cancelled() => Err(ApicizeError::Cancelled {
                    description: String::from("Cancelled"), source: None
                }),
                result = run_request_item(
                    cloned_workspace,
                    cloned_token.clone(),
                    cloned_tests_started,
                    request_id,
                    Arc::new(HashMap::new()),
                    override_number_of_runs,
                ) => {
                    Ok(result)
                }
            }
        });
    }

    let completed_items = executing_items.join_all().await;

    let mut error: Option<ApicizeError> = None;
    let mut result = ApicizeExecution {
        duration: tests_started.elapsed().as_millis(),
        items: vec![],
        success: true,
        requests_with_passed_tests_count: 0,
        requests_with_failed_tests_count: 0,
        requests_with_errors: 0,
        passed_test_count: 0,
        failed_test_count: 0,
    };

    for completed_item in &completed_items {
        match completed_item {
            Ok(item) => {
                result.add_totals(item);
                result.items.push(item.to_owned());
            }
            Err(err) => {
                error = Some(err.to_owned());
                break;
            }
        }
    }

    match error {
        Some(err) => Err(err),
        None => Ok(result),
    }
}

#[allow(clippy::too_many_arguments)]
#[async_recursion]
async fn run_request_group(
    workspace: Arc<Workspace>,
    cancellation_token: CancellationToken,
    tests_started: Arc<Instant>,
    execution: WorkbookExecution,
    group_child_ids: Vec<String>,
    run_number: usize,
    mut variables: Arc<HashMap<String, Value>>,
) -> ApicizeExecutionGroupRun {
    let mut items: Vec<ApicizeExecutionItem> = vec![];
    let number_of_children = group_child_ids.len();
    let executed_at = tests_started.elapsed().as_millis();
    let start_instant = Instant::now();

    if execution == WorkbookExecution::Sequential || number_of_children < 2 {
        for child_id in group_child_ids {
            let executed_child = run_request_item(
                workspace.clone(),
                cancellation_token.clone(),
                tests_started.clone(),
                child_id.clone(),
                variables.clone(),
                None,
            )
            .await;

            if let Some(updated_variables) = executed_child.get_variables() {
                variables = Arc::new(updated_variables.clone());
            }
            items.push(executed_child);
        }
    } else {
        let mut child_items: JoinSet<Option<ApicizeExecutionItem>> = JoinSet::new();
        for id in group_child_ids {
            let cloned_cancellation = cancellation_token.clone();
            let executed_item = run_request_item(
                workspace.clone(),
                cancellation_token.clone(),
                tests_started.clone(),
                id,
                variables.clone(),
                None,
            );
            child_items.spawn(async move {
                select! {
                    _ = cloned_cancellation.cancelled() => None,
                    result =  executed_item => {
                        Some(result)
                    }
                }
            });
        }

        while let Some(child_results) = child_items.join_next().await {
            if let Ok(Some(result)) = child_results {
                items.push(result);
            }
        }
    }

    let mut executed_run = ApicizeExecutionGroupRun {
        run_number,
        executed_at,
        duration: start_instant.elapsed().as_millis(),
        items: vec![], // placeholder
        variables: None,
        success: true,
        requests_with_passed_tests_count: 0,
        requests_with_failed_tests_count: 0,
        requests_with_errors: 0,
        passed_test_count: 0,
        failed_test_count: 0,
    };

    for item in &items {
        executed_run.add_totals(item);
        Clone::clone_from(&mut executed_run.variables, item.get_variables());
    }
    executed_run.items = items;
    return executed_run;
}

/// Run the specified request entry recursively.  We are marking everything
/// static since this is being launched in a spawn
#[async_recursion]
pub async fn run_request_item(
    workspace: Arc<Workspace>,
    cancellation_token: CancellationToken,
    tests_started: Arc<Instant>,
    request_id: String,
    variables: Arc<HashMap<String, Value>>,
    override_number_of_runs: Option<usize>,
) -> ApicizeExecutionItem {
    let entity = workspace.requests.entities.get(&request_id).unwrap();
    let params = workspace.retrieve_parameters(entity, &variables);
    let name = entity.get_name().as_str();

    let executed_at = tests_started.elapsed().as_millis();
    let start_instant = Instant::now();
    let number_of_runs = override_number_of_runs.unwrap_or(entity.get_runs());

    match entity {
        WorkbookRequestEntry::Info(request) => {
            let mut runs: Vec<ApicizeExecutionRequestRun> = vec![];

            // todo!("It would be nice not to clone these, but with recursion it may be necessary evil");
            let shared_entity = Arc::new(entity.clone());
            let shared_request = Arc::new(request.clone());
            let shared_variables = Arc::new(params.variables);

            if request.multi_run_execution == WorkbookExecution::Sequential || number_of_runs < 2 {
                for ctr in 0..number_of_runs {
                    let run = execute_request_run(
                        workspace.clone(),
                        tests_started.clone(),
                        ctr + 1,
                        shared_request.clone(),
                        shared_entity.clone(),
                        shared_variables.clone(),
                    )
                    .await;
                    runs.push(run);
                }
            } else {
                let mut child_runs: JoinSet<Option<ApicizeExecutionRequestRun>> = JoinSet::new();

                for ctr in 0..number_of_runs {
                    let cloned_cancellation = cancellation_token.clone();
                    let executed_request_run = execute_request_run(
                        workspace.clone(),
                        tests_started.clone(),
                        ctr + 1,
                        shared_request.clone(),
                        shared_entity.clone(),
                        shared_variables.clone(),
                    );
                    child_runs.spawn(async move {
                        select! {
                            _ = cloned_cancellation.cancelled() => None,
                            result =  executed_request_run => {
                                Some(result)
                            }
                        }
                    });
                }

                let completed_runs = child_runs.join_all().await;
                for completed_run in completed_runs.into_iter().flatten() {
                    runs.push(completed_run);
                }
            }

            let mut executed_request = ApicizeExecutionRequest {
                id: request_id,
                name: String::from(name),
                executed_at,
                duration: start_instant.elapsed().as_millis(),
                runs: vec![],
                variables: None,
                success: true,
                requests_with_passed_tests_count: 0,
                requests_with_failed_tests_count: 0,
                requests_with_errors: 0,
                passed_test_count: 0,
                failed_test_count: 0,
            };

            for run in &runs {
                executed_request.add_totals(run);
            }

            executed_request.runs = runs;

            ApicizeExecutionItem::Request(Box::new(executed_request))
        }
        WorkbookRequestEntry::Group(group) => {
            let group_child_ids = if let Some(child_ids) = &workspace.requests.child_ids {
                if let Some(group_child_ids) = child_ids.get(&group.id) {
                    group_child_ids.clone()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let mut runs: Vec<ApicizeExecutionGroupRun> = vec![];

            match group.multi_run_execution {
                WorkbookExecution::Sequential => {
                    for ctr in 0..number_of_runs {
                        runs.push(
                            run_request_group(
                                workspace.clone(),
                                cancellation_token.clone(),
                                tests_started.clone(),
                                group.execution.clone(),
                                group_child_ids.clone(),
                                ctr + 1,
                                variables.clone(),
                            )
                            .await,
                        )
                    }
                }
                WorkbookExecution::Concurrent => {
                    let mut executing_runs: JoinSet<Option<ApicizeExecutionGroupRun>> =
                        JoinSet::new();
                    for ctr in 0..number_of_runs {
                        let cloned_cancellation = cancellation_token.clone();
                        let executed_group = run_request_group(
                            workspace.clone(),
                            cancellation_token.clone(),
                            tests_started.clone(),
                            group.execution.clone(),
                            group_child_ids.clone(),
                            ctr + 1,
                            variables.clone(),
                        );
                        executing_runs.spawn(async move {
                            select! {
                                _ = cloned_cancellation.cancelled() => None,
                                result = executed_group => {
                                    Some(result)
                                }
                            }
                        });
                    }
                    for completed_run in executing_runs.join_all().await {
                        if let Some(r) = completed_run {
                            runs.push(r);
                        }
                        runs.sort_by_key(|r| r.run_number);
                    }
                }
            }

            let mut executed_group = ApicizeExecutionGroup {
                id: request_id,
                name: String::from(name),
                executed_at,
                duration: start_instant.elapsed().as_millis(),
                runs: vec![],
                success: true,
                requests_with_passed_tests_count: 0,
                requests_with_failed_tests_count: 0,
                requests_with_errors: 0,
                passed_test_count: 0,
                failed_test_count: 0,
            };

            for run in &runs {
                executed_group.add_totals(run);
            }
            executed_group.runs = runs;

            ApicizeExecutionItem::Group(Box::new(executed_group))
        }
    }
}

/// Execute the specified request's tests
fn execute_request_test(
    request: &WorkbookRequest,
    response: &ApicizeResponse,
    variables: &HashMap<String, Value>,
    tests_started: &Arc<Instant>,
) -> Result<Option<ApicizeTestResponse>, ApicizeError> {
    // Return empty test results if no test
    if request.test.is_none() {
        return Ok(None);
    }
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

    let mut init_code = String::new();
    init_code.push_str(&format!(
        "runTestSuite({}, {}, {}, {}, () => {{{}}})",
        serde_json::to_string(request).unwrap(),
        serde_json::to_string(response).unwrap(),
        serde_json::to_string(variables).unwrap(),
        std::time::UNIX_EPOCH.elapsed().unwrap().as_millis()
            - cloned_tests_started.elapsed().as_millis()
            + 1,
        request.test.as_ref().unwrap()
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

/// Dispatch the specified request and execute its tests
async fn execute_request_run(
    workspace: Arc<Workspace>,
    tests_started: Arc<Instant>,
    run_number: usize,
    request: Arc<WorkbookRequest>,
    request_as_entry: Arc<WorkbookRequestEntry>,
    variables: Arc<HashMap<String, Value>>,
) -> ApicizeExecutionRequestRun {
    let shared_workspace = workspace.clone();
    let shared_test_started = tests_started.clone();

    let executed_at = shared_test_started.elapsed().as_millis();
    let start_instant = Instant::now();

    let params = shared_workspace.retrieve_parameters(&request_as_entry, &variables);

    let dispatch_response = dispatch_request(
        &request,
        &params.variables,
        params.authorization,
        params.certificate,
        params.proxy,
        params.auth_certificate,
        params.auth_proxy,
    )
    .await;

    match dispatch_response {
        Ok((packaged_request, response)) => {
            let test_result = execute_request_test(
                &request.clone(),
                &response,
                &variables,
                &shared_test_started,
            );
            match test_result {
                Ok(test_response) => {
                    let mut test_count = 0;
                    let mut failed_test_count = 0;
                    let result_variables: Option<HashMap<String, Value>>;
                    let test_results = match test_response {
                        Some(response) => {
                            result_variables = Some(response.variables.clone());
                            if let Some(test_results) = &response.results {
                                test_count = test_results.len();
                                failed_test_count +=
                                    test_results.iter().filter(|r| !r.success).count();
                            }
                            response.results
                        }
                        None => {
                            result_variables = None;
                            None
                        }
                    };

                    ApicizeExecutionRequestRun {
                        run_number,
                        executed_at,
                        duration: start_instant.elapsed().as_millis(),
                        request: Some(packaged_request.clone()),
                        response: Some(response.clone()),
                        success: test_count == 0 || failed_test_count == 0,
                        error: None,
                        tests: test_results,
                        input_variables: if params.variables.is_empty() {
                            None
                        } else {
                            Some(params.variables.clone())
                        },
                        variables: result_variables,
                        requests_with_passed_tests_count: if test_count == 0
                            && failed_test_count == 0
                        {
                            1
                        } else {
                            0
                        },
                        requests_with_failed_tests_count: if failed_test_count > 0 { 1 } else { 0 },
                        requests_with_errors: 0,
                        passed_test_count: test_count - failed_test_count,
                        failed_test_count,
                    }
                }
                Err(err) => ApicizeExecutionRequestRun {
                    run_number,
                    executed_at,
                    duration: start_instant.elapsed().as_millis(),
                    request: Some(packaged_request.clone()),
                    response: Some(response.clone()),
                    success: false,
                    error: Some(err),
                    tests: None,
                    input_variables: None,
                    variables: None,
                    requests_with_passed_tests_count: 0,
                    requests_with_failed_tests_count: 0,
                    requests_with_errors: 1,
                    passed_test_count: 0,
                    failed_test_count: 0,
                },
            }
        }
        Err(err) => ApicizeExecutionRequestRun {
            run_number,
            executed_at,
            duration: start_instant.elapsed().as_millis(),
            request: None,
            response: None,
            success: false,
            error: Some(err),
            tests: None,
            input_variables: None,
            variables: None,
            requests_with_passed_tests_count: 0,
            requests_with_failed_tests_count: 0,
            requests_with_errors: 1,
            passed_test_count: 0,
            failed_test_count: 0,
        },
    }
}

/// Dispatch the specified request (via reqwest), returning either the repsonse or error
async fn dispatch_request(
    request: &WorkbookRequest,
    variables: &HashMap<String, Value>,
    authorization: Option<&WorkbookAuthorization>,
    certificate: Option<&WorkbookCertificate>,
    proxy: Option<&WorkbookProxy>,
    auth_certificate: Option<&WorkbookCertificate>,
    auth_proxy: Option<&WorkbookProxy>,
) -> Result<(ApicizeRequest, ApicizeResponse), ApicizeError> {
    let method = match request.method {
        Some(WorkbookRequestMethod::Get) => reqwest::Method::GET,
        Some(WorkbookRequestMethod::Post) => reqwest::Method::POST,
        Some(WorkbookRequestMethod::Put) => reqwest::Method::PUT,
        Some(WorkbookRequestMethod::Delete) => reqwest::Method::DELETE,
        Some(WorkbookRequestMethod::Head) => reqwest::Method::HEAD,
        Some(WorkbookRequestMethod::Options) => reqwest::Method::OPTIONS,
        None => reqwest::Method::GET,
        _ => panic!("Invalid method"),
    };

    let timeout: Duration;
    if let Some(t) = request.timeout {
        timeout = Duration::from_millis(t as u64);
    } else {
        timeout = Duration::from_secs(30);
    }

    // let keep_alive: bool;
    // if let Some(b) = request.keep_alive {
    //     keep_alive = b;
    // } else {
    //     keep_alive = true;
    // }

    let subs = &variables
        .iter()
        .map(|(name, value)| {
            let v = if let Some(s) = value.as_str() {
                String::from(s)
            } else {
                format!("{}", value)
            };
            (format!("{{{{{}}}}}", name), v)
        })
        .collect();

    // Build the reqwest client and request
    let mut reqwest_builder = Client::builder()
        // .http2_keep_alive_while_idle(keep_alive)
        .timeout(timeout);

    // Add certificate to builder if configured
    if let Some(active_cert) = certificate {
        match active_cert.append_to_builder(reqwest_builder) {
            Ok(updated_builder) => reqwest_builder = updated_builder,
            Err(err) => return Err(err),
        }
    }

    // Add proxy to builder if configured
    if let Some(active_proxy) = proxy {
        match active_proxy.append_to_builder(reqwest_builder) {
            Ok(updated_builder) => reqwest_builder = updated_builder,
            Err(err) => return Err(ApicizeError::from_reqwest(err)),
        }
    }

    let builder_result = reqwest_builder.build();
    let mut oauth2_token: Option<TokenResult> = None;

    match builder_result {
        Ok(client) => {
            let mut request_builder = client.request(
                method,
                WorkbookRequestEntry::clone_and_sub(request.url.as_str(), subs),
            );

            // Add headers, including authorization if applicable
            let mut headers = reqwest::header::HeaderMap::new();
            if let Some(h) = &request.headers {
                for nvp in h {
                    if nvp.disabled != Some(true) {
                        headers.insert(
                            reqwest::header::HeaderName::try_from(
                                WorkbookRequestEntry::clone_and_sub(&nvp.name, subs),
                            )
                            .unwrap(),
                            reqwest::header::HeaderValue::try_from(
                                WorkbookRequestEntry::clone_and_sub(&nvp.value, subs),
                            )
                            .unwrap(),
                        );
                    }
                }
            }

            match authorization {
                Some(WorkbookAuthorization::Basic {
                    username, password, ..
                }) => {
                    request_builder = request_builder.basic_auth(username, Some(password));
                }
                Some(WorkbookAuthorization::ApiKey { header, value, .. }) => {
                    headers.append(
                        reqwest::header::HeaderName::try_from(header).unwrap(),
                        reqwest::header::HeaderValue::try_from(value).unwrap(),
                    );
                }
                Some(WorkbookAuthorization::OAuth2Client {
                    id,
                    access_token_url,
                    client_id,
                    client_secret,
                    scope, // send_credentials_in_body: _,
                    ..
                }) => {
                    match oauth2::get_oauth2_client_credentials(
                        id,
                        access_token_url,
                        client_id,
                        client_secret,
                        scope,
                        auth_certificate,
                        auth_proxy,
                    )
                    .await
                    {
                        Ok(token_result) => {
                            request_builder = request_builder.bearer_auth(token_result.token.clone());
                            oauth2_token = Some(token_result);
                        }
                        Err(err) => return Err(err),
                    }
                },
                Some(WorkbookAuthorization::OAuth2Pkce { token, .. }) => {
                    match token {
                        Some(t) => {
                            request_builder = request_builder.bearer_auth(t.clone());
                        }
                        None => {
                            return Err(ApicizeError::Error {
                                description: String::from("PKCE authorization is not available in CLI test runner"),
                                source: None
                            });
                        }
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
                            WorkbookRequestEntry::clone_and_sub(&nvp.name, subs),
                            WorkbookRequestEntry::clone_and_sub(&nvp.value, subs),
                        ));
                    }
                }
                request_builder = request_builder.query(&query);
            }

            // Add body, if applicable
            match &request.body {
                Some(WorkbookRequestBody::Text { data }) => {
                    let s = WorkbookRequestEntry::clone_and_sub(data, subs);
                    request_builder = request_builder.body(Body::from(s.clone()));
                }
                Some(WorkbookRequestBody::JSON { data }) => {
                    let s = WorkbookRequestEntry::clone_and_sub(
                        serde_json::to_string(&data).unwrap().as_str(),
                        subs,
                    );
                    request_builder = request_builder.body(Body::from(s.clone()));
                }
                Some(WorkbookRequestBody::XML { data }) => {
                    let s = WorkbookRequestEntry::clone_and_sub(data, subs);
                    request_builder = request_builder.body(Body::from(s.clone()));
                }
                Some(WorkbookRequestBody::Form { data }) => {
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
                Some(WorkbookRequestBody::Raw { data }) => {
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
                                let (decoded, _, malformed) = request_encoding.decode(&data);
                                Some(ApicizeBody {
                                    data: Some(data.clone()),
                                    text: if malformed {
                                        None
                                    } else {
                                        Some(decoded.to_string())
                                    },
                                })
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
                            let headers =
                                if response_headers.is_empty() {
                                    None
                                } else {
                                    Some(HashMap::from_iter(
                                        response_headers
                                            .iter()
                                            .map(|(h, v)| {
                                                (
                                                    String::from(h.as_str()),
                                                    String::from(v.to_str().unwrap_or(
                                                        "(Header Contains Non-ASCII Data)",
                                                    )),
                                                )
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
                                        Some(ApicizeBody {
                                            data: Some(data.clone()),
                                            text: if malformed {
                                                None
                                            } else {
                                                Some(decoded.to_string())
                                            },
                                        })
                                    };

                                    let response = (
                                        ApicizeRequest {
                                            url: request_url,
                                            method: request
                                                .method
                                                .as_ref()
                                                .unwrap()
                                                .as_str()
                                                .to_string(),
                                            headers: request_headers,
                                            body: request_body,
                                            variables: if variables.is_empty() {
                                                None
                                            } else {
                                                Some(variables.clone())
                                            },
                                        },
                                        ApicizeResponse {
                                            status: status.as_u16(),
                                            status_text,
                                            headers,
                                            body: response_body,
                                            oauth2_token
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

/// Cleanup V8 platform, should only be called once at end of application
pub fn cleanup_v8() {
    unsafe {
        v8::V8::dispose();
    }
    v8::V8::dispose_platform();
}

#[cfg(test)]
mod tests {
    use mockito::Matcher;
    use serde_json::Value;
    use std::{
        collections::HashMap,
        sync::Arc,
        thread::sleep,
        time::{Duration, Instant},
    };
    use tokio::task::JoinSet;
    use tokio_util::sync::CancellationToken;

    use crate::{
        apicize::{ApicizeExecution, ApicizeExecutionItem, ApicizeResponse},
        oauth2_client_tokens::TokenResult,
        test_runner::{self, dispatch_request, execute_request_test},
        ApicizeError, IndexedEntities, IndexedRequests, WorkbookCertificate, WorkbookNameValuePair,
        WorkbookProxy, WorkbookRequest, WorkbookRequestEntry, WorkbookRequestMethod, Workspace,
    };

    use crate::oauth2_client_tokens::tests::MockOAuth2ClientTokens;

    #[tokio::test]
    async fn dispatch_requests_and_handles_bad_domain() {
        let request = WorkbookRequest {
            id: String::from(""),
            name: String::from("test"),
            url: String::from("https://foofooxxxxxx/"),
            method: Some(WorkbookRequestMethod::Post),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: None,
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };
        let response =
            dispatch_request(&request, &HashMap::new(), None, None, None, None, None).await;
        match &response {
            Ok(_) => {}
            Err(err) => {
                println!("{}: {}", err.get_label(), err);
            }
        }
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn dispatch_requests_and_handles_timeout() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_chunked_body(|_| {
                sleep(Duration::from_secs(1));
                Ok({})
            })
            .create();

        let request = WorkbookRequest {
            id: String::from(""),
            name: String::from("test"),
            url: server.url(),
            method: Some(WorkbookRequestMethod::Get),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: Some(1),
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };
        let response =
            dispatch_request(&request, &HashMap::new(), None, None, None, None, None).await;
        match &response {
            Ok(_) => {}
            Err(err) => {
                println!("{}: {}", err.get_label(), err);
            }
        }
        assert!(response.is_err());
        mock.assert();
    }

    #[tokio::test]
    async fn dispatch_requests_with_substituted_variables() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .match_query(Matcher::AllOf(vec![Matcher::UrlEncoded(
                "abc".into(),
                "123".into(),
            )]))
            .match_header("xxx", "zzz")
            .match_body("foo")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_body("ok")
            .create();

        let request = WorkbookRequest {
            id: String::from(""),
            name: String::from("test"),
            url: server.url() + "/{{page}}",
            method: Some(WorkbookRequestMethod::Post),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: None,
            keep_alive: None,
            runs: 1,
            headers: Some(vec![WorkbookNameValuePair {
                name: String::from("xxx"),
                value: String::from("{{xxx}}"),
                disabled: None,
            }]),
            query_string_params: Some(vec![WorkbookNameValuePair {
                name: String::from("abc"),
                value: String::from("{{abc}}"),
                disabled: None,
            }]),
            body: Some(crate::WorkbookRequestBody::Text {
                data: String::from("{{stuff}}"),
            }),
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let variables = HashMap::from([
            (String::from("page"), Value::from("test")),
            (String::from("abc"), Value::from("123")),
            (String::from("xxx"), Value::from("zzz")),
            (String::from("stuff"), Value::from("foo")),
        ]);
        let response = dispatch_request(&request, &variables, None, None, None, None, None).await;
        mock.assert();
        assert_eq!(response.unwrap().1.status, 200);
    }

    #[tokio::test]
    async fn dispatch_requests_with_basic_auth() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .match_header("Authorization", "Basic bmFtZTpzaGho")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_body("ok")
            .create();

        let request = WorkbookRequest {
            id: String::from(""),
            name: String::from("test"),
            url: server.url() + "/test",
            method: Some(WorkbookRequestMethod::Post),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: None,
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let response = dispatch_request(
            &request,
            &HashMap::new(),
            Some(&crate::WorkbookAuthorization::Basic {
                id: String::from(""),
                name: String::from(""),
                persistence: None,
                username: String::from("name"),
                password: String::from("shhh"),
            }),
            None,
            None,
            None,
            None,
        )
        .await;
        mock.assert();
        assert_eq!(response.unwrap().1.status, 200);
    }

    #[tokio::test]
    async fn dispatch_requests_with_api_key_auth() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .match_header("x-api-key", "abc")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_body("ok")
            .create();

        let request = WorkbookRequest {
            id: String::from(""),
            name: String::from("test"),
            url: server.url() + "/test",
            method: Some(WorkbookRequestMethod::Post),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: None,
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let response = dispatch_request(
            &request,
            &HashMap::new(),
            Some(&crate::WorkbookAuthorization::ApiKey {
                id: String::from(""),
                name: String::from(""),
                persistence: None,
                header: String::from("x-api-key"),
                value: String::from("abc"),
            }),
            None,
            None,
            None,
            None,
        )
        .await;
        mock.assert();
        assert_eq!(response.unwrap().1.status, 200);
    }

    #[tokio::test]
    async fn dispatch_requests_with_oauth2_auth() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .match_header("authorization", "Bearer ***TOKEN***")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_body("ok")
            .create();

        let request = WorkbookRequest {
            id: String::from(""),
            name: String::from("test"),
            url: server.url() + "/test",
            method: Some(WorkbookRequestMethod::Post),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: None,
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let oauth2_context = MockOAuth2ClientTokens::get_oauth2_client_credentials_context();
        oauth2_context
            .expect()
            .withf(
                |id, url, _client_id, _client_secret, _scope, _certificaite, _proxy| {
                    id == String::from("11111") && url == String::from("https://server")
                },
            )
            .returning(
                |_id: &str,
                 _token_url: &str,
                 _client_id: &str,
                 _client_secret: &str,
                 _scope: &Option<String>,
                 _certificate: Option<&WorkbookCertificate>,
                 _proxy: Option<&WorkbookProxy>| {
                    Ok(TokenResult {
                        token: String::from("***TOKEN***"),
                        cached: true,
                        url: None,
                        certificate: None,
                        proxy: None,
                    })
                },
            );

        let response = dispatch_request(
            &request,
            &HashMap::new(),
            Some(&crate::WorkbookAuthorization::OAuth2Client {
                id: String::from("11111"),
                name: String::from("My Token"),
                persistence: None,
                access_token_url: String::from("https://server"),
                client_id: String::from("me"),
                client_secret: String::from("shhh"),
                scope: Some(String::from("x")),
                selected_certificate: None,
                selected_proxy: None,
            }),
            None,
            None,
            None,
            None,
        )
        .await;
        mock.assert();
        assert_eq!(response.unwrap().1.status, 200);
    }

    #[tokio::test]
    async fn execute_request_test_runs_test() {
        let request = WorkbookRequest {
            id: String::from("xxx"),
            name: String::from("xxx"),
            test: Some(String::from("describe('test', () => { it('runs', () => { expect(response.status).to.equal(200) }) })")),
            url: String::from("http://foo"),
            method: Some(WorkbookRequestMethod::Get),
            timeout: Some(5000),
            headers: None,
            query_string_params: None,
            body: None,
            keep_alive: None,
            runs: 1,
            multi_run_execution: crate::WorkbookExecution::Sequential,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let response = ApicizeResponse {
            status: 200,
            status_text: String::from("Ok"),
            headers: None,
            body: None,
            oauth2_token: None,
        };

        let variables: HashMap<String, Value> = HashMap::new();

        let tests_started = Arc::new(Instant::now());

        let result = execute_request_test(&request, &response, &variables, &tests_started);

        let mut successes = 0;
        let mut failures = 0;
        for test_result in result.unwrap().unwrap().results.unwrap().iter() {
            // if let Some(logs) = &test_result.logs {
            //     println!("Logs: {}", logs.join("; "));
            // }
            // if let Some(error) = &test_result.error {
            //     println!("Error: {}", error);
            // }

            if test_result.success {
                successes += 1;
            } else {
                failures += 1;
            }
        }

        assert_eq!(successes, 1);
        assert_eq!(failures, 0);
    }

    #[tokio::test]
    async fn execute_request_test_includes_jsonpath() {
        let request = WorkbookRequest {
            id: String::from("xxx"),
            name: String::from("xxx"),
            test: Some(String::from("describe('test', () => { it('works', () => { var foo = { \"abc\": 123 }; expect(jsonpath('$.abc', foo)[0]).to.equal(123) }) })")),
            url: String::from("http://foo"),
            method: Some(WorkbookRequestMethod::Get),
            timeout: Some(5000),
            headers: None,
            query_string_params: None,
            body: None,
            keep_alive: None,
            runs: 1,
            multi_run_execution: crate::WorkbookExecution::Sequential,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let response = ApicizeResponse {
            status: 200,
            status_text: String::from("Ok"),
            headers: None,
            body: None,
            oauth2_token: None,
        };

        let variables: HashMap<String, Value> = HashMap::new();

        let tests_started = Arc::new(Instant::now());

        let result = execute_request_test(&request, &response, &variables, &tests_started);

        let mut successes = 0;
        let mut failures = 0;
        for test_result in result.unwrap().unwrap().results.unwrap().iter() {
            // if let Some(logs) = &test_result.logs {
            //     println!("Logs: {}", logs.join("; "));
            // }
            // if let Some(error) = &test_result.error {
            //     println!("Error: {}", error);
            // }
            if test_result.success {
                successes += 1;
            } else {
                failures += 1;
            }
        }

        assert_eq!(successes, 1);
        assert_eq!(failures, 0);
    }

    #[tokio::test]
    async fn execute_request_test_includes_xpath() {
        let request = WorkbookRequest {
            id: String::from("xxx"),
            name: String::from("xxx"),
            test: Some(String::from("describe('test', () => { it('works', () => { const xml = \"<foo><bar>test</bar></foo>\"; const doc = new dom().parseFromString(xml, 'text/xml'); expect(xpath.select('//bar', doc)[0].firstChild.data).to.equal('test') }) })")),
            url: String::from("http://foo"),
            method: Some(WorkbookRequestMethod::Get),
            timeout: Some(5000),
            headers: None,
            query_string_params: None,
            body: None,
            keep_alive: None,
            runs: 1,
            multi_run_execution: crate::WorkbookExecution::Sequential,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        };

        let response = ApicizeResponse {
            status: 200,
            status_text: String::from("Ok"),
            headers: None,
            body: None,
            oauth2_token: None,
        };

        let variables: HashMap<String, Value> = HashMap::new();

        let tests_started = Arc::new(Instant::now());

        let result = execute_request_test(&request, &response, &variables, &tests_started);

        let mut successes = 0;
        let mut failures = 0;
        for test_result in result.unwrap().unwrap().results.unwrap().iter() {
            // if let Some(logs) = &test_result.logs {
            //     println!("Logs: {}", logs.join("; "));
            // }
            // if let Some(error) = &test_result.error {
            //     println!("Error: {}", error);
            // }
            if test_result.success {
                successes += 1;
            } else {
                failures += 1;
            }
        }

        assert_eq!(successes, 1);
        assert_eq!(failures, 0);
    }

    async fn wait_and_cancel(
        cancellation: CancellationToken,
    ) -> Result<ApicizeExecution, ApicizeError> {
        sleep(Duration::from_millis(10));
        cancellation.cancel();
        Ok(ApicizeExecution {
            duration: 0,
            items: vec![],
            success: false,
            requests_with_passed_tests_count: 0,
            requests_with_failed_tests_count: 0,
            requests_with_errors: 0,
            passed_test_count: 0,
            failed_test_count: 0,
        })
    }

    #[tokio::test]
    async fn run_honors_override_number_of_runs() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_body("Ok")
            .create();

        let request = WorkbookRequestEntry::Info(WorkbookRequest {
            id: String::from("123"),
            name: String::from("test"),
            url: server.url(),
            method: Some(WorkbookRequestMethod::Get),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: Some(500),
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        });

        let workspace = Workspace {
            requests: IndexedRequests {
                top_level_ids: vec![String::from("123")],
                entities: HashMap::from([(String::from("123"), request)]),
                child_ids: None,
            },
            scenarios: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            authorizations: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            certificates: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            proxies: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            defaults: None,
            warnings: None,
        };

        let tests_started = Arc::new(Instant::now());
        let cancellation = CancellationToken::new();

        let attempt = test_runner::run(
            Arc::new(workspace),
            Some(vec![String::from("123")]),
            Some(cancellation.clone()),
            tests_started,
            Some(4),
        )
        .await;

        let runs = if let ApicizeExecutionItem::Request(result) =
            attempt.unwrap().items.first().unwrap()
        {
            result.runs.len()
        } else {
            0
        };
        assert_eq!(runs, 4)
    }

    #[tokio::test]
    async fn run_honors_cancel() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_chunked_body(|_| {
                sleep(Duration::from_secs(5000));
                Ok({})
            })
            .create();

        let request = WorkbookRequestEntry::Info(WorkbookRequest {
            id: String::from("123"),
            name: String::from("test"),
            url: server.url(),
            method: Some(WorkbookRequestMethod::Get),
            multi_run_execution: crate::WorkbookExecution::Sequential,
            timeout: Some(60000),
            keep_alive: None,
            runs: 1,
            headers: None,
            query_string_params: None,
            body: None,
            test: None,
            selected_scenario: None,
            selected_authorization: None,
            selected_certificate: None,
            selected_proxy: None,
            warnings: None,
        });

        let workspace = Workspace {
            requests: IndexedRequests {
                top_level_ids: vec![String::from("123")],
                entities: HashMap::from([(String::from("123"), request)]),
                child_ids: None,
            },
            scenarios: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            authorizations: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            certificates: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            proxies: IndexedEntities {
                top_level_ids: vec![],
                entities: HashMap::new(),
            },
            defaults: None,
            warnings: None,
        };

        let tests_started = Arc::new(Instant::now());
        let cancellation = CancellationToken::new();

        let mut results: JoinSet<Result<ApicizeExecution, ApicizeError>> = JoinSet::new();

        let attempt = test_runner::run(
            Arc::new(workspace),
            Some(vec![String::from("123")]),
            Some(cancellation.clone()),
            tests_started,
            None,
        );

        results.spawn(attempt);
        let cloned_cancellation = cancellation.clone();
        results.spawn(wait_and_cancel(cloned_cancellation));

        let completed_results = results.join_all().await;
        let has_cancelled_result = completed_results
            .iter()
            .any(|r| r.as_ref().is_err_and(|err| err.get_label() == "Cancelled"));
        assert!(has_cancelled_result);
    }
}

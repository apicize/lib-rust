use std::sync::Arc;

use apicize_lib::{
    ApicizeError, ApicizeExecution, ApicizeRequestResultContent, ApicizeResult, ApicizeRunner,
    ExecutionConcurrency, Identifiable, IndexedEntities, NameValuePair, PersistedIndex, Request,
    RequestBody, RequestEntry, RequestGroup, Scenario, ScenarioPlain, Selection,
    StoredRequestEntry, TestRunnerContext, TestRunnerContextInit, Variable, VariableSourceType,
    WorkbookDefaultParameters, Workspace, workspace::ParameterLockStatus,
};
use mockito::Matcher;
use serde_json::json;
use serial_test::serial;
use tokio_util::sync::CancellationToken;

/// Helper to build a minimal workspace with the given request entries
fn build_workspace(entries: Vec<RequestEntry>) -> Workspace {
    Workspace {
        private_lock_status: ParameterLockStatus::UnlockedNoPassword,
        vault_lock_status: ParameterLockStatus::UnlockedNoPassword,
        private_password: None,
        vault_password: None,
        requests: IndexedEntities::<RequestEntry>::new(&entries),
        scenarios: IndexedEntities::default(),
        authorizations: IndexedEntities::default(),
        certificates: IndexedEntities::default(),
        proxies: IndexedEntities::default(),
        data: IndexedEntities::default(),
        defaults: WorkbookDefaultParameters::default(),
        private_encryption: None,
        vault_encryption: None,
    }
}

/// Helper to create a basic request with a given ID and URL
fn make_request(id: &str, name: &str, url: &str) -> Request {
    Request {
        id: id.to_string(),
        name: name.to_string(),
        url: url.to_string(),
        test: None,
        ..Default::default()
    }
}

/// Helper to create a request with a test script
fn make_request_with_test(id: &str, name: &str, url: &str, test: &str) -> Request {
    Request {
        id: id.to_string(),
        name: name.to_string(),
        url: url.to_string(),
        test: Some(test.to_string()),
        ..Default::default()
    }
}

/// Helper to create a group with children
fn make_group(
    id: &str,
    name: &str,
    children: Vec<RequestEntry>,
    concurrency: ExecutionConcurrency,
) -> RequestGroup {
    RequestGroup {
        id: id.to_string(),
        name: name.to_string(),
        children: Some(children),
        execution: concurrency,
        ..Default::default()
    }
}

fn build_context(
    workspace: Workspace,
    cancellation: Option<CancellationToken>,
) -> Arc<TestRunnerContext> {
    Arc::new(TestRunnerContext::new(TestRunnerContextInit {
        workspace,
        cancellation,
        executing_request_or_group_id: "test-run",
        single_run_no_timeout: false,
        allowed_data_path: &None,
        enable_trace: false,
        execution_counter_callback: None,
    }))
}

// =============================================================================
// TestRunnerContext unit tests
// =============================================================================

#[test]
#[serial]
fn test_get_request_entry_valid() {
    let req = make_request("req-1", "Request 1", "http://example.com");
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);
    let entry = ctx.get_request_entry("req-1");
    assert!(entry.is_ok());
}

#[test]
#[serial]
fn test_get_request_entry_invalid_id() {
    let ws = build_workspace(vec![]);
    let ctx = build_context(ws, None);
    let entry = ctx.get_request_entry("nonexistent");
    assert!(entry.is_err());
    match entry.err().unwrap() {
        ApicizeError::InvalidId { description } => {
            assert!(description.contains("nonexistent"));
        }
        other => panic!("Expected InvalidId error, got: {}", other),
    }
}

#[test]
#[serial]
fn test_get_request_valid() {
    let req = make_request("req-1", "Request 1", "http://example.com");
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);
    let result = ctx.get_request("req-1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_id(), "req-1");
}

#[test]
#[serial]
fn test_get_request_on_group_returns_error() {
    let group = make_group("grp-1", "Group 1", vec![], ExecutionConcurrency::Sequential);
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);
    let result = ctx.get_request("grp-1");
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_get_group_valid() {
    let group = make_group("grp-1", "Group 1", vec![], ExecutionConcurrency::Sequential);
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);
    let result = ctx.get_group("grp-1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_id(), "grp-1");
}

#[test]
#[serial]
fn test_get_group_on_request_returns_error() {
    let req = make_request("req-1", "Request 1", "http://example.com");
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);
    let result = ctx.get_group("req-1");
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_get_group_children_with_children() {
    let child1 = make_request("child-1", "Child 1", "http://example.com/1");
    let child2 = make_request("child-2", "Child 2", "http://example.com/2");
    let group = make_group(
        "grp-1",
        "Group 1",
        vec![RequestEntry::Request(child1), RequestEntry::Request(child2)],
        ExecutionConcurrency::Sequential,
    );
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);
    let children = ctx.get_group_children("grp-1");
    assert_eq!(children.len(), 2);
    assert!(children.contains(&"child-1".to_string()));
    assert!(children.contains(&"child-2".to_string()));
}

#[test]
#[serial]
fn test_get_group_children_no_children() {
    let group = make_group("grp-1", "Group 1", vec![], ExecutionConcurrency::Sequential);
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);
    let children = ctx.get_group_children("grp-1");
    assert!(children.is_empty());
}

#[test]
#[serial]
fn test_get_group_children_nonexistent_group() {
    let ws = build_workspace(vec![]);
    let ctx = build_context(ws, None);
    let children = ctx.get_group_children("nonexistent");
    assert!(children.is_empty());
}

#[test]
#[serial]
fn test_get_request_key_inherits_from_parent() {
    let child = make_request("child-1", "Child", "http://example.com");
    let mut group = make_group(
        "grp-1",
        "Group 1",
        vec![RequestEntry::Request(child)],
        ExecutionConcurrency::Sequential,
    );
    group.key = Some("group-key".to_string());
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);
    let key = ctx.get_request_key("child-1").unwrap();
    assert_eq!(key, Some("group-key".to_string()));
}

#[test]
#[serial]
fn test_get_request_key_own_key() {
    let mut req = make_request("req-1", "Request 1", "http://example.com");
    req.key = Some("my-key".to_string());
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);
    let key = ctx.get_request_key("req-1").unwrap();
    assert_eq!(key, Some("my-key".to_string()));
}

#[test]
#[serial]
fn test_get_request_key_none() {
    let req = make_request("req-1", "Request 1", "http://example.com");
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);
    let key = ctx.get_request_key("req-1").unwrap();
    assert_eq!(key, None);
}

#[test]
#[serial]
fn test_ellapsed_in_ms() {
    let ws = build_workspace(vec![]);
    let ctx = build_context(ws, None);
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(ctx.ellapsed_in_ms() >= 10);
}

// =============================================================================
// Integration tests: run() with a mock HTTP server
// =============================================================================

#[tokio::test]
#[serial]
async fn test_run_single_request_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "hello"}"#)
        .create_async()
        .await;

    let req = make_request(
        "req-1",
        "Test Request",
        &format!("{}/api/test", server.url()),
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    let result = results.into_iter().next().unwrap().unwrap();
    match result {
        ApicizeResult::Request(req_result) => {
            assert!(req_result.success);
            assert_eq!(req_result.request_error_count, 0);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_run_single_request_with_passing_test() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"value": 42}"#)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "Test Request",
        &format!("{}/api/test", server.url()),
        r#"
        describe('response', () => {
            it('should have status 200', () => {
                expect(response.status).to.equal(200)
            })
        })
        "#,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let result = results.into_iter().next().unwrap().unwrap();
    match result {
        ApicizeResult::Request(req_result) => {
            assert!(req_result.success);
            assert_eq!(req_result.test_pass_count, 1);
            assert_eq!(req_result.test_fail_count, 0);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_run_single_request_with_failing_test() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(404)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "Test Request",
        &format!("{}/api/test", server.url()),
        r#"
        describe('response', () => {
            it('should have status 200', () => {
                expect(response.status).to.equal(200)
            })
        })
        "#,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let result = results.into_iter().next().unwrap().unwrap();
    match result {
        ApicizeResult::Request(req_result) => {
            assert!(!req_result.success);
            assert_eq!(req_result.test_pass_count, 0);
            assert_eq!(req_result.test_fail_count, 1);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_run_request_invalid_id_returns_error() {
    let ws = build_workspace(vec![]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["nonexistent".to_string()]).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
}

// =============================================================================
// Cancellation tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_cancellation_stops_execution() {
    let mut server = mockito::Server::new_async().await;
    // Use a delay to simulate a slow response
    let _mock = server
        .mock("GET", "/api/slow")
        .with_status(200)
        .with_body("ok")
        .create_async()
        .await;

    let cancel = CancellationToken::new();
    let req = make_request(
        "req-1",
        "Slow Request",
        &format!("{}/api/slow", server.url()),
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, Some(cancel.clone()));

    // Cancel before execution starts
    cancel.cancel();

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match &results[0] {
        Err(ApicizeError::Cancelled) => {}
        other => panic!(
            "Expected Cancelled error, got an unexpected result: {}",
            match other {
                Ok(_) => "Ok(...)".to_string(),
                Err(e) => format!("Err({})", e),
            }
        ),
    }
}

// =============================================================================
// Disabled request/group tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_disabled_request_skipped_in_group() {
    let mut server = mockito::Server::new_async().await;
    let mock_enabled = server
        .mock("GET", "/enabled")
        .with_status(200)
        .create_async()
        .await;
    let mock_disabled = server
        .mock("GET", "/disabled")
        .with_status(200)
        .expect(0) // Should NOT be called
        .create_async()
        .await;

    let enabled_req = make_request("req-1", "Enabled", &format!("{}/enabled", server.url()));
    let mut disabled_req = make_request("req-2", "Disabled", &format!("{}/disabled", server.url()));
    disabled_req.disabled = true;

    let group = make_group(
        "grp-1",
        "Group 1",
        vec![
            RequestEntry::Request(enabled_req),
            RequestEntry::Request(disabled_req),
        ],
        ExecutionConcurrency::Sequential,
    );
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            // Only the enabled request should have run
            assert_eq!(group_result.request_success_count, 1);
        }
        _ => panic!("Expected Group result"),
    }
    mock_enabled.assert_async().await;
    mock_disabled.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_disabled_request_runs_when_force_run() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let mut req = make_request(
        "req-1",
        "Disabled Direct",
        &format!("{}/api/test", server.url()),
    );
    req.disabled = true;

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    // When running directly (top-level), force_run=true so disabled is ignored
    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    mock.assert_async().await;
}

// =============================================================================
// Sequential group execution tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_sequential_group_execution() {
    let mut server = mockito::Server::new_async().await;
    let mock1 = server
        .mock("GET", "/api/one")
        .with_status(200)
        .create_async()
        .await;
    let mock2 = server
        .mock("GET", "/api/two")
        .with_status(200)
        .create_async()
        .await;

    let req1 = make_request("req-1", "Request 1", &format!("{}/api/one", server.url()));
    let req2 = make_request("req-2", "Request 2", &format!("{}/api/two", server.url()));
    let group = make_group(
        "grp-1",
        "Group 1",
        vec![RequestEntry::Request(req1), RequestEntry::Request(req2)],
        ExecutionConcurrency::Sequential,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.request_success_count, 2);
        }
        _ => panic!("Expected Group result"),
    }
    mock1.assert_async().await;
    mock2.assert_async().await;
}

// =============================================================================
// Concurrent group execution tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_group_execution() {
    let mut server = mockito::Server::new_async().await;
    let mock1 = server
        .mock("GET", "/api/one")
        .with_status(200)
        .create_async()
        .await;
    let mock2 = server
        .mock("GET", "/api/two")
        .with_status(200)
        .create_async()
        .await;
    let mock3 = server
        .mock("GET", "/api/three")
        .with_status(200)
        .create_async()
        .await;

    let req1 = make_request("req-1", "Request 1", &format!("{}/api/one", server.url()));
    let req2 = make_request("req-2", "Request 2", &format!("{}/api/two", server.url()));
    let req3 = make_request("req-3", "Request 3", &format!("{}/api/three", server.url()));
    let group = make_group(
        "grp-1",
        "Group 1",
        vec![
            RequestEntry::Request(req1),
            RequestEntry::Request(req2),
            RequestEntry::Request(req3),
        ],
        ExecutionConcurrency::Concurrent,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.request_success_count, 3);
        }
        _ => panic!("Expected Group result"),
    }
    mock1.assert_async().await;
    mock2.assert_async().await;
    mock3.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_concurrent_group_preserves_order() {
    let mut server = mockito::Server::new_async().await;
    let _mock1 = server
        .mock("GET", "/api/one")
        .with_status(200)
        .create_async()
        .await;
    let _mock2 = server
        .mock("GET", "/api/two")
        .with_status(200)
        .create_async()
        .await;

    let req1 = make_request("req-1", "Request 1", &format!("{}/api/one", server.url()));
    let req2 = make_request("req-2", "Request 2", &format!("{}/api/two", server.url()));
    let group = make_group(
        "grp-1",
        "Group 1",
        vec![RequestEntry::Request(req1), RequestEntry::Request(req2)],
        ExecutionConcurrency::Concurrent,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            // Verify results are sorted by child_ids order even with concurrent execution
            match &group_result.content {
                apicize_lib::ApicizeGroupResultContent::Results { results } => {
                    assert_eq!(results.len(), 2);
                    assert_eq!(results[0].get_id(), "req-1");
                    assert_eq!(results[1].get_id(), "req-2");
                }
                _ => panic!("Expected Results content"),
            }
        }
        _ => panic!("Expected Group result"),
    }
}

// =============================================================================
// Concurrent request runs tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_request_runs() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .expect(3) // Should be called 3 times
        .create_async()
        .await;

    let mut req = make_request("req-1", "Multi Run", &format!("{}/api/test", server.url()));
    req.runs = 3;
    req.multi_run_execution = ExecutionConcurrency::Concurrent;

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(req_result.success);
            assert_eq!(req_result.request_success_count, 3);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_sequential_request_runs() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .expect(3)
        .create_async()
        .await;

    let mut req = make_request("req-1", "Multi Run", &format!("{}/api/test", server.url()));
    req.runs = 3;
    req.multi_run_execution = ExecutionConcurrency::Sequential;

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(req_result.success);
            assert_eq!(req_result.request_success_count, 3);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// Zero runs edge case
// =============================================================================

#[tokio::test]
#[serial]
async fn test_request_zero_runs_returns_none() {
    let mut req = make_request("req-1", "Zero Runs", "http://example.com");
    req.runs = 0;

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    // Zero runs produces no results
    assert!(results.is_empty());
}

#[tokio::test]
#[serial]
async fn test_group_zero_runs_returns_none() {
    let mut group = make_group(
        "grp-1",
        "Zero Runs Group",
        vec![],
        ExecutionConcurrency::Sequential,
    );
    group.runs = 0;

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert!(results.is_empty());
}

// =============================================================================
// Concurrent group runs tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_group_runs() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .expect(6) // 2 children x 3 runs
        .create_async()
        .await;

    let req1 = make_request("req-1", "Request 1", &format!("{}/api/test", server.url()));
    let req2 = make_request("req-2", "Request 2", &format!("{}/api/test", server.url()));
    let mut group = make_group(
        "grp-1",
        "Group 1",
        vec![RequestEntry::Request(req1), RequestEntry::Request(req2)],
        ExecutionConcurrency::Sequential,
    );
    group.runs = 3;
    group.multi_run_execution = ExecutionConcurrency::Concurrent;

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            // 3 runs * 2 requests = 6 successful requests
            assert_eq!(group_result.request_success_count, 6);
        }
        _ => panic!("Expected Group result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// Error handling tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_request_to_unreachable_host_produces_error() {
    // Use a URL that will fail to connect
    let mut req = make_request("req-1", "Unreachable", "http://192.0.2.1:1/unreachable");
    req.timeout = Some(500); // Short timeout so test doesn't hang

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(!req_result.success);
            assert_eq!(req_result.request_error_count, 1);
        }
        _ => panic!("Expected Request result"),
    }
}

#[tokio::test]
#[serial]
async fn test_request_with_empty_url_returns_error() {
    let req = make_request("req-1", "Empty URL", "");
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(!req_result.success);
            assert_eq!(req_result.request_error_count, 1);
        }
        _ => panic!("Expected Request result"),
    }
}

// =============================================================================
// Multiple top-level requests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_run_multiple_top_level_requests() {
    let mut server = mockito::Server::new_async().await;
    let mock1 = server
        .mock("GET", "/api/one")
        .with_status(200)
        .create_async()
        .await;
    let mock2 = server
        .mock("GET", "/api/two")
        .with_status(200)
        .create_async()
        .await;

    let req1 = make_request("req-1", "Request 1", &format!("{}/api/one", server.url()));
    let req2 = make_request("req-2", "Request 2", &format!("{}/api/two", server.url()));
    let ws = build_workspace(vec![
        RequestEntry::Request(req1),
        RequestEntry::Request(req2),
    ]);
    let ctx = build_context(ws, None);

    let results = ctx
        .run(vec!["req-1".to_string(), "req-2".to_string()])
        .await;
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_ok());
    mock1.assert_async().await;
    mock2.assert_async().await;
}

// =============================================================================
// Nested group tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_nested_groups() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let req = make_request("req-1", "Request 1", &format!("{}/api/test", server.url()));
    let inner_group = make_group(
        "inner-grp",
        "Inner Group",
        vec![RequestEntry::Request(req)],
        ExecutionConcurrency::Sequential,
    );
    let outer_group = make_group(
        "outer-grp",
        "Outer Group",
        vec![RequestEntry::Group(inner_group)],
        ExecutionConcurrency::Sequential,
    );

    let ws = build_workspace(vec![RequestEntry::Group(outer_group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["outer-grp".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.request_success_count, 1);
        }
        _ => panic!("Expected Group result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// Concurrent cancellation during multi-run
// =============================================================================

#[tokio::test]
#[serial]
async fn test_cancellation_during_concurrent_runs() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let cancel = CancellationToken::new();
    let mut req = make_request("req-1", "Multi Run", &format!("{}/api/test", server.url()));
    req.runs = 100;
    req.multi_run_execution = ExecutionConcurrency::Concurrent;

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, Some(cancel.clone()));

    // Cancel immediately
    cancel.cancel();

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
    match &results[0] {
        Err(ApicizeError::Cancelled) => {}
        other => panic!(
            "Expected Cancelled error, got an unexpected result: {}",
            match other {
                Ok(_) => "Ok(...)".to_string(),
                Err(e) => format!("Err({})", e),
            }
        ),
    }
}

#[tokio::test]
#[serial]
async fn test_cancellation_during_concurrent_group_runs() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let cancel = CancellationToken::new();
    let req = make_request("req-1", "Request 1", &format!("{}/api/test", server.url()));
    let mut group = make_group(
        "grp-1",
        "Group 1",
        vec![RequestEntry::Request(req)],
        ExecutionConcurrency::Sequential,
    );
    group.runs = 100;
    group.multi_run_execution = ExecutionConcurrency::Concurrent;

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, Some(cancel.clone()));

    cancel.cancel();

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
}

// =============================================================================
// Concurrent group with mixed success/failure
// =============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_group_mixed_results() {
    let mut server = mockito::Server::new_async().await;
    let mock_success = server
        .mock("GET", "/api/success")
        .with_status(200)
        .create_async()
        .await;
    let mock_fail = server
        .mock("GET", "/api/fail")
        .with_status(500)
        .create_async()
        .await;

    let req_ok = make_request_with_test(
        "req-ok",
        "Success",
        &format!("{}/api/success", server.url()),
        r#"
        describe('test', () => {
            it('passes', () => {
                expect(response.status).to.equal(200)
            })
        })
        "#,
    );
    let req_fail = make_request_with_test(
        "req-fail",
        "Failure",
        &format!("{}/api/fail", server.url()),
        r#"
        describe('test', () => {
            it('should be 200', () => {
                expect(response.status).to.equal(200)
            })
        })
        "#,
    );

    let group = make_group(
        "grp-1",
        "Mixed Group",
        vec![
            RequestEntry::Request(req_ok),
            RequestEntry::Request(req_fail),
        ],
        ExecutionConcurrency::Concurrent,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(!group_result.success);
            assert_eq!(group_result.request_success_count, 1);
            assert_eq!(group_result.request_failure_count, 1);
            assert_eq!(group_result.test_pass_count, 1);
            assert_eq!(group_result.test_fail_count, 1);
        }
        _ => panic!("Expected Group result"),
    }
    mock_success.assert_async().await;
    mock_fail.assert_async().await;
}

// =============================================================================
// flatten_test_results (indirectly tested through test execution)
// =============================================================================

#[tokio::test]
#[serial]
async fn test_nested_describe_blocks_flatten_names() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"value": 1}"#)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "Test Request",
        &format!("{}/api/test", server.url()),
        r#"
        describe('outer', () => {
            describe('inner', () => {
                it('should pass', () => {
                    expect(response.status).to.equal(200)
                })
            })
        })
        "#,
    );

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(req_result.success);
            assert_eq!(req_result.test_pass_count, 1);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// HTTP method tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_post_request() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/api/create")
        .with_status(201)
        .create_async()
        .await;

    let mut req = make_request(
        "req-1",
        "POST Request",
        &format!("{}/api/create", server.url()),
    );
    req.method = Some("POST".to_string());

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    mock.assert_async().await;
}

// =============================================================================
// V8 test script error handling (panic-prone areas)
// =============================================================================

#[tokio::test]
#[serial]
async fn test_invalid_javascript_test_script_produces_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "Bad JS",
        &format!("{}/api/test", server.url()),
        // Invalid JS that should cause a compilation/runtime error
        r#"
        this is not valid javascript {{{{
        "#,
    );

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            // Should have an error, not a panic
            assert!(!req_result.success);
            assert_eq!(req_result.request_error_count, 1);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_test_script_throwing_exception_produces_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "Throwing Test",
        &format!("{}/api/test", server.url()),
        r#"
        throw new Error("intentional error");
        "#,
    );

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(!req_result.success);
            assert_eq!(req_result.request_error_count, 1);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// Concurrent safety: multiple requests sharing Arc<TestRunnerContext>
// =============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_access_to_shared_context() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .expect(5)
        .create_async()
        .await;

    let mut entries = Vec::new();
    for i in 0..5 {
        let req = make_request(
            &format!("req-{}", i),
            &format!("Request {}", i),
            &format!("{}/api/test", server.url()),
        );
        entries.push(RequestEntry::Request(req));
    }

    let group = make_group(
        "grp-1",
        "Concurrent Group",
        entries,
        ExecutionConcurrency::Concurrent,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.request_success_count, 5);
        }
        _ => panic!("Expected Group result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// Concurrent V8 isolates: ensure parallel test scripts don't interfere
// =============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_v8_isolates_are_independent() {
    let mut server = mockito::Server::new_async().await;
    let mock_200 = server
        .mock("GET", "/api/ok")
        .with_status(200)
        .expect(3)
        .create_async()
        .await;
    let mock_404 = server
        .mock("GET", "/api/notfound")
        .with_status(404)
        .expect(2)
        .create_async()
        .await;

    let mut entries = Vec::new();
    for i in 0..3 {
        let req = make_request_with_test(
            &format!("req-ok-{}", i),
            &format!("OK {}", i),
            &format!("{}/api/ok", server.url()),
            r#"
            describe('status', () => {
                it('equals 200', () => {
                    expect(response.status).to.equal(200)
                })
            })
            "#,
        );
        entries.push(RequestEntry::Request(req));
    }
    for i in 0..2 {
        let req = make_request_with_test(
            &format!("req-nf-{}", i),
            &format!("NotFound {}", i),
            &format!("{}/api/notfound", server.url()),
            r#"
            describe('status', () => {
                it('equals 404', () => {
                    expect(response.status).to.equal(404)
                })
            })
            "#,
        );
        entries.push(RequestEntry::Request(req));
    }

    let group = make_group(
        "grp-1",
        "Mixed V8 Group",
        entries,
        ExecutionConcurrency::Concurrent,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.test_pass_count, 5);
            assert_eq!(group_result.test_fail_count, 0);
        }
        _ => panic!("Expected Group result"),
    }
    mock_200.assert_async().await;
    mock_404.assert_async().await;
}

// =============================================================================
// Concurrent request + group runs with cancellation race
// =============================================================================

#[tokio::test]
#[serial]
async fn test_cancellation_race_with_concurrent_group_children() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let cancel = CancellationToken::new();

    let mut entries = Vec::new();
    for i in 0..10 {
        let req = make_request(
            &format!("req-{}", i),
            &format!("Request {}", i),
            &format!("{}/api/test", server.url()),
        );
        entries.push(RequestEntry::Request(req));
    }

    let group = make_group(
        "grp-1",
        "Cancel Race Group",
        entries,
        ExecutionConcurrency::Concurrent,
    );

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, Some(cancel.clone()));

    // Cancel mid-flight
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        cancel_clone.cancel();
    });

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    // Should either complete or be cancelled - must not panic
    assert_eq!(results.len(), 1);
}

// =============================================================================
// Empty group
// =============================================================================

#[tokio::test]
#[serial]
async fn test_empty_group() {
    let group = make_group(
        "grp-1",
        "Empty Group",
        vec![],
        ExecutionConcurrency::Sequential,
    );
    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    assert_eq!(results.len(), 1);
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.request_success_count, 0);
        }
        _ => panic!("Expected Group result"),
    }
}

// =============================================================================
// Test script accessing response body
// =============================================================================

#[tokio::test]
#[serial]
async fn test_script_can_access_json_response_body() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/data")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"value": 42}"#)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "JSON Body Test",
        &format!("{}/api/data", server.url()),
        r#"
        describe('body', () => {
            it('has expected value', () => {
                expect(response.body.data.value).to.equal(42)
            })
        })
        "#,
    );

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Request(req_result) => {
            assert!(req_result.success);
            assert_eq!(req_result.test_pass_count, 1);
            assert_eq!(req_result.test_fail_count, 0);
        }
        _ => panic!("Expected Request result"),
    }
    mock.assert_async().await;
}

// =============================================================================
// Setup script tests
// =============================================================================

/// Helper to create a request with setup and (optional) test scripts
fn make_request_with_setup(
    id: &str,
    name: &str,
    url: &str,
    setup: &str,
    test: Option<&str>,
) -> Request {
    Request {
        id: id.to_string(),
        name: name.to_string(),
        url: url.to_string(),
        setup: Some(setup.to_string()),
        test: test.map(|t| t.to_string()),
        ..Default::default()
    }
}

/// Helper to extract the single execution from a request result
fn get_execution(result: ApicizeResult) -> Box<ApicizeExecution> {
    match result {
        ApicizeResult::Request(req_result) => match req_result.content {
            ApicizeRequestResultContent::Execution { execution } => execution,
            _ => panic!("Expected Execution content"),
        },
        _ => panic!("Expected Request result"),
    }
}

#[tokio::test]
#[serial]
async fn test_setup_modifies_request() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/api/modified")
        .match_header("x-setup", "added")
        .match_header("accept", "text/plain")
        .match_query(Matcher::UrlEncoded("b".into(), "2".into()))
        .match_body(Matcher::Json(json!({"value": 2, "extra": true})))
        .with_status(200)
        .create_async()
        .await;

    let mut req = make_request_with_setup(
        "req-1",
        "Setup Request",
        &format!("{}/api/original", server.url()),
        r#"
        request.url = request.url.replace('/original', '/modified')
        request.method = 'POST'
        request.setHeader('X-Setup', 'added')
        request.setHeader('ACCEPT', 'text/plain')
        request.removeQueryParam('a')
        request.setQueryParam('b', 2)
        const body = JSON.parse(request.body.data)
        body.value += 1
        body.extra = true
        request.body.data = body
        "#,
        None,
    );
    req.headers = Some(vec![NameValuePair {
        name: "Accept".to_string(),
        value: "application/json".to_string(),
        disabled: None,
    }]);
    req.query_string_params = Some(vec![NameValuePair {
        name: "a".to_string(),
        value: "1".to_string(),
        disabled: None,
    }]);
    req.body = Some(RequestBody::JSON {
        data: r#"{"value": 1}"#.to_string(),
    });

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(execution.error.is_none());
    assert!(execution.success);
    assert_eq!(execution.method.as_deref(), Some("POST"));
    let sent = execution.test_context.request.as_ref().unwrap();
    assert_eq!(sent.url, format!("{}/api/modified?b=2", server.url()));
    assert_eq!(
        sent.headers.get("x-setup").map(|s| s.as_str()),
        Some("added")
    );
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_sets_headers_by_name() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/named")
        .match_header("foo", "888")
        .match_query(Matcher::UrlEncoded("q".into(), "x".into()))
        .with_status(200)
        .create_async()
        .await;

    let req = make_request_with_setup(
        "req-1",
        "Setup Request",
        &format!("{}/api/named", server.url()),
        r#"
        request.headers['foo'] = '888'
        request.queryStringParams['q'] = 'x'
        "#,
        None,
    );

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(execution.error.is_none());
    let sent = execution.test_context.request.as_ref().unwrap();
    assert_eq!(sent.headers.get("foo").map(|s| s.as_str()), Some("888"));
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_output_used_for_substitution_and_test() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/abc")
        .match_header("x-sig", "sig-123")
        .with_status(200)
        .create_async()
        .await;

    let mut req = make_request_with_setup(
        "req-1",
        "Setup Output",
        &format!("{}/api/{{{{path}}}}", server.url()),
        r#"
        console.log('setting output')
        output('path', 'abc')
        output('sig', `sig-${123}`)
        "#,
        Some(
            r#"
            console.log('testing')
            describe('output', () => {
                it('is available to test', () => {
                    expect($.sig).to.equal('sig-123')
                })
            })
            "#,
        ),
    );
    req.headers = Some(vec![NameValuePair {
        name: "X-Sig".to_string(),
        value: "{{sig}}".to_string(),
        disabled: None,
    }]);

    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(execution.error.is_none(), "{:?}", execution.error);
    assert!(execution.success);
    assert_eq!(execution.test_pass_count, 1);

    let output = execution.output_variables.as_ref().unwrap();
    assert_eq!(output.get("path"), Some(&json!("abc")));
    assert_eq!(output.get("sig"), Some(&json!("sig-123")));

    // Setup logs come before test logs
    let logs = execution.logs.as_ref().unwrap();
    assert_eq!(logs.len(), 2);
    assert!(logs[0].ends_with("[log] setting output"));
    assert!(logs[1].ends_with("[log] testing"));
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_error_skips_dispatch() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .expect(0)
        .create_async()
        .await;

    let req = make_request_with_setup(
        "req-1",
        "Failing Setup",
        &format!("{}/api/test", server.url()),
        r#"
        console.log('before')
        throw new Error('boom')
        "#,
        Some("describe('x', () => { it('runs', () => {}) })"),
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(!execution.success);
    assert!(execution.tests.is_none());
    assert_eq!(
        execution.error,
        Some(ApicizeError::FailedSetup {
            description: "boom".to_string()
        })
    );
    let logs = execution.logs.as_ref().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].ends_with("[log] before"));
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_syntax_error() {
    let req = make_request_with_setup(
        "req-1",
        "Bad Setup",
        "http://localhost:1/api/test",
        "this is not javascript (",
        None,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(!execution.success);
    assert!(matches!(
        execution.error,
        Some(ApicizeError::FailedSetup { .. })
    ));
}

#[tokio::test]
#[serial]
async fn test_setup_does_not_expose_test_functions() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/test")
        .with_status(200)
        .create_async()
        .await;

    let req = make_request_with_setup(
        "req-1",
        "Setup Globals",
        &format!("{}/api/test", server.url()),
        r#"
        for (const name of ['describe', 'it', 'tag', 'expect', 'assert', 'should']) {
            if (typeof globalThis[name] !== 'undefined') {
                throw new Error(`${name} should not be available`)
            }
        }
        for (const name of ['output', 'jsonpath', 'console', 'BodyType']) {
            if (typeof globalThis[name] === 'undefined') {
                throw new Error(`${name} should be available`)
            }
        }
        "#,
        None,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(execution.error.is_none(), "{:?}", execution.error);
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_invalid_header_returns_error() {
    let req = make_request_with_setup(
        "req-1",
        "Invalid Header",
        "http://localhost:1/api/test",
        "request.setHeader('bad header', 'x')",
        None,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(!execution.success);
    match &execution.error {
        Some(ApicizeError::Http { description, .. }) => {
            assert_eq!(description, "Invalid header name: bad header")
        }
        _ => panic!("Expected HTTP error"),
    }
}

#[tokio::test]
#[serial]
async fn test_group_setup_output_used_by_children() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/from-group")
        .with_status(200)
        .create_async()
        .await;

    let req = make_request_with_test(
        "req-1",
        "Child",
        &format!("{}/api/{{{{path}}}}", server.url()),
        r#"
        describe('group output', () => {
            it('is available', () => {
                expect($.path).to.equal('from-group')
            })
        })
        "#,
    );
    let mut group = make_group(
        "grp-1",
        "Group",
        vec![RequestEntry::Request(req)],
        ExecutionConcurrency::Sequential,
    );
    group.setup = Some("output('path', 'from-group')".to_string());

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.test_pass_count, 1);
        }
        _ => panic!("Expected Group result"),
    }
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_group_setup_error() {
    let mut group = make_group(
        "grp-1",
        "Group",
        vec![RequestEntry::Request(make_request(
            "req-1",
            "Child",
            "http://localhost:1/",
        ))],
        ExecutionConcurrency::Sequential,
    );
    group.setup = Some("throw new Error('group boom')".to_string());

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    let result = results.into_iter().next().unwrap();
    assert!(matches!(
        result,
        Err(ApicizeError::FailedSetup { description }) if description == "group boom"
    ));
}

#[test]
fn test_stored_request_setup_round_trip() {
    let req = make_request_with_setup("req-1", "Stored", "http://x", "output('a', 1)", None);
    let stored = StoredRequestEntry::from(RequestEntry::Request(req));
    let json = serde_json::to_string(&stored).unwrap();
    assert!(json.contains(r#""setup":"output('a', 1)""#));
    let restored = RequestEntry::from(serde_json::from_str::<StoredRequestEntry>(&json).unwrap());
    match restored {
        RequestEntry::Request(r) => assert_eq!(r.setup.as_deref(), Some("output('a', 1)")),
        _ => panic!("Expected request"),
    }
}

#[tokio::test]
#[serial]
async fn test_setup_script_is_not_substituted() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/modified")
        .with_status(200)
        .create_async()
        .await;

    // Group output "path" is available before the request's setup runs; the setup script's
    // literal "{{path}}" strings should not be substituted
    let req = make_request_with_setup(
        "req-1",
        "Child",
        &format!("{}/api/{{{{path}}}}", server.url()),
        r#"
        output('literal', '{{path}}')
        request.url = request.url.replace('{{path}}', 'modified')
        "#,
        Some(
            r#"
            describe('setup', () => {
                it('was not substituted', () => {
                    expect($.literal).to.equal('{{' + 'path}}')
                    expect($.path).to.equal('from-group')
                })
            })
            "#,
        ),
    );
    let mut group = make_group(
        "grp-1",
        "Group",
        vec![RequestEntry::Request(req)],
        ExecutionConcurrency::Sequential,
    );
    group.setup = Some("output('path', 'from-group')".to_string());

    let ws = build_workspace(vec![RequestEntry::Group(group)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.test_pass_count, 1);
        }
        _ => panic!("Expected Group result"),
    }
    mock.assert_async().await;
}

/// Helper to build a single-variable plain scenario index
fn make_scenarios(id: &str, var_name: &str, var_value: &str) -> IndexedEntities<Scenario> {
    <IndexedEntities<Scenario> as PersistedIndex<Scenario>>::new(
        Some(vec![Scenario::Plain(Box::new(ScenarioPlain {
            id: id.to_string(),
            name: "Scenario".to_string(),
            variables: Some(vec![Variable {
                name: var_name.to_string(),
                source_type: VariableSourceType::Text,
                value: var_value.to_string(),
                disabled: None,
            }]),
            validation_state: Default::default(),
            validation_warnings: None,
            validation_errors: None,
        }))]),
        None,
        None,
    )
}

#[tokio::test]
#[serial]
async fn test_setup_output_overrides_scenario() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/from-output")
        .with_status(200)
        .create_async()
        .await;

    let mut req = make_request_with_setup(
        "req-1",
        "Precedence",
        &format!("{}/api/{{{{x}}}}", server.url()),
        r#"
        if ($.x !== 'from-scenario') throw new Error(`expected scenario value, got ${$.x}`)
        output('x', 'from-output')
        "#,
        Some(
            r#"
            describe('precedence', () => {
                it('output overrides scenario', () => {
                    expect($.x).to.equal('from-output')
                    expect(scenario.x).to.equal('from-scenario')
                })
            })
            "#,
        ),
    );
    req.selected_scenario = Selection {
        id: "scn-1".to_string(),
        name: "Scenario".to_string(),
    };

    let mut ws = build_workspace(vec![RequestEntry::Request(req)]);
    ws.scenarios = make_scenarios("scn-1", "x", "from-scenario");
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(execution.error.is_none(), "{:?}", execution.error);
    assert!(execution.success);
    assert_eq!(execution.test_pass_count, 1);
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_output_overrides_scenario_for_subsequent_requests() {
    let mut server = mockito::Server::new_async().await;
    let mock_scenario = server
        .mock("GET", "/api/from-scenario")
        .with_status(200)
        .expect(1)
        .create_async()
        .await;
    let mock_output = server
        .mock("GET", "/api/from-output")
        .with_status(200)
        .expect(2)
        .create_async()
        .await;

    let url = format!("{}/api/{{{{x}}}}", server.url());
    // First request receives the scenario value, then overrides it via output
    let req1 = make_request_with_test(
        "req-1",
        "First",
        &url,
        r#"
        describe('first', () => {
            it('receives scenario value', () => {
                expect($.x).to.equal('from-scenario')
            })
        })
        output('x', 'from-output')
        "#,
    );
    // Second and third requests receive the overridden value
    let subsequent_test = r#"
        describe('subsequent', () => {
            it('receives overridden value', () => {
                expect($.x).to.equal('from-output')
            })
        })
        "#;
    let req2 = make_request_with_test("req-2", "Second", &url, subsequent_test);
    let req3 = make_request_with_test("req-3", "Third", &url, subsequent_test);

    let mut group = make_group(
        "grp-1",
        "Group",
        vec![
            RequestEntry::Request(req1),
            RequestEntry::Request(req2),
            RequestEntry::Request(req3),
        ],
        ExecutionConcurrency::Sequential,
    );
    group.selected_scenario = Selection {
        id: "scn-1".to_string(),
        name: "Scenario".to_string(),
    };

    let mut ws = build_workspace(vec![RequestEntry::Group(group)]);
    ws.scenarios = make_scenarios("scn-1", "x", "from-scenario");
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["grp-1".to_string()]).await;
    match results.into_iter().next().unwrap().unwrap() {
        ApicizeResult::Group(group_result) => {
            assert!(group_result.success);
            assert_eq!(group_result.test_pass_count, 3);
            assert_eq!(group_result.test_fail_count, 0);
        }
        _ => panic!("Expected Group result"),
    }
    mock_scenario.assert_async().await;
    mock_output.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_raw_body_and_base64() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/api/raw")
        .match_header("authorization", "Basic dXNlcjpwYXNz")
        .match_body(vec![0u8, 1, 254, 255])
        .with_status(200)
        .create_async()
        .await;

    let req = make_request_with_setup(
        "req-1",
        "Raw",
        &format!("{}/api/raw", server.url()),
        r#"
        request.method = 'POST'
        request.setHeader('Authorization', `Basic ${btoa('user:pass')}`)
        request.body = { type: BodyType.Raw, data: new Uint8Array([0, 1, 254, 255]) }
        "#,
        None,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert!(execution.error.is_none(), "{:?}", execution.error);
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_setup_null_request_fails() {
    let req = make_request_with_setup(
        "req-1",
        "Null Request",
        "http://localhost:1/",
        "request = null",
        None,
    );
    let ws = build_workspace(vec![RequestEntry::Request(req)]);
    let ctx = build_context(ws, None);

    let results = ctx.run(vec!["req-1".to_string()]).await;
    let execution = get_execution(results.into_iter().next().unwrap().unwrap());
    assert_eq!(
        execution.error,
        Some(ApicizeError::FailedSetup {
            description: "request must be an object".to_string()
        })
    );
}

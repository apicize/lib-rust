use apicize_lib::ExecutionState;

#[test]
fn test_execution_state_serialize() {
    let state = ExecutionState::SUCCESS | ExecutionState::ERROR;
    let json = serde_json::to_string(&state).unwrap();
    eprintln!("Serialized: {}", json);
    // Should serialize as plain number, not object
    assert_eq!(json, "10");
}

#[test]
fn test_execution_state_deserialize_from_number() {
    // TypeScript sends plain numbers
    let result: Result<ExecutionState, _> = serde_json::from_str("10");
    eprintln!("From number 10 (SUCCESS | ERROR): {:?}", result);
    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.contains(ExecutionState::SUCCESS));
    assert!(state.contains(ExecutionState::ERROR));
}

#[test]
fn test_execution_state_deserialize_from_object() {
    // Still support old object format for backwards compatibility
    let result: Result<ExecutionState, _> = serde_json::from_str(r#"{"bits":10}"#);
    eprintln!("From object {{bits:10}}: {:?}", result);
    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.contains(ExecutionState::SUCCESS));
    assert!(state.contains(ExecutionState::ERROR));
}

#[test]
fn test_execution_state_round_trip() {
    let original = ExecutionState::SUCCESS | ExecutionState::FAILURE;
    let json = serde_json::to_string(&original).unwrap();
    eprintln!("Round trip JSON: {}", json);
    let deserialized: ExecutionState = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_execution_state_from_typescript_values() {
    // Test individual flags from TypeScript enum values
    let running: ExecutionState = serde_json::from_str("1").unwrap();
    assert_eq!(running, ExecutionState::RUNNING);
    assert_eq!(serde_json::to_string(&running).unwrap(), "1");

    let success: ExecutionState = serde_json::from_str("2").unwrap();
    assert_eq!(success, ExecutionState::SUCCESS);
    assert_eq!(serde_json::to_string(&success).unwrap(), "2");

    let failure: ExecutionState = serde_json::from_str("4").unwrap();
    assert_eq!(failure, ExecutionState::FAILURE);
    assert_eq!(serde_json::to_string(&failure).unwrap(), "4");

    let error: ExecutionState = serde_json::from_str("8").unwrap();
    assert_eq!(error, ExecutionState::ERROR);
    assert_eq!(serde_json::to_string(&error).unwrap(), "8");
}

#[test]
fn test_execution_state_combined_flags() {
    // Test combined flags (bitwise OR)
    let combined = ExecutionState::SUCCESS | ExecutionState::FAILURE | ExecutionState::ERROR;
    let json = serde_json::to_string(&combined).unwrap();
    assert_eq!(json, "14"); // 2 | 4 | 8 = 14

    let deserialized: ExecutionState = serde_json::from_str(&json).unwrap();
    assert!(deserialized.contains(ExecutionState::SUCCESS));
    assert!(deserialized.contains(ExecutionState::FAILURE));
    assert!(deserialized.contains(ExecutionState::ERROR));
    assert!(!deserialized.contains(ExecutionState::RUNNING));
}

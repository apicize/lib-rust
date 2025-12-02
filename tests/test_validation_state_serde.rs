use apicize_lib::ValidationState;

#[test]
fn test_validation_state_serialize() {
    let state = ValidationState::WARNING | ValidationState::ERROR;
    let json = serde_json::to_string(&state).unwrap();
    eprintln!("Serialized: {}", json);
    // Should serialize as plain number, not object
    assert_eq!(json, "3");
}

#[test]
fn test_validation_state_deserialize_from_number() {
    // TypeScript sends plain numbers
    let result: Result<ValidationState, _> = serde_json::from_str("3");
    eprintln!("From number 3 (WARNING | ERROR): {:?}", result);
    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.contains(ValidationState::WARNING));
    assert!(state.contains(ValidationState::ERROR));
}

#[test]
fn test_validation_state_deserialize_from_object() {
    // Still support old object format for backwards compatibility
    let result: Result<ValidationState, _> = serde_json::from_str(r#"{"bits":3}"#);
    eprintln!("From object {{bits:3}}: {:?}", result);
    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.contains(ValidationState::WARNING));
    assert!(state.contains(ValidationState::ERROR));
}

#[test]
fn test_validation_state_round_trip() {
    let original = ValidationState::WARNING;
    let json = serde_json::to_string(&original).unwrap();
    eprintln!("Round trip JSON: {}", json);
    let deserialized: ValidationState = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_validation_state_individual_flags() {
    let warning: ValidationState = serde_json::from_str("1").unwrap();
    assert_eq!(warning, ValidationState::WARNING);
    assert_eq!(serde_json::to_string(&warning).unwrap(), "1");

    let error: ValidationState = serde_json::from_str("2").unwrap();
    assert_eq!(error, ValidationState::ERROR);
    assert_eq!(serde_json::to_string(&error).unwrap(), "2");
}

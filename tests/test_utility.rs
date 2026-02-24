use std::path::Path;

use apicize_lib::{
    build_absolute_file_name, convert_json, extract_csv, extract_json, generate_uuid,
    get_existing_absolute_file_name, get_relative_file_name, sequential, ApicizeError,
    ExecutionConcurrency,
};

// =============================================================================
// generate_uuid
// =============================================================================

#[test]
fn test_generate_uuid_returns_valid_uuid() {
    let id = generate_uuid();
    assert!(!id.is_empty());
    // UUID v4 format: 8-4-4-4-12 hex chars
    assert_eq!(id.len(), 36);
    assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
}

#[test]
fn test_generate_uuid_is_unique() {
    let id1 = generate_uuid();
    let id2 = generate_uuid();
    assert_ne!(id1, id2);
}

// =============================================================================
// sequential
// =============================================================================

#[test]
fn test_sequential_returns_sequential() {
    assert!(matches!(sequential(), ExecutionConcurrency::Sequential));
}

// =============================================================================
// convert_json
// =============================================================================

#[test]
fn test_convert_json_valid_object() {
    let result = convert_json("test", r#"{"key": "value"}"#);
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_object());
    assert_eq!(val["key"], "value");
}

#[test]
fn test_convert_json_valid_array() {
    let result = convert_json("test", r#"[1, 2, 3]"#);
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_array());
    assert_eq!(val.as_array().unwrap().len(), 3);
}

#[test]
fn test_convert_json_valid_string() {
    let result = convert_json("test", r#""hello""#);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str().unwrap(), "hello");
}

#[test]
fn test_convert_json_valid_number() {
    let result = convert_json("test", "42");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_i64().unwrap(), 42);
}

#[test]
fn test_convert_json_valid_null() {
    let result = convert_json("test", "null");
    assert!(result.is_ok());
    assert!(result.unwrap().is_null());
}

#[test]
fn test_convert_json_invalid_returns_error() {
    let result = convert_json("bad-json", "not valid json {{{");
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::Serialization { name, .. } => {
            assert_eq!(name, "bad-json");
        }
        other => panic!("Expected Serialization error, got: {}", other),
    }
}

#[test]
fn test_convert_json_empty_string_returns_error() {
    let result = convert_json("empty", "");
    assert!(result.is_err());
}

// =============================================================================
// get_existing_absolute_file_name
// =============================================================================

#[test]
fn test_get_existing_absolute_file_name_no_allowed_path() {
    let result = get_existing_absolute_file_name("test.json", &None);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::Error { description } => {
            assert!(description.contains("unsaved workbook"));
        }
        other => panic!("Expected Error, got: {}", other),
    }
}

#[test]
fn test_get_existing_absolute_file_name_file_not_found() {
    let temp_dir = std::env::temp_dir();
    let result =
        get_existing_absolute_file_name("nonexistent_file_12345.json", &Some(temp_dir));
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::FileAccess { description, file_name } => {
            assert_eq!(description, "Not found");
            assert_eq!(file_name, Some("nonexistent_file_12345.json".to_string()));
        }
        other => panic!("Expected FileAccess error, got: {}", other),
    }
}

#[test]
fn test_get_existing_absolute_file_name_file_exists() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("apicize_test_utility_exists.txt");
    std::fs::write(&test_file, "test").unwrap();

    let result = get_existing_absolute_file_name(
        "apicize_test_utility_exists.txt",
        &Some(temp_dir),
    );
    assert!(result.is_ok());
    assert!(result.unwrap().exists());

    std::fs::remove_file(&test_file).ok();
}

// =============================================================================
// build_absolute_file_name
// =============================================================================

#[test]
fn test_build_absolute_file_name_valid_directory() {
    let temp_dir = std::env::temp_dir();
    let result = build_absolute_file_name("output.json", &temp_dir);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), temp_dir.join("output.json"));
}

#[test]
fn test_build_absolute_file_name_nonexistent_directory() {
    let bad_dir = Path::new("/nonexistent_dir_12345");
    let result = build_absolute_file_name("output.json", bad_dir);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::FileAccess { description, file_name } => {
            assert!(description.contains("invalid directory"));
            assert!(file_name.is_some());
        }
        other => panic!("Expected FileAccess error, got: {}", other),
    }
}

// =============================================================================
// get_relative_file_name
// =============================================================================

#[test]
fn test_get_relative_file_name_valid_child() {
    let parent = Path::new("/home/user/projects");
    let child = Path::new("/home/user/projects/data/file.json");
    let result = get_relative_file_name(child, parent);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "data/file.json");
}

#[test]
fn test_get_relative_file_name_direct_child() {
    let parent = Path::new("/home/user/projects");
    let child = Path::new("/home/user/projects/file.json");
    let result = get_relative_file_name(child, parent);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "file.json");
}

#[test]
fn test_get_relative_file_name_not_a_child() {
    let parent = Path::new("/home/user/projects");
    let child = Path::new("/home/other/file.json");
    let result = get_relative_file_name(child, parent);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::Error { description } => {
            assert!(description.contains("is not a child of"));
        }
        other => panic!("Expected Error, got: {}", other),
    }
}

#[test]
fn test_get_relative_file_name_not_absolute() {
    let parent = Path::new("/home/user/projects");
    let child = Path::new("relative/path/file.json");
    let result = get_relative_file_name(child, parent);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::Error { description } => {
            assert!(description.contains("is not an absolute path"));
        }
        other => panic!("Expected Error, got: {}", other),
    }
}

// =============================================================================
// extract_json (file-based)
// =============================================================================

#[test]
fn test_extract_json_valid_file() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("apicize_test_extract.json");
    std::fs::write(&test_file, r#"{"name": "test", "count": 5}"#).unwrap();

    let result = extract_json(
        "test-data",
        "apicize_test_extract.json",
        &Some(temp_dir),
    );
    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["name"], "test");
    assert_eq!(val["count"], 5);

    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_extract_json_invalid_json_file() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("apicize_test_bad.json");
    std::fs::write(&test_file, "not json content").unwrap();

    let result = extract_json(
        "bad-data",
        "apicize_test_bad.json",
        &Some(temp_dir),
    );
    assert!(result.is_err());
    match result.unwrap_err() {
        ApicizeError::Serialization { name, .. } => {
            assert_eq!(name, "bad-data");
        }
        other => panic!("Expected Serialization error, got: {}", other),
    }

    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_extract_json_file_not_found() {
    let temp_dir = std::env::temp_dir();
    let result = extract_json(
        "missing",
        "apicize_nonexistent_99999.json",
        &Some(temp_dir),
    );
    assert!(result.is_err());
}

#[test]
fn test_extract_json_no_allowed_path() {
    let result = extract_json("test", "file.json", &None);
    assert!(result.is_err());
}

// =============================================================================
// extract_csv (file-based)
// =============================================================================

#[test]
fn test_extract_csv_valid_file() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("apicize_test_extract.csv");
    std::fs::write(&test_file, "name,age\nAlice,30\nBob,25\n").unwrap();

    let result = extract_csv(
        "csv-data",
        "apicize_test_extract.csv",
        &Some(temp_dir),
    );
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_array());
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[0]["age"].as_i64().unwrap(), 30);
    assert_eq!(arr[1]["name"], "Bob");
    assert_eq!(arr[1]["age"].as_i64().unwrap(), 25);

    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_extract_csv_empty_file() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("apicize_test_empty.csv");
    std::fs::write(&test_file, "").unwrap();

    let result = extract_csv(
        "empty-csv",
        "apicize_test_empty.csv",
        &Some(temp_dir),
    );
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_array());
    assert_eq!(val.as_array().unwrap().len(), 0);

    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_extract_csv_headers_only() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("apicize_test_headers_only.csv");
    std::fs::write(&test_file, "col1,col2,col3\n").unwrap();

    let result = extract_csv(
        "headers-only",
        "apicize_test_headers_only.csv",
        &Some(temp_dir),
    );
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_array());
    assert_eq!(val.as_array().unwrap().len(), 0);

    std::fs::remove_file(&test_file).ok();
}

#[test]
fn test_extract_csv_file_not_found() {
    let temp_dir = std::env::temp_dir();
    let result = extract_csv(
        "missing",
        "apicize_nonexistent_99999.csv",
        &Some(temp_dir),
    );
    assert!(result.is_err());
}

#[test]
fn test_extract_csv_no_allowed_path() {
    let result = extract_csv("test", "file.csv", &None);
    assert!(result.is_err());
}

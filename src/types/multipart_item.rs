use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

/// Type of multipart form item
#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum MultipartItemType {
    /// Text value
    #[default]
    Text,
    /// File contents
    File,
}

impl MultipartItemType {
    fn is_text(&self) -> bool {
        *self == MultipartItemType::Text
    }
}

/// Item in a multipart form body, either a text value or file contents
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MultipartItem {
    /// Name of form field
    pub name: String,
    /// Type of item (defaults to Text)
    #[serde(
        rename = "type",
        default,
        skip_serializing_if = "MultipartItemType::is_text"
    )]
    pub item_type: MultipartItemType,
    /// Value (Text items only)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// File name (File items only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// File contents, serialized as Base64 (File items only)
    #[serde_as(as = "Option<Base64<Standard, Unpadded>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<u8>>,
    /// If set to true, item should be ignored when dispatching Apicize Requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_file_item() -> MultipartItem {
        MultipartItem {
            name: "upload".to_string(),
            item_type: MultipartItemType::File,
            value: String::new(),
            file_name: Some("hello.txt".to_string()),
            data: Some(b"hello".to_vec()),
            disabled: None,
        }
    }

    #[test]
    fn test_text_item_deserializes_without_type() {
        let item: MultipartItem =
            serde_json::from_value(json!({"name": "a", "value": "x"})).unwrap();

        assert!(item.item_type == MultipartItemType::Text);
        assert_eq!(item.value, "x");
        assert!(item.file_name.is_none());
        assert!(item.data.is_none());
    }

    #[test]
    fn test_text_item_serializes_without_type() {
        let item = MultipartItem {
            name: "a".to_string(),
            item_type: MultipartItemType::Text,
            value: "x".to_string(),
            file_name: None,
            data: None,
            disabled: None,
        };

        let value = serde_json::to_value(&item).unwrap();

        assert_eq!(value, json!({"name": "a", "value": "x"}));
    }

    #[test]
    fn test_file_item_serializes_base64_contents() {
        let value = serde_json::to_value(make_file_item()).unwrap();

        assert_eq!(
            value,
            json!({
                "name": "upload",
                "type": "File",
                "fileName": "hello.txt",
                "data": "aGVsbG8"
            })
        );
    }

    #[test]
    fn test_file_item_round_trips() {
        let item = make_file_item();

        let text = serde_json::to_string(&item).unwrap();
        let result: MultipartItem = serde_json::from_str(&text).unwrap();

        assert!(result == item);
    }

    #[test]
    fn test_file_item_accepts_padded_base64() {
        let item: MultipartItem = serde_json::from_value(json!({
            "name": "upload",
            "type": "File",
            "fileName": "hello.txt",
            "data": "aGVsbG8="
        }))
        .unwrap();

        assert_eq!(item.data, Some(b"hello".to_vec()));
    }
}

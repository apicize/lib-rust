use serde::{Deserialize, Serialize};

/// String name/value pairs used to store values like Apicize headers, query string parameters, etc.
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct NameValuePair {
    /// Name of value
    pub name: String,
    /// Value
    pub value: String,
    /// If set to true, name/value pair should be ignored when dispatching Apicize Requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}
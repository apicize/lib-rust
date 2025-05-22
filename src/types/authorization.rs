use super::Selection;
use crate::{utility::*, Identifiable};
use serde::{Deserialize, Serialize};

/// Authorization information used when dispatching an Apicize Request
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum Authorization {
    /// Basic authentication (basic authorization header)
    #[serde(rename_all = "camelCase")]
    Basic {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of authorization configuration
        name: String,
        /// User name
        username: String,
        /// Password
        password: String,
        /// Warning if invalid
        warning: Option<String>,
    },
    /// OAuth2 client flow (bearer authorization header)
    #[serde(rename_all = "camelCase")]
    OAuth2Client {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        /// Client secret (allowed to be blank)
        client_secret: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Audience to add to token
        audience: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: Option<String>,
        /// Selected certificate, if applicable
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_certificate: Option<Selection>,
        /// Selected proxy, if applicable
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_proxy: Option<Selection>,
        #[serde(skip_serializing_if = "Option::is_none")]
        send_credentials_in_body: Option<bool>,
        /// Warning if invalid
        warning: Option<String>,
    },
    /// OAuth2 PKCE flow (note, this can only be used interactively)
    #[serde(rename_all = "camelCase")]
    OAuth2Pkce {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// URL for authorization
        authorize_url: String,
        /// URL to retrieve access token from
        access_token_url: String,
        /// Client ID
        client_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: Option<String>,
        /// Currently active token (needs to be set before usage)
        #[serde(skip_serializing)]
        token: Option<String>,
        /// Currently active refresh token if available (needs to be set before usage)
        #[serde(skip_serializing)]
        refresh_token: Option<String>,
        /// Expiration of currently active token in seconds past Unix epoch (needs to be set before usage)
        #[serde(skip_serializing)]
        expiration: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        send_credentials_in_body: Option<bool>,
        /// Warning if invalid
        warning: Option<String>,
    },
    /// API key authentication (sent in HTTP header)
    #[serde(rename_all = "camelCase")]
    ApiKey {
        /// Uniquely identifies authorization configuration
        #[serde(default = "generate_uuid")]
        id: String,
        /// Indicates if/how authorization will be persisted
        /// Human-readable name of authorization configuration
        name: String,
        /// Name of header (ex. "x-api-key")
        header: String,
        /// Value of key to include as header value
        value: String,
        /// Warning if invalid
        warning: Option<String>,
    },
}

// impl Authorization {
//     fn get_id_and_name(&self) -> (&String, &String) {
//         match self {
//             Authorization::Basic { id, name, .. } => (id, name),
//             Authorization::OAuth2Client { id, name, .. } => (id, name),
//             Authorization::OAuth2Pkce { id, name, .. } => (id, name),
//             Authorization::ApiKey { id, name, .. } => (id, name),
//         }
//     }
// }

impl Default for Authorization {
    fn default() -> Self {
        Authorization::ApiKey {
            id: generate_uuid(),
            name: String::default(),
            header: String::default(),
            value: String::default(),
            warning: None,
        }
    }
}

impl Identifiable for Authorization {
    fn get_id(&self) -> &str {
        match self {
            Authorization::Basic { id, .. } => id,
            Authorization::OAuth2Client { id,  .. } => id,
            Authorization::OAuth2Pkce { id,  .. } => id,
            Authorization::ApiKey { id, .. } => id,
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Authorization::Basic { name, .. } => name,
            Authorization::OAuth2Client { name,  .. } => name,
            Authorization::OAuth2Pkce { name,  .. } => name,
            Authorization::ApiKey { name, .. } => name,
        }
    }

    fn get_title(&self) -> String {
        let name = self.get_name();
        if name.is_empty() {
            "(Unamed)".to_string()
        } else {
            name.to_string()
        }
    }
    
    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        let new_id = generate_uuid();
        
        match cloned {
            Authorization::Basic { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
            Authorization::OAuth2Client { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
            Authorization::OAuth2Pkce { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
            Authorization::ApiKey { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
        }

        cloned
    }
}

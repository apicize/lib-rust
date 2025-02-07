use serde::{Deserialize, Serialize};
use crate::{utility::*, Identifable};
use super::Selection;

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
        /// Scope to add to token (multiple scopes should be space-delimited)
        scope: Option<String>,
        /// Selected certificate, if applicable
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_certificate: Option<Selection>,
        /// Selected proxy, if applicable
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_proxy: Option<Selection>,
        // #[serde(skip_serializing_if="Option::is_none")]
        // send_credentials_in_body: Option<bool>,
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
        // #[serde(skip_serializing_if="Option::is_none")]
        // send_credentials_in_body: Option<bool>,
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
    },
}

impl Authorization {
    fn get_id_and_name(&self) -> (&String, &String) {
        match self {
            Authorization::Basic { id, name, .. } => (id, name),
            Authorization::OAuth2Client { id, name, .. } => (id, name),
            Authorization::OAuth2Pkce { id, name, .. } => (id, name),
            Authorization::ApiKey { id, name, .. } => (id, name),
        }
    }
}

impl Identifable for Authorization {
    fn get_id(&self) -> &String {
        let (id, _) = self.get_id_and_name();
        id
    }

    fn get_name(&self) -> &String {
        let (_, name) = self.get_id_and_name();
        name
    }

    fn get_title(&self) -> String {
        let (id, name) = self.get_id_and_name();
        if name.is_empty() {
            format!("{} (Unnamed)", id)
        } else {
            name.to_string()
        }
    }
}
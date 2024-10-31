//! oauth2_client_tokens
//!
//! This module implements OAuth2 client flow support, including support for caching tokens

use std::collections::HashMap;
use std::ops::Add;
use std::time::Instant;

use oauth2::basic::BasicClient;
use oauth2::reqwest;
use oauth2::{ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};
use tokio::sync::Mutex;

use crate::{ExecutionError, WorkbookCertificate, WorkbookProxy};

lazy_static! {
    /// Static cache of retrieved OAuth2 client tokens
    static ref OAUTH2_TOKEN_CACHE: Mutex<HashMap<String, (Instant, String)>> =
        Mutex::new(HashMap::new());
}

/// Return cached oauth2 token, with indicator of whether value was cached
pub async fn get_oauth2_client_credentials(
    id: &str,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scope: &Option<String>,
    certificate: Option<&WorkbookCertificate>,
    proxy: Option<&WorkbookProxy>,
) -> Result<(String, bool), ExecutionError> {
    let cloned_scope = scope.clone();

    // Check cache and return if token found and not expired
    let mut tokens = OAUTH2_TOKEN_CACHE.lock().await;
    let valid_token = match tokens.get(id) {
        Some((expiration, cached_token)) => {
            let now = Instant::now();
            if expiration.gt(&now) {
                Some(cached_token.clone())
            } else {
                None
            }
        }
        None => None,
    };

    if let Some(token) = valid_token {
        return Ok((token, true));
    }

    // We have to create a blocked span when using oauth because some error types
    // in oauth2 library dependencies do not implement Copy
    // match spawn_blocking(move || {
    // Retrieve an access token
    let mut client = BasicClient::new(ClientId::new(String::from(client_id)))
        .set_token_uri(TokenUrl::new(String::from(token_url)).expect("Unable to parse OAuth token URL"));

    if !client_secret.trim().is_empty() {
        client = client.set_client_secret(ClientSecret::new(String::from(client_secret)));
    }

    let mut token_request = client.exchange_client_credentials();
    if let Some(scope_value) = cloned_scope {
        token_request = token_request.add_scope(Scope::new(scope_value.clone()));
    }

    let mut reqwest_builder =
        reqwest::ClientBuilder::new().redirect(reqwest::redirect::Policy::none());

    // let cert1 = fs::read("/home/jason/projects/warehouse/adp-extract/certs/test.pem").unwrap();
    // let key1 = fs::read("/home/jason/projects/warehouse/adp-extract/certs/test.key").unwrap();
    // let identity_result = reqwest::Identity::from_pkcs8_pem(&cert1, &key1);
    // reqwest_builder = reqwest_builder
    //     .identity(identity_result.unwrap())
    //     .connection_verbose(true)
    //     .use_native_tls()
    //     .tls_info(true);

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
            Err(err) => return Err(ExecutionError::Reqwest(err)),
        }
    }

    let http_client = reqwest_builder
        .build()
        .expect("Unable to build OAuth HTTP client");

    match token_request.request_async(&http_client).await {
        Ok(token_response) => {
            let expiration = match token_response.expires_in() {
                Some(token_expires_in) => Instant::now().add(token_expires_in),
                None => Instant::now(),
            };
            let token = token_response.access_token().secret().clone();
            tokens.insert(String::from(id), (expiration, token.clone()));
            Ok((token, false))
        }
        Err(err) => Err(ExecutionError::OAuth2(err)),
    }
}

/// Clear all cached OAuth2 tokens
pub async fn clear_all_oauth2_tokens() -> usize {
    let mut cache = OAUTH2_TOKEN_CACHE.lock().await;
    let count = cache.len();
    cache.clear();
    count
}

/// Clear specified cached OAuth2 credentials, returning true if value was cached
pub async fn clear_oauth2_token(id: String) -> bool {
    OAUTH2_TOKEN_CACHE.lock().await.remove(&id).is_some()
}

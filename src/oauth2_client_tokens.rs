//! oauth2_client_tokens
//!
//! This module implements OAuth2 client flow support, including support for caching tokens
use crate::{ExecutionError, WorkbookCertificate, WorkbookProxy};
use oauth2::basic::BasicClient;
use oauth2::reqwest;
use oauth2::{ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};
use std::collections::HashMap;
use std::ops::Add;
use std::sync::LazyLock;
use std::time::Instant;
use tokio::sync::Mutex;

pub static TOKEN_CACHE: LazyLock<Mutex<HashMap<String, (Instant, String)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Return cached oauth2 token, with indicator of whether value was retrieved from cache
pub async fn get_oauth2_client_credentials<'a>(
    id: &str,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scope: &'a Option<String>,
    certificate: Option<&'a WorkbookCertificate>,
    proxy: Option<&'a WorkbookProxy>,
) -> Result<(String, bool), ExecutionError> {
    let cloned_scope = scope.clone();

    // Check cache and return if token found and not expired
    let mut locked_cache = TOKEN_CACHE.lock().await;
    let valid_token = match locked_cache.get(id) {
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

    // Retrieve an access token
    let mut client = BasicClient::new(ClientId::new(String::from(client_id))).set_token_uri(
        TokenUrl::new(String::from(token_url)).expect("Unable to parse OAuth token URL"),
    );

    if !client_secret.trim().is_empty() {
        client = client.set_client_secret(ClientSecret::new(String::from(client_secret)));
    }

    let mut token_request = client.exchange_client_credentials();
    if let Some(scope_value) = cloned_scope {
        token_request = token_request.add_scope(Scope::new(scope_value.clone()));
    }

    let mut reqwest_builder =
        reqwest::ClientBuilder::new().redirect(reqwest::redirect::Policy::none());

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
            locked_cache.insert(String::from(id), (expiration, token.clone()));
            Ok((token, false))
        }
        Err(err) => Err(ExecutionError::OAuth2(err)),
    }
}

/// Clear all cached OAuth2 tokens
pub async fn clear_all_oauth2_tokens<'a>() -> usize {
    let locked_cache = &mut TOKEN_CACHE.lock().await;
    let count = locked_cache.len();
    locked_cache.clear();
    count
}

/// Clear specified cached OAuth2 credentials, returning true if value was cached
pub async fn clear_oauth2_token(id: &str) -> bool {
    let mut locked_cache = TOKEN_CACHE.lock().await;
    locked_cache.remove(&String::from(id)).is_some()
}

#[cfg(test)]
pub mod tests {

    use std::time::{Duration, Instant};

    use mockall::automock;
    use serial_test::{parallel, serial};

    use crate::oauth2_client_tokens::{
        clear_all_oauth2_tokens, clear_oauth2_token, get_oauth2_client_credentials, TOKEN_CACHE,
    };

    pub struct OAuth2ClientTokens;
    #[automock]
    impl OAuth2ClientTokens {
        pub async fn get_oauth2_client_credentials<'a>(
            _id: &str,
            _token_url: &str,
            _client_id: &str,
            _client_secret: &str,
            _scope: &'a Option<String>,
            _certificate: Option<&'a crate::WorkbookCertificate>,
            _proxy: Option<&'a crate::WorkbookProxy>,
        ) -> Result<(String, bool), crate::ExecutionError> { Ok((String::from(""), false))}
        pub async fn clear_all_oauth2_tokens<'a>() -> usize { 1 }
        pub async fn clear_oauth2_token(_id: &str) -> bool { true }    
    }

    // Note - because we are using shared storage for cached tokens, some tests cannot be run in parallel, thus the "serial" attributes.
    // We also do explicitly run some tests in parallel to ensure that the module itself is threadsafe.

    const FAKE_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[tokio::test()]
    #[serial]
    async fn get_oauth2_client_credentials_returns_cached_token() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
            let expiration = Instant::now()
                .checked_add(Duration::from_millis(10000))
                .unwrap();
            locked_cache.insert(String::from("abc"), (expiration, String::from("123")));
        }
        assert_eq!(
            (get_oauth2_client_credentials(
                "abc",
                "http://server",
                "me",
                "shhh",
                &None,
                None,
                None
            )
            .await)
                .unwrap(),
            (String::from("123"), true)
        );
    }

    #[tokio::test]
    #[serial]
    async fn get_oauth2_client_credentials_calls_server() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
        }
        let mut server = mockito::Server::new_async().await;
        let oauth2_response = format!(
            "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
            FAKE_TOKEN
        );
        let mock = server
            .mock("POST", "/")
            // .match_body("foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(oauth2_response)
            .create();

        let result = get_oauth2_client_credentials(
            "abc",
            server.url().as_str(),
            "me",
            "shhh",
            &None,
            None,
            None,
        )
        .await;

        mock.assert();

        assert_eq!(result.unwrap(), (String::from(FAKE_TOKEN), false));

        {
            let locked_cache = TOKEN_CACHE.lock().await;
            assert!(locked_cache.get(&String::from("abc")).is_some());
        }
    }

    #[tokio::test]
    #[serial]
    async fn get_oauth2_client_credentials_ignores_expired_cache() {
        let mut server = mockito::Server::new_async().await;
        let oauth2_response = format!(
            "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
            FAKE_TOKEN
        );
        let mock = server
            .mock("POST", "/")
            // .match_body("foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(oauth2_response)
            .create();

        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
            let expiration = Instant::now()
                .checked_sub(Duration::from_millis(100000))
                .unwrap();
            locked_cache.insert(String::from("abc"), (expiration, String::from("123")));
            assert_eq!(
                locked_cache.get(&String::from("abc")),
                Some(&(expiration, String::from("123")))
            );
        }

        let result = get_oauth2_client_credentials(
            "abc",
            server.url().as_str(),
            "me",
            "shhh",
            &None,
            None,
            None,
        )
        .await;

        mock.assert();

        assert_eq!(result.unwrap(), (String::from(FAKE_TOKEN), false));
        {
            let locked_cache = TOKEN_CACHE.lock().await;
            assert!(locked_cache.get(&String::from("abc")).is_some());
        }
    }

    #[tokio::test]
    #[serial]
    async fn clear_all_oauth2_tokens_clears_tokens() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
            let expiration = Instant::now()
                .checked_add(Duration::from_millis(1000))
                .unwrap();
            locked_cache.insert(String::from("abc"), (expiration, String::from("123")));
            assert_eq!(
                locked_cache.get(&String::from("abc")),
                Some(&(expiration, String::from("123")))
            );
        }
        assert_eq!(clear_all_oauth2_tokens().await, 1);
        {
            let locked_cache = TOKEN_CACHE.lock().await;
            assert_eq!(locked_cache.len(), 0);
        }
    }

    #[tokio::test]
    #[serial]
    async fn clear_oauth2_token_removes_item() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
            let expiration = Instant::now()
                .checked_add(Duration::from_millis(1000))
                .unwrap();
            locked_cache.insert(String::from("abc"), (expiration, String::from("123")));
            assert_eq!(
                locked_cache.get(&String::from("abc")),
                Some(&(expiration, String::from("123")))
            );
        }
        assert_eq!(clear_oauth2_token("abc").await, true);
        {
            let locked_cache = TOKEN_CACHE.lock().await;
            assert_eq!(locked_cache.get(&String::from("abc")), None);
        }
    }

    #[tokio::test]
    #[serial]
    async fn clear_oauth2_token_ignores_invalid_id() {
        assert_eq!(clear_oauth2_token("abc_bogus").await, false);
    }

    #[tokio::test()]
    #[parallel]
    async fn get_oauth2_client_credentials_parallel_1() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
        }
        let mut server = mockito::Server::new_async().await;
        let oauth2_response = format!(
            "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
            FAKE_TOKEN
        );
        let mock = server
            .mock("POST", "/")
            // .match_body("foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(oauth2_response)
            .create();
        assert_eq!(
            (get_oauth2_client_credentials("abc1", &server.url(), "me", "shhh", &None, None, None)
                .await)
                .unwrap(),
            (String::from(FAKE_TOKEN), false)
        );
        mock.assert();

        // Second attempt will use cache
        assert_eq!(
            (get_oauth2_client_credentials("abc1", &server.url(), "me", "shhh", &None, None, None)
                .await)
                .unwrap(),
            (String::from(FAKE_TOKEN), true)
        );
        mock.expect_at_most(0);
    }

    #[tokio::test()]
    #[parallel]
    async fn get_oauth2_client_credentials_parallel_2() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
        }
        let mut server = mockito::Server::new_async().await;
        let oauth2_response = format!(
            "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
            FAKE_TOKEN
        );
        let mock = server
            .mock("POST", "/")
            // .match_body("foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(oauth2_response)
            .create();
        assert_eq!(
            (get_oauth2_client_credentials("abc2", &server.url(), "me", "shhh", &None, None, None)
                .await)
                .unwrap(),
            (String::from(FAKE_TOKEN), false)
        );
        mock.assert();

        // Second attempt will use cache
        assert_eq!(
            (get_oauth2_client_credentials("abc2", &server.url(), "me", "shhh", &None, None, None)
                .await)
                .unwrap(),
            (String::from(FAKE_TOKEN), true)
        );
        mock.expect_at_most(0);
    }

    #[tokio::test()]
    #[parallel]
    async fn get_oauth2_client_credentials_parallel_3() {
        {
            let mut locked_cache = TOKEN_CACHE.lock().await;
            locked_cache.clear();
        }
        let mut server = mockito::Server::new_async().await;
        let oauth2_response = format!(
            "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
            FAKE_TOKEN
        );
        let mock = server
            .mock("POST", "/")
            // .match_body("foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(oauth2_response)
            .create();
        assert_eq!(
            (get_oauth2_client_credentials("abc3", &server.url(), "me", "shhh", &None, None, None)
                .await)
                .unwrap(),
            (String::from(FAKE_TOKEN), false)
        );
        mock.assert();

        // Second attempt will use cache
        assert_eq!(
            (get_oauth2_client_credentials("abc3", &server.url(), "me", "shhh", &None, None, None)
                .await)
                .unwrap(),
            (String::from(FAKE_TOKEN), true)
        );
        mock.expect_at_most(0);
    }
}

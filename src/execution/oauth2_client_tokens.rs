//! This module implements OAuth2 client flow support
use crate::{
    retrieve_oauth2_token_from_cache, store_oauth2_token_in_cache, ApicizeError, CachedTokenInfo,
    Certificate, Identifiable, Proxy,
};
use oauth2::basic::BasicClient;
use oauth2::{reqwest, AuthType};
use oauth2::{ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::time::{SystemTime, UNIX_EPOCH};

/// OAuth2 issued client token result
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenResult {
    /// Issued token
    pub token: String,
    /// Set to True if token was retrieved via cache
    pub cached: bool,
    /// URL used to retrieve token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Name of the certificate parameter, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    /// Name of the proxy parameter, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

/// Return cached oauth2 token, with indicator of whether value was retrieved from cache
#[allow(clippy::too_many_arguments)]
pub async fn get_oauth2_client_credentials<'a>(
    id: &str,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    send_credentials_in_body: bool,
    scopes: &'a Option<String>,
    audience: &'a Option<String>,
    certificate: Option<&'a Certificate>,
    proxy: Option<&'a Proxy>,
    enable_trace: bool,
) -> Result<TokenResult, ApicizeError> {
    // Check cache and return if token found and not expired
    let valid_token = match retrieve_oauth2_token_from_cache(id).await {
        Some(cached_token) => match cached_token.expiration {
            Some(expiration) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if expiration.gt(&now) {
                    Some(cached_token.clone())
                } else {
                    None
                }
            }
            None => None,
        },
        None => None,
    };

    if let Some(cached_token) = valid_token {
        return Ok(TokenResult {
            token: cached_token.access_token,
            cached: true,
            url: None,
            certificate: None,
            proxy: None,
        });
    }

    // Retrieve an access token
    let mut client = BasicClient::new(ClientId::new(String::from(client_id)))
        .set_token_uri(
            TokenUrl::new(String::from(token_url)).expect("Unable to parse OAuth token URL"),
        )
        .set_auth_type(if send_credentials_in_body {
            AuthType::RequestBody
        } else {
            AuthType::BasicAuth
        });

    if !client_secret.trim().is_empty() {
        client = client.set_client_secret(ClientSecret::new(String::from(client_secret)));
    }

    let mut token_request = client.exchange_client_credentials();

    if let Some(scope_value) = &scopes {
        if !scope_value.is_empty() {
            token_request = token_request.add_scope(Scope::new(scope_value.clone()));
        }
    }

    if let Some(audience_value) = &audience {
        if !audience_value.is_empty() {
            token_request = token_request.add_extra_param("audience", audience_value);
        }
    }

    let mut reqwest_builder = reqwest::ClientBuilder::new()
        .connection_verbose(enable_trace)
        .redirect(reqwest::redirect::Policy::none());

    // Add certificate to builder if configured
    if let Some(active_cert) = certificate {
        match active_cert.append_to_builder(reqwest_builder) {
            Ok(updated_builder) => reqwest_builder = updated_builder,
            Err(err) => {
                return Err(ApicizeError::OAuth2Client {
                    description: String::from("Error assigning OAuth certificate"),
                    source: Some(Box::new(err)),
                })
            }
        }
    }

    // Add proxy to builder if configured
    if let Some(active_proxy) = proxy {
        match active_proxy.append_to_builder(reqwest_builder) {
            Ok(updated_builder) => reqwest_builder = updated_builder,
            Err(err) => {
                return Err(ApicizeError::OAuth2Client {
                    description: String::from("Error assigning OAuth proxy"),
                    source: Some(Box::new(ApicizeError::from_reqwest(err))),
                })
            }
        }
    }

    let http_client = match reqwest_builder.build() {
        Ok(client) => client,
        Err(err) => {
            return Err(ApicizeError::OAuth2Client {
                description: String::from("Error building OAuth request"),
                source: Some(Box::new(ApicizeError::from_reqwest(err))),
            })
        }
    };

    match token_request.request_async(&http_client).await {
        Ok(token_response) => {
            let expiration = token_response.expires_in().map(|token_expires_in| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .add(token_expires_in.as_secs())
            });
            let token = token_response.access_token().secret().clone();
            store_oauth2_token_in_cache(
                id,
                CachedTokenInfo {
                    access_token: token.clone(),
                    refresh_token: None,
                    expiration,
                },
            ).await;
            Ok(TokenResult {
                token,
                cached: false,
                url: Some(String::from(token_url)),
                certificate: certificate.map(|c| c.get_name().to_owned()),
                proxy: proxy.map(|p| p.get_name().to_owned()),
            })
        }
        Err(err) => Err(ApicizeError::OAuth2Client {
            description: String::from("Error dispatching OAuth2 token request"),
            source: Some(Box::new(ApicizeError::from_oauth2(err))),
        }),
    }
}

// #[cfg(test)]
// pub mod tests {
//     use std::ops::{Add, Sub};
//     use std::time::{SystemTime, UNIX_EPOCH};

//     use mockall::automock;
//     use serial_test::{parallel, serial};

//     use crate::oauth2_client_tokens::{
//         clear_all_oauth2_tokens, clear_oauth2_token, get_oauth2_client_credentials,
//         CachedTokenInfo, TokenResult, OAUTH2_CLIENT_TOKEN_CACHE,
//     };

//     pub struct OAuth2ClientTokens;
//     #[automock]
//     impl OAuth2ClientTokens {
//         pub async fn get_oauth2_client_credentials<'a>(
//             _id: &str,
//             _token_url: &str,
//             _client_id: &str,
//             _client_secret: &str,
//             _send_credentials_in_body: bool,
//             _scope: &'a Option<String>,
//             _audience: &'a Option<String>,
//             _certificate: Option<&'a crate::Certificate>,
//             _proxy: Option<&'a crate::Proxy>,
//             _enable_trace: bool,
//         ) -> Result<TokenResult, crate::ApicizeError> {
//             Ok(TokenResult {
//                 token: String::from(""),
//                 cached: false,
//                 url: None,
//                 certificate: None,
//                 proxy: None,
//             })
//         }
//         pub async fn clear_all_oauth2_tokens<'a>() -> usize {
//             1
//         }
//         pub async fn clear_oauth2_token(_id: &str) -> bool {
//             true
//         }
//     }

//     // Note - because we are using shared storage for cached tokens, some tests cannot be run in parallel, thus the "serial" attributes.
//     // We also do explicitly run some tests in parallel to ensure that the module itself is threadsafe.

//     const FAKE_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

//     #[tokio::test()]
//     #[serial]
//     async fn get_oauth2_client_credentials_returns_cached_token() {
//         {
//             let mut locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             locked_cache.clear();
//             let expiration = Some(
//                 SystemTime::now()
//                     .duration_since(UNIX_EPOCH)
//                     .unwrap()
//                     .as_secs()
//                     .add(10),
//             );
//             locked_cache.insert(
//                 String::from("abc"),
//                 CachedTokenInfo {
//                     expiration,
//                     access_token: String::from("123"),
//                     refresh_token: None,
//                 },
//             );
//         }
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc",
//                 "http://server",
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from("123"),
//                 cached: true,
//                 url: None,
//                 certificate: None,
//                 proxy: None
//             }
//         );
//     }

//     #[tokio::test]
//     #[serial]
//     async fn get_oauth2_client_credentials_calls_server() {
//         {
//             let mut locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             locked_cache.clear();
//         }
//         let mut server = mockito::Server::new_async().await;
//         let oauth2_response = format!(
//             "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
//             FAKE_TOKEN
//         );
//         let mock = server
//             .mock("POST", "/")
//             // .match_body("foo")
//             .with_status(200)
//             .with_header("Content-Type", "application/json")
//             .with_body(oauth2_response)
//             .create();

//         let result = get_oauth2_client_credentials(
//             "abc",
//             server.url().as_str(),
//             "me",
//             "shhh",
//             false,
//             &None,
//             &None,
//             None,
//             None,
//             false,
//         )
//         .await;

//         mock.assert();

//         assert_eq!(
//             result.unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: false,
//                 url: Some(server.url()),
//                 certificate: None,
//                 proxy: None
//             }
//         );

//         {
//             let locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             assert!(locked_cache.get(&String::from("abc")).is_some());
//         }
//     }

//     #[tokio::test]
//     #[serial]
//     async fn get_oauth2_client_credentials_ignores_expired_cache() {
//         let mut server = mockito::Server::new_async().await;
//         let oauth2_response = format!(
//             "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
//             FAKE_TOKEN
//         );
//         let mock = server
//             .mock("POST", "/")
//             // .match_body("foo")
//             .with_status(200)
//             .with_header("Content-Type", "application/json")
//             .with_body(oauth2_response)
//             .create();

//         {
//             let mut locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             locked_cache.clear();
//             let expiration = Some(
//                 SystemTime::now()
//                     .duration_since(UNIX_EPOCH)
//                     .unwrap()
//                     .as_secs()
//                     .sub(10),
//             );
//             let cached_token = CachedTokenInfo {
//                 expiration,
//                 access_token: String::from("123"),
//                 refresh_token: None,
//             };
//             locked_cache.insert(String::from("abc"), cached_token.clone());
//             assert_eq!(locked_cache.get(&String::from("abc")), Some(&cached_token));
//         }

//         let result = get_oauth2_client_credentials(
//             "abc",
//             server.url().as_str(),
//             "me",
//             "shhh",
//             false,
//             &None,
//             &None,
//             None,
//             None,
//             false,
//         )
//         .await;

//         mock.assert();

//         assert_eq!(
//             result.unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: false,
//                 url: Some(server.url()),
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         {
//             let locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             assert!(locked_cache.get(&String::from("abc")).is_some());
//         }
//     }

//     #[tokio::test]
//     #[serial]
//     async fn clear_all_oauth2_tokens_clears_tokens() {
//         {
//             let mut locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             locked_cache.clear();
//             let expiration = Some(
//                 SystemTime::now()
//                     .duration_since(UNIX_EPOCH)
//                     .unwrap()
//                     .as_secs()
//                     .add(10),
//             );
//             let cached_token = CachedTokenInfo {
//                 expiration,
//                 access_token: String::from("123"),
//                 refresh_token: None,
//             };
//             locked_cache.insert(String::from("abc"), cached_token.clone());
//             assert_eq!(locked_cache.get(&String::from("abc")), Some(&cached_token));
//         }
//         assert_eq!(clear_all_oauth2_tokens().await, 1);
//         {
//             let locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             assert_eq!(locked_cache.len(), 0);
//         }
//     }

//     #[tokio::test]
//     #[serial]
//     async fn clear_oauth2_token_removes_item() {
//         {
//             let mut locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             locked_cache.clear();
//             let expiration = Some(
//                 SystemTime::now()
//                     .duration_since(UNIX_EPOCH)
//                     .unwrap()
//                     .as_secs()
//                     .add(10),
//             );
//             let cached_token = CachedTokenInfo {
//                 expiration,
//                 access_token: String::from("123"),
//                 refresh_token: None,
//             };
//             locked_cache.insert(String::from("abc"), cached_token.clone());
//             assert_eq!(locked_cache.get(&String::from("abc")), Some(&cached_token));
//         }
//         assert_eq!(clear_oauth2_token("abc").await, true);
//         {
//             let locked_cache = OAUTH2_CLIENT_TOKEN_CACHE.lock().await;
//             assert_eq!(locked_cache.get(&String::from("abc")), None);
//         }
//     }

//     #[tokio::test]
//     #[serial]
//     async fn clear_oauth2_token_ignores_invalid_id() {
//         assert_eq!(clear_oauth2_token("abc_bogus").await, false);
//     }

//     #[tokio::test()]
//     #[parallel]
//     async fn get_oauth2_client_credentials_parallel_1() {
//         let mut server = mockito::Server::new_async().await;
//         let oauth2_response = format!(
//             "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
//             FAKE_TOKEN
//         );
//         let mock = server
//             .mock("POST", "/")
//             // .match_body("foo")
//             .with_status(200)
//             .with_header("Content-Type", "application/json")
//             .with_body(oauth2_response)
//             .create();
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc1",
//                 &server.url(),
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: false,
//                 url: Some(server.url()),
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         mock.assert();

//         // Second attempt will use cache
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc1",
//                 &server.url(),
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: true,
//                 url: None,
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         mock.expect_at_most(0);
//     }

//     #[tokio::test()]
//     #[parallel]
//     async fn get_oauth2_client_credentials_parallel_2() {
//         let mut server = mockito::Server::new_async().await;
//         let oauth2_response = format!(
//             "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
//             FAKE_TOKEN
//         );
//         let mock = server
//             .mock("POST", "/")
//             // .match_body("foo")
//             .with_status(200)
//             .with_header("Content-Type", "application/json")
//             .with_body(oauth2_response)
//             .create();
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc2",
//                 &server.url(),
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: false,
//                 url: Some(server.url()),
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         mock.assert();

//         // Second attempt will use cache
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc2",
//                 &server.url(),
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: true,
//                 url: None,
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         mock.expect_at_most(0);
//     }

//     #[tokio::test()]
//     #[parallel]
//     async fn get_oauth2_client_credentials_parallel_3() {
//         let mut server = mockito::Server::new_async().await;
//         let oauth2_response = format!(
//             "{{\"access_token\":\"{}\",\"expires_in\":86400,\"token_type\":\"Bearer\"}}",
//             FAKE_TOKEN
//         );
//         let mock = server
//             .mock("POST", "/")
//             // .match_body("foo")
//             .with_status(200)
//             .with_header("Content-Type", "application/json")
//             .with_body(oauth2_response)
//             .create();
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc3",
//                 &server.url(),
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: false,
//                 url: Some(server.url()),
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         mock.assert();

//         // Second attempt will use cache
//         assert_eq!(
//             (get_oauth2_client_credentials(
//                 "abc3",
//                 &server.url(),
//                 "me",
//                 "shhh",
//                 false,
//                 &None,
//                 &None,
//                 None,
//                 None,
//                 false,
//             )
//             .await)
//                 .unwrap(),
//             TokenResult {
//                 token: String::from(FAKE_TOKEN),
//                 cached: true,
//                 url: None,
//                 certificate: None,
//                 proxy: None
//             }
//         );
//         mock.expect_at_most(0);
//     }
// }

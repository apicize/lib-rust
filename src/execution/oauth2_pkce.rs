//! This module implements helpers for OAuth2 PKCE flow.  It does not include mechanisms
//! to enable interactive retrieval of tokens (i.e. HTML browser)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl, basic::BasicClient, reqwest,
    url::ParseError,
};
use reqwest::Url;

/// OAuth2 issued PKCE token result
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PkceTokenResult {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Expiration of token in seconds past Unix epoch
    pub expiration: Option<u64>,
}

/// Generate authorization URL and include the CSRF token and PKCE verifier in the response
pub fn generate_authorization(
    authorize_uri: &str,
    redirect_uri: &str,
    client_id: &str,
    send_credentials_in_body: bool,
    scopes: Option<String>,
    audience: Option<String>,
) -> Result<(Url, CsrfToken, String), ParseError> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_auth_uri(AuthUrl::new(authorize_uri.to_string())?)
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?)
        .set_auth_type(if send_credentials_in_body {
            AuthType::RequestBody
        } else {
            AuthType::BasicAuth
        });

    let mut auth = client.authorize_url(CsrfToken::new_random);

    if let Some(scope_value) = &scopes
        && !scope_value.is_empty()
    {
        auth = auth.add_scope(Scope::new(scope_value.clone()));
    }

    if let Some(audience_value) = &audience
        && !audience_value.is_empty()
    {
        auth = auth.add_extra_param("audience", audience_value);
    }

    let (url, csrf_token) = auth.set_pkce_challenge(pkce_challenge).url();

    // let (url, csrf_token) = BasicClient::new(ClientId::new(client_id.to_string()))
    //     .set_auth_uri(AuthUrl::new(authorize_uri.to_string())?)
    //     .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string())?)
    //     .authorize_url(CsrfToken::new_random)
    //     .add_scopes(
    //         scopes
    //             .unwrap_or_default()
    //             .into_iter()
    //             .map(|s| Scope::new(s.to_string())),
    //     )
    //     .set_pkce_challenge(pkce_challenge)
    //     .url();
    Ok((url, csrf_token, pkce_verifier.into_secret()))
}

/// Retrieve access token (after call to generate_authorization)
pub async fn retrieve_access_token(
    access_token_uri: &str,
    redirect_uri: &str,
    client_id: &str,
    code: &str,
    verifier: &str,
    enable_trace: bool,
) -> Result<PkceTokenResult, String> {
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .connection_verbose(enable_trace)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    // println!("Token URL: {}, Redirect URL: {}, Client ID: {}, Code: {}, Verifier: {}", access_token_uri, redirect_uri, client_id, code, verifier);

    match BasicClient::new(ClientId::new(client_id.to_string()))
        .set_token_uri(TokenUrl::new(access_token_uri.to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string()).unwrap())
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(PkceCodeVerifier::new(verifier.to_string()))
        .request_async(&http_client)
        .await
    {
        Ok(token_result) => {
            let access_token = token_result.access_token().secret().to_string();
            let refresh_token = token_result.refresh_token().map(|t| t.secret().to_string());
            let expiration = token_result.expires_in().map(|e| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + e.as_secs()
            });

            Ok(PkceTokenResult {
                access_token,
                refresh_token,
                expiration,
            })
        }
        Err(err) => Err(format!("{err:?}")),
    }
}

/// Exchange refresh token for access token (after call to retrieve_access_token)
pub async fn refresh_token(
    access_token_uri: &str,
    refresh_token: &str,
    client_id: &str,
) -> Result<PkceTokenResult, String> {
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let token_result = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_token_uri(
            TokenUrl::new(access_token_uri.to_string()).expect("Unable to parse token_url"),
        )
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(&http_client)
        .await
        .expect("Unable to retrieve token");

    let access_token = token_result.access_token().secret().to_string();
    let refresh_token = token_result.refresh_token().map(|t| t.secret().to_string());
    let expiration = token_result.expires_in().map(|e| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + e.as_secs()
    });

    Ok(PkceTokenResult {
        access_token,
        refresh_token,
        expiration,
    })
}

#[cfg(test)]
pub mod tests {

    use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
    use reqwest::Url;
    use sha2::{Digest, Sha256};

    use super::generate_authorization;

    #[test]
    fn test_generate_authorization_url_has_client_id() {
        let (url, ..) = generate_authorization(
            "https://auth.com/",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        assert!(
            parsed
                .query_pairs()
                .any(|q| q.0 == "client_id" && q.1 == "client1")
        );
    }

    #[test]
    fn test_generate_authorization_url_has_redirct_uri() {
        let (url, ..) = generate_authorization(
            "https://auth.com/",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        assert!(
            parsed
                .query_pairs()
                .any(|q| q.0 == "redirect_uri" && q.1 == "https://localhost:3000/")
        );
    }

    #[test]
    fn test_generate_authorization_url_has_response_type() {
        let (url, ..) = generate_authorization(
            "https://auth.com/",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        assert!(
            parsed
                .query_pairs()
                .any(|q| q.0 == "response_type" && q.1 == "code")
        );
    }

    #[test]
    fn test_generate_authorization_url_includes_existing_query_param() {
        let (url, ..) = generate_authorization(
            "https://auth.com/?abc=123",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        assert!(parsed.query_pairs().any(|q| q.0 == "abc" && q.1 == "123"));
    }

    #[test]
    fn test_generate_authorization_url_includes_scopes() {
        let (url, ..) = generate_authorization(
            "https://auth.com/?abc=123",
            "https://localhost:3000/",
            "client1",
            false,
            Some("scope1 scope2".to_string()),
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        assert!(
            parsed
                .query_pairs()
                .any(|q| q.0 == "scope" && q.1 == "scope1 scope2")
        );
    }

    #[test]
    fn test_generate_authorization_url_includes_code_challenge_method() {
        let (url, ..) = generate_authorization(
            "https://auth.com/?abc=123",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        assert!(
            parsed
                .query_pairs()
                .any(|q| q.0 == "code_challenge_method" && q.1 == "S256")
        );
    }

    #[test]
    fn test_generate_authorization_url_includes_valid_challenge_and_verifier() {
        let (url, .., verifier) = generate_authorization(
            "https://auth.com/?abc=123",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        let parsed = Url::parse(url.as_str()).unwrap();
        let challenge = parsed
            .query_pairs()
            .find(|p| p.0 == "code_challenge")
            .map(|p| p.1)
            .unwrap();
        let mut hasher = Sha256::new();
        hasher.update(verifier);
        let hashed_secret = BASE64_URL_SAFE_NO_PAD.encode(hasher.finalize());

        assert_eq!(hashed_secret, challenge.to_string());
    }

    #[test]
    fn test_generate_authorization_url_returns_csrf_token() {
        let (_url, csrf_token, _verifier) = generate_authorization(
            "https://auth.com/?abc=123",
            "https://localhost:3000/",
            "client1",
            false,
            None,
            None,
        )
        .unwrap();
        assert_ne!(csrf_token.secret().as_str(), "");
    }
}

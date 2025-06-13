use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::Mutex;

/// Cached token
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CachedTokenInfo {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Expiration of token in seconds past Unix epoch
    pub expiration: Option<u64>,
}

pub static OAUTH2_TOKEN_CACHE: LazyLock<Mutex<HashMap<String, CachedTokenInfo>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Retrieve cached OAuth2 token
pub async fn retrieve_oauth2_token_from_cache(authorization_id: &str) -> Option<CachedTokenInfo> {
    let locked_cache = &mut OAUTH2_TOKEN_CACHE.lock().await;
    locked_cache.get(authorization_id).cloned()
}

/// Store OAuth2 token in cache
pub async fn store_oauth2_token_in_cache(authorization_id: &str, token_info: CachedTokenInfo) {
    let locked_cache = &mut OAUTH2_TOKEN_CACHE.lock().await;
    locked_cache.insert(authorization_id.to_owned(), token_info);
}

/// Clear all cached OAuth2 tokens
pub async fn clear_all_oauth2_tokens_from_cache<'a>() -> usize {
    let locked_cache = &mut OAUTH2_TOKEN_CACHE.lock().await;
    let count = locked_cache.len();
    locked_cache.clear();
    count
}

/// Clear specified cached OAuth2 credentials, returning true if value was cached
pub async fn clear_oauth2_token_from_cache(id: &str) -> bool {
    let mut locked_cache = OAUTH2_TOKEN_CACHE.lock().await;
    locked_cache.remove(&String::from(id)).is_some()
}

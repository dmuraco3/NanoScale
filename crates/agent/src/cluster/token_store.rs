use std::collections::HashMap;
use std::time::{Duration, Instant};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::sync::RwLock;

const TOKEN_TTL_SECONDS: u64 = 600;

#[derive(Debug, Default)]
pub struct TokenStore {
    tokens: RwLock<HashMap<String, Instant>>,
}

impl TokenStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn generate_token(&self) -> String {
        self.prune_expired().await;

        let token: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let expiration = Instant::now() + Duration::from_secs(TOKEN_TTL_SECONDS);
        self.tokens.write().await.insert(token.clone(), expiration);

        token
    }

    pub async fn consume_valid_token(&self, token: &str) -> bool {
        self.prune_expired().await;

        let mut tokens = self.tokens.write().await;
        if let Some(expiration) = tokens.get(token) {
            if *expiration > Instant::now() {
                tokens.remove(token);
                return true;
            }

            tokens.remove(token);
        }

        false
    }

    async fn prune_expired(&self) {
        let now = Instant::now();
        self.tokens
            .write()
            .await
            .retain(|_, expiration| *expiration > now);
    }

    #[must_use]
    pub const fn token_ttl_seconds() -> u64 {
        TOKEN_TTL_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn generate_and_consume_token_is_one_time() {
        let store = TokenStore::new();
        let token = store.generate_token().await;
        assert_eq!(token.len(), 32);

        assert!(store.consume_valid_token(&token).await);
        assert!(!store.consume_valid_token(&token).await);
    }

    #[tokio::test]
    async fn generate_token_produces_distinct_values() {
        let store = TokenStore::new();
        let token_a = store.generate_token().await;
        let token_b = store.generate_token().await;
        assert_ne!(token_a, token_b);
    }
}

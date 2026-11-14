//! User settings: the OpenRouter key and the model chosen for each AI stage.

use async_trait::async_trait;
use hands_off_core::model::ModelSettings;
use hands_off_core::storage::ProjectStore;
use hands_off_core::{Error, Result};
use hands_off_llm::KeyProvider;
use secrecy::SecretString;
use std::sync::Arc;

const KEY_SETTING: &str = "openrouter_api_key";
const MODELS_SETTING: &str = "models";
const MIN_KEY_LEN: usize = 8;
const MAX_KEY_LEN: usize = 512;

/// Where the active API key comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// The `OPENROUTER_API_KEY` environment variable.
    Environment,
    /// Saved through the settings API.
    Stored,
    /// No key is available.
    None,
}

/// Reads and writes settings, and supplies the API key to the model provider.
#[derive(Clone)]
pub struct Settings {
    environment_key: Option<SecretString>,
    store: Arc<dyn ProjectStore>,
}

impl Settings {
    /// Settings backed by `store`, preferring `environment_key` when present.
    #[must_use]
    pub fn new(environment_key: Option<SecretString>, store: Arc<dyn ProjectStore>) -> Self {
        Self {
            environment_key,
            store,
        }
    }

    /// Which source currently provides the key.
    ///
    /// # Errors
    /// [`Error::Internal`] if storage fails.
    pub async fn key_source(&self) -> Result<KeySource> {
        if self.environment_key.is_some() {
            return Ok(KeySource::Environment);
        }
        Ok(if self.store.setting(KEY_SETTING).await?.is_some() {
            KeySource::Stored
        } else {
            KeySource::None
        })
    }

    /// Save an API key.
    ///
    /// # Errors
    /// [`Error::Validation`] if the key has an implausible length or contains whitespace.
    pub async fn set_key(&self, key: &str) -> Result<()> {
        let key = key.trim();
        if !(MIN_KEY_LEN..=MAX_KEY_LEN).contains(&key.len()) || key.chars().any(char::is_whitespace)
        {
            return Err(Error::Validation(
                "that does not look like an API key".into(),
            ));
        }
        self.store.set_setting(KEY_SETTING, key).await
    }

    /// Remove the stored key. An environment key is unaffected.
    ///
    /// # Errors
    /// [`Error::Internal`] if storage fails.
    pub async fn clear_key(&self) -> Result<()> {
        self.store.delete_setting(KEY_SETTING).await
    }

    /// The model chosen for each stage (defaults until changed).
    ///
    /// # Errors
    /// [`Error::Internal`] if storage fails or the saved settings are unreadable.
    pub async fn models(&self) -> Result<ModelSettings> {
        match self.store.setting(MODELS_SETTING).await? {
            Some(json) => serde_json::from_str(&json).map_err(Error::internal),
            None => Ok(ModelSettings::default()),
        }
    }

    /// Save the model for each stage.
    ///
    /// # Errors
    /// [`Error::Validation`] if any model id is malformed.
    pub async fn set_models(&self, models: &ModelSettings) -> Result<()> {
        models.validate()?;
        let json = serde_json::to_string(models).map_err(Error::internal)?;
        self.store.set_setting(MODELS_SETTING, &json).await
    }
}

#[async_trait]
impl KeyProvider for Settings {
    async fn api_key(&self) -> Result<Option<SecretString>> {
        if let Some(key) = &self.environment_key {
            return Ok(Some(key.clone()));
        }
        Ok(self
            .store
            .setting(KEY_SETTING)
            .await?
            .map(SecretString::from))
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;
    use crate::storage::SqliteStore;

    async fn settings(env: Option<&str>) -> Settings {
        let store: Arc<dyn ProjectStore> = Arc::new(SqliteStore::in_memory().await.unwrap());
        Settings::new(env.map(|k| SecretString::from(k.to_owned())), store)
    }

    #[tokio::test]
    async fn stores_and_clears_a_key() {
        let settings = settings(None).await;
        assert_eq!(settings.key_source().await.unwrap(), KeySource::None);
        assert!(settings.api_key().await.unwrap().is_none());

        settings.set_key("  sk-or-v1-abcdef  ").await.unwrap();
        assert_eq!(settings.key_source().await.unwrap(), KeySource::Stored);
        assert_eq!(
            settings.api_key().await.unwrap().unwrap().expose_secret(),
            "sk-or-v1-abcdef"
        );

        settings.clear_key().await.unwrap();
        assert_eq!(settings.key_source().await.unwrap(), KeySource::None);
    }

    #[tokio::test]
    async fn the_environment_key_wins_and_cannot_be_cleared() {
        let settings = settings(Some("env-key-12345")).await;
        settings.set_key("stored-key-12345").await.unwrap();
        assert_eq!(settings.key_source().await.unwrap(), KeySource::Environment);
        assert_eq!(
            settings.api_key().await.unwrap().unwrap().expose_secret(),
            "env-key-12345"
        );
        settings.clear_key().await.unwrap();
        assert_eq!(settings.key_source().await.unwrap(), KeySource::Environment);
    }

    #[tokio::test]
    async fn implausible_keys_are_rejected() {
        let settings = settings(None).await;
        for bad in ["", "short", "has a space in it", &"k".repeat(513)] {
            assert!(
                matches!(settings.set_key(bad).await, Err(Error::Validation(_))),
                "{bad:?}"
            );
        }
        assert_eq!(settings.key_source().await.unwrap(), KeySource::None);
    }

    #[tokio::test]
    async fn models_default_then_persist_and_validate() {
        let settings = settings(None).await;
        assert_eq!(settings.models().await.unwrap(), ModelSettings::default());

        let chosen = ModelSettings {
            suggester: "typesafe/jev-router".into(),
            ..ModelSettings::default()
        };
        settings.set_models(&chosen).await.unwrap();
        assert_eq!(settings.models().await.unwrap(), chosen);

        let bad = ModelSettings {
            generator: "no spaces allowed".into(),
            ..ModelSettings::default()
        };
        assert!(settings.set_models(&bad).await.is_err());
        assert_eq!(settings.models().await.unwrap(), chosen);
    }
}

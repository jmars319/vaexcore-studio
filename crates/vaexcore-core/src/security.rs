use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

const REDACTED: &str = "[redacted]";

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SensitiveString(String);

impl SensitiveString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SensitiveString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SensitiveString")
            .field(&REDACTED)
            .finish()
    }
}

impl Serialize for SensitiveString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(REDACTED)
    }
}

impl<'de> Deserialize<'de> for SensitiveString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SecretRef {
    pub provider: String,
    pub id: String,
}

impl SecretRef {
    pub fn local(id: impl Into<String>) -> Self {
        Self {
            provider: "local-sqlite".to_string(),
            id: id.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
    #[error("secret store failed: {0}")]
    Store(String),
}

pub trait SecretStore: Send + Sync {
    fn put_secret(
        &self,
        scope: &str,
        value: &SensitiveString,
    ) -> Result<SecretRef, SecretStoreError>;

    fn get_secret(
        &self,
        reference: &SecretRef,
    ) -> Result<Option<SensitiveString>, SecretStoreError>;
}

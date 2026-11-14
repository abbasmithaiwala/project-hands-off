//! Strongly typed identifiers.

use std::fmt;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::Error;

macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generate a fresh random identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value)
                    .map(Self)
                    .map_err(|_| Error::Validation(format!("invalid {}: {value}", stringify!($name))))
            }
        }
    };
}

uuid_id!(
    /// Identifier of an uploaded image.
    ImageId
);
uuid_id!(
    /// Identifier of a background job.
    JobId
);

/// Index of a detected object within one image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(transparent)]
pub struct ObjectId(pub u32);

/// Lowercase hex SHA-256 digest of an image's bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(try_from = "String")]
pub struct ContentHash(String);

impl ContentHash {
    /// Length of a hex-encoded SHA-256 digest.
    pub const LEN: usize = 64;

    /// Validate and wrap a hex digest.
    ///
    /// # Errors
    /// Returns [`Error::Validation`] unless the value is 64 lowercase hex characters.
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        let valid = value.len() == Self::LEN
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if valid {
            Ok(Self(value))
        } else {
            Err(Error::Validation(
                "content hash must be 64 lowercase hex characters".into(),
            ))
        }
    }

    /// The digest as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ContentHash {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_id_round_trips_through_string() {
        let id = ImageId::new();
        assert_eq!(id.to_string().parse::<ImageId>().unwrap(), id);
    }

    #[test]
    fn image_id_rejects_garbage() {
        assert!("not-a-uuid".parse::<ImageId>().is_err());
    }

    #[test]
    fn content_hash_accepts_valid_digest() {
        assert!(ContentHash::new("a".repeat(64)).is_ok());
    }

    #[test]
    fn content_hash_rejects_wrong_length_case_and_characters() {
        assert!(ContentHash::new("a".repeat(63)).is_err());
        assert!(ContentHash::new("A".repeat(64)).is_err());
        assert!(ContentHash::new("g".repeat(64)).is_err());
    }

    #[test]
    fn content_hash_is_validated_on_deserialize() {
        assert!(serde_json::from_str::<ContentHash>("\"short\"").is_err());
    }
}

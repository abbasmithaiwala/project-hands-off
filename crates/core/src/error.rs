//! Error type shared across crates, with stable machine-readable codes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Stable error identifiers exposed through the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The request was malformed or failed validation.
    Validation,
    /// The uploaded media type is not supported.
    UnsupportedMedia,
    /// The payload exceeds a configured limit.
    PayloadTooLarge,
    /// The requested resource does not exist.
    NotFound,
    /// The operation exceeded its time budget.
    Timeout,
    /// The operation was cancelled.
    Cancelled,
    /// An unexpected server-side failure.
    Internal,
}

/// Errors produced by Hands-Off crates.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// Unsupported media type.
    #[error("{0}")]
    UnsupportedMedia(String),
    /// A size or dimension limit was exceeded.
    #[error("{0}")]
    PayloadTooLarge(String),
    /// The named resource does not exist.
    #[error("{0} not found")]
    NotFound(&'static str),
    /// The operation timed out.
    #[error("operation timed out")]
    Timeout,
    /// The operation was cancelled.
    #[error("operation cancelled")]
    Cancelled,
    /// An unexpected failure; the source is logged but never shown to clients.
    #[error("internal error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The stable code for this error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Validation(_) => ErrorCode::Validation,
            Self::UnsupportedMedia(_) => ErrorCode::UnsupportedMedia,
            Self::PayloadTooLarge(_) => ErrorCode::PayloadTooLarge,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Timeout => ErrorCode::Timeout,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    /// Wrap any error as an internal failure.
    pub fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal(Box::new(source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_variant_to_its_code() {
        let cases = [
            (Error::Validation("x".into()), ErrorCode::Validation),
            (
                Error::UnsupportedMedia("x".into()),
                ErrorCode::UnsupportedMedia,
            ),
            (
                Error::PayloadTooLarge("x".into()),
                ErrorCode::PayloadTooLarge,
            ),
            (Error::NotFound("image"), ErrorCode::NotFound),
            (Error::Timeout, ErrorCode::Timeout),
            (Error::Cancelled, ErrorCode::Cancelled),
            (Error::internal(std::fmt::Error), ErrorCode::Internal),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn internal_error_hides_its_source_in_display() {
        let error = Error::internal(std::io::Error::other("disk exploded"));
        assert_eq!(error.to_string(), "internal error");
    }

    #[test]
    fn codes_serialize_as_snake_case() {
        let json = serde_json::to_string(&ErrorCode::PayloadTooLarge).unwrap();
        assert_eq!(json, "\"payload_too_large\"");
    }
}

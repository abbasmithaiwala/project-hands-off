//! Mapping from domain errors to HTTP responses.

use axum::Json;
use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use hands_off_core::{Error, ErrorCode};
use serde::Serialize;
use utoipa::ToSchema;

/// JSON body of every error response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Stable machine-readable code.
    pub code: ErrorCode,
    /// Human-readable explanation.
    pub message: String,
}

/// A domain [`Error`] that renders as an HTTP response.
#[derive(Debug)]
pub struct ApiError(pub Error);

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

impl From<MultipartError> for ApiError {
    fn from(error: MultipartError) -> Self {
        let message = error.body_text();
        Self(if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            Error::PayloadTooLarge("the upload exceeds the maximum allowed size".into())
        } else {
            Error::Validation(message)
        })
    }
}

fn status_for(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::Validation => StatusCode::BAD_REQUEST,
        ErrorCode::UnsupportedMedia => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::Timeout => StatusCode::GATEWAY_TIMEOUT,
        ErrorCode::Cancelled => StatusCode::CONFLICT,
        ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = self.0.code();
        if code == ErrorCode::Internal {
            tracing::error!(error = ?self.0, "request failed");
        }
        let body = ErrorBody {
            code,
            message: self.0.to_string(),
        };
        (status_for(code), Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_distinct_sensible_status() {
        assert_eq!(status_for(ErrorCode::Validation), StatusCode::BAD_REQUEST);
        assert_eq!(
            status_for(ErrorCode::UnsupportedMedia),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            status_for(ErrorCode::PayloadTooLarge),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(status_for(ErrorCode::NotFound), StatusCode::NOT_FOUND);
        assert_eq!(status_for(ErrorCode::Timeout), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(status_for(ErrorCode::Cancelled), StatusCode::CONFLICT);
        assert_eq!(
            status_for(ErrorCode::Unavailable),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            status_for(ErrorCode::Internal),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn internal_errors_do_not_leak_their_source() {
        let response =
            ApiError(Error::internal(std::io::Error::other("db password"))).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

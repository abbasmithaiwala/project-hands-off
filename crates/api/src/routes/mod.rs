//! HTTP route handlers, grouped by resource.
//!
//! Error responses are documented per handler through the `OpenAPI` annotations, so the
//! `missing_errors_doc` lint is not applied here.
#![allow(clippy::missing_errors_doc)]

pub mod edits;
pub mod health;
pub mod images;
pub mod jobs;
pub mod objects;
pub mod projects;
pub mod settings;

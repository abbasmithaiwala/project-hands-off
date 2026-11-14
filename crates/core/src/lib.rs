//! Domain types, errors and configuration shared by every crate.

pub mod config;
pub mod error;
pub mod geometry;
pub mod ids;
pub mod job;
pub mod model;
pub mod storage;

pub use config::{Config, Limits};
pub use error::{Error, ErrorCode, Result};
pub use geometry::{BBox, NormalizedPoint};
pub use ids::{ContentHash, ImageId, JobId, ObjectId};

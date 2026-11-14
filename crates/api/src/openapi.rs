//! The OpenAPI description of the HTTP API. The TypeScript client is generated from it.

use hands_off_core::job::{JobEvent, JobRecord, JobStatus};
use hands_off_core::model::{
    Axis, DetectedObject, Edit, EditChange, ImageFormat, ImageRecord, ModelSettings, Property,
    PropertyList, Widget, WidgetKind,
};
use hands_off_core::{BBox, ContentHash, ErrorCode, ImageId, JobId, NormalizedPoint, ObjectId};
use utoipa::OpenApi;

use crate::error::ErrorBody;
use crate::routes::{health, images, jobs, objects, settings};

/// OpenAPI document for the v1 API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Hands-Off API",
        description = "Edit images by direct manipulation.",
        license(name = "Apache-2.0")
    ),
    paths(
        health::health,
        images::upload,
        images::get_image,
        images::get_original,
        images::get_thumbnail,
        images::analyze,
        images::list_objects,
        images::get_mask,
        objects::properties,
        objects::widget,
        settings::get_settings,
        settings::set_key,
        settings::clear_key,
        settings::set_models,
        jobs::get_job,
        jobs::cancel_job,
        jobs::job_events,
    ),
    components(schemas(
        Axis, BBox, ContentHash, DetectedObject, Edit, EditChange, ErrorBody, ErrorCode,
        ImageFormat, ImageId, ImageRecord, JobEvent, JobId, JobRecord, JobStatus, NormalizedPoint,
        ObjectId, Property, Widget, WidgetKind, images::JobCreated, images::UploadForm, ModelSettings, PropertyList, objects::PlanWidget,
        settings::KeyOrigin, settings::KeyStatus, settings::SetKey, settings::SettingsView,
    )),
    tags(
        (name = "system", description = "Server status"),
        (name = "images", description = "Upload and fetch images"),
        (name = "objects", description = "AI-assisted controls for detected objects"),
        (name = "settings", description = "API key and model choices"),
        (name = "jobs", description = "Background jobs and their event streams"),
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_documents_every_route() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = spec["paths"].as_object().unwrap();
        for path in [
            "/api/v1/health",
            "/api/v1/images",
            "/api/v1/images/{id}",
            "/api/v1/images/{id}/original",
            "/api/v1/images/{id}/thumbnail",
            "/api/v1/images/{id}/analyze",
            "/api/v1/images/{id}/objects",
            "/api/v1/images/{id}/objects/{object_id}/mask",
            "/api/v1/images/{id}/objects/{object_id}/properties",
            "/api/v1/images/{id}/objects/{object_id}/widget",
            "/api/v1/settings",
            "/api/v1/settings/openrouter-key",
            "/api/v1/settings/models",
            "/api/v1/jobs/{id}",
            "/api/v1/jobs/{id}/events",
        ] {
            assert!(paths.contains_key(path), "missing {path}");
        }
    }

    #[test]
    fn spec_includes_the_widget_union_for_the_frontend() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let schemas = spec["components"]["schemas"].as_object().unwrap();
        for name in ["Widget", "Edit", "JobEvent", "ErrorBody", "ImageRecord"] {
            assert!(schemas.contains_key(name), "missing schema {name}");
        }
    }
}

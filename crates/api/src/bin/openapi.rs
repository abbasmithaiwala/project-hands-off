//! Prints the `OpenAPI` document to stdout. Used by `just gen-api`.

use hands_off_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> Result<(), serde_json::Error> {
    println!("{}", ApiDoc::openapi().to_pretty_json()?);
    Ok(())
}

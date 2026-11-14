//! A rule-based stand-in for the models, so the whole app can be tried without an API key
//! or credits. Enabled with `HANDS_OFF_DEMO_AI=1`.

use std::time::Duration;

use async_trait::async_trait;
use hands_off_core::{Error, Result};
use serde_json::{Value, json};

use crate::contracts::{SUGGESTIONS_SCHEMA, WIDGET_SCHEMA};
use crate::provider::{
    ImageGenerator, ImageRequest, ImageResponse, JsonRequest, JsonResponse, LlmProvider, Usage,
};

/// Answers suggestion and widget requests from simple rules.
#[derive(Debug, Default, Clone, Copy)]
pub struct DemoProvider;

fn line_value<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.lines()
        .find_map(|l| l.strip_prefix(prefix))
        .map_or("", str::trim)
}

fn suggestions(label: &str) -> Value {
    let property = |key: &str, name: &str, widget: &str| json!({ "key": key, "label": name, "widget": widget });
    let properties = match label {
        "couch" | "chair" | "bed" | "bench" => vec![
            property("length", &format!("{} length", capitalise(label)), "slider"),
            property("height", &format!("{} height", capitalise(label)), "slider"),
            property("shape", "Outline shape", "control_points"),
        ],
        "person" => vec![
            property("arm_position", "Arm position", "multi_handle"),
            property("expression", "Expression", "multi_handle"),
            property("build", "Body width", "slider"),
        ],
        "bottle" | "vase" | "cup" | "bowl" | "wine glass" => vec![
            property("shape", "Shape", "control_points"),
            property("height", "Height", "slider"),
            property("fill_level", "Fill level", "slider"),
        ],
        "sandwich" | "pizza" | "cake" | "donut" | "hot dog" => vec![
            property("height", "Stack height", "slider"),
            property("width", "Width", "slider"),
        ],
        _ => vec![
            property("size", "Size", "slider"),
            property("shape", "Shape", "control_points"),
        ],
    };
    json!({ "properties": properties })
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

fn widget(key: &str, kind: &str) -> Value {
    let point = |x: f64, y: f64| json!({ "x": x, "y": y });
    match kind {
        "slider" => {
            let vertical = key.contains("height");
            let (start, end) = if vertical {
                (point(0.5, 0.9), point(0.5, 0.1))
            } else {
                (point(0.1, 0.5), point(0.9, 0.5))
            };
            json!({ "kind": kind, "axis": { "start": start, "end": end }, "min": 0.5, "max": 1.5, "initial": 1.0, "points": null, "handles": null })
        }
        "control_points" => {
            let ring: Vec<Value> = (0..8)
                .map(|i| {
                    let angle = f64::from(i) * std::f64::consts::TAU / 8.0;
                    point(0.5 + 0.35 * angle.cos(), 0.5 + 0.35 * angle.sin())
                })
                .collect();
            json!({ "kind": kind, "axis": null, "min": null, "max": null, "initial": null, "points": ring, "handles": null })
        }
        _ => {
            json!({ "kind": kind, "axis": null, "min": null, "max": null, "initial": null, "points": null,
                     "handles": [point(0.25, 0.3), point(0.75, 0.3), point(0.3, 0.75), point(0.7, 0.75)] })
        }
    }
}

#[async_trait]
impl LlmProvider for DemoProvider {
    async fn complete_json(&self, request: JsonRequest) -> Result<JsonResponse> {
        let label = line_value(&request.user_text, "Object:");
        let content = match request.schema_name {
            SUGGESTIONS_SCHEMA => suggestions(label),
            WIDGET_SCHEMA => {
                let key = line_value(&request.user_text, "Property:")
                    .split_whitespace()
                    .next()
                    .unwrap_or("size");
                widget(key, line_value(&request.user_text, "Widget:"))
            }
            other => {
                return Err(Error::Unavailable(format!(
                    "the demo provider cannot answer `{other}` requests"
                )));
            }
        };
        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok(JsonResponse {
            content: content.to_string(),
            model: "demo".into(),
            usage: Usage::default(),
            latency: Duration::from_millis(250),
        })
    }
}

/// Stands in for the image model in demo mode by returning the preview (or the original) as
/// the result, so the whole flow can be tried without an account.
#[derive(Debug, Default, Clone, Copy)]
pub struct DemoGenerator;

#[async_trait]
impl ImageGenerator for DemoGenerator {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResponse> {
        tokio::time::sleep(Duration::from_millis(900)).await;
        let source = request.preview.unwrap_or(request.original);
        Ok(ImageResponse {
            bytes: source.bytes,
            model: "demo".into(),
            usage: Usage::default(),
            latency: Duration::from_millis(900),
        })
    }
}

#[cfg(test)]
mod tests {
    use hands_off_core::NormalizedPoint;
    use hands_off_core::model::{Property, WidgetKind};

    use super::*;
    use crate::contracts::{
        parse_suggestions, parse_widget, planner_user_text, suggester_user_text, suggestion_schema,
    };
    use crate::provider::Stage;

    fn request(schema_name: &'static str, user_text: String) -> JsonRequest {
        JsonRequest {
            stage: Stage::Suggester,
            models: Vec::new(),
            system: String::new(),
            user_text,
            images: Vec::new(),
            schema_name,
            schema: suggestion_schema(),
            repair: None,
        }
    }

    fn place(x: f32, y: f32) -> NormalizedPoint {
        NormalizedPoint::new(x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)).unwrap()
    }

    #[tokio::test]
    async fn suggestions_pass_the_real_validator_for_common_labels() {
        for label in ["couch", "person", "bottle", "sandwich", "toaster"] {
            let answer = DemoProvider
                .complete_json(request(SUGGESTIONS_SCHEMA, suggester_user_text(label)))
                .await
                .unwrap();
            let parsed = parse_suggestions(&answer.content).unwrap();
            assert!(parsed.len() >= 2, "{label}");
        }
    }

    #[tokio::test]
    async fn widgets_pass_the_real_validator_for_every_kind() {
        for (key, kind) in [
            ("length", WidgetKind::Slider),
            ("shape", WidgetKind::ControlPoints),
            ("arm_position", WidgetKind::MultiHandle),
        ] {
            let property = Property {
                key: key.into(),
                label: key.into(),
                widget: kind,
            };
            let answer = DemoProvider
                .complete_json(request(
                    WIDGET_SCHEMA,
                    planner_user_text("couch", &property, [0.1, 0.1, 0.9, 0.9]),
                ))
                .await
                .unwrap();
            assert!(
                parse_widget(&answer.content, kind, &place, &|p| p).is_ok(),
                "{key}"
            );
        }
    }

    #[tokio::test]
    async fn unknown_schemas_are_refused() {
        assert!(
            DemoProvider
                .complete_json(request("something_else", String::new()))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn the_demo_generator_returns_the_preview_when_there_is_one() {
        let request = |preview: Option<Vec<u8>>| ImageRequest {
            models: Vec::new(),
            instruction: String::new(),
            original: crate::provider::ImageInput {
                mime: "image/png",
                bytes: vec![1],
            },
            preview: preview.map(|bytes| crate::provider::ImageInput {
                mime: "image/png",
                bytes,
            }),
        };
        assert_eq!(
            DemoGenerator
                .generate(request(Some(vec![9])))
                .await
                .unwrap()
                .bytes,
            [9]
        );
        assert_eq!(
            DemoGenerator.generate(request(None)).await.unwrap().bytes,
            [1]
        );
    }
}

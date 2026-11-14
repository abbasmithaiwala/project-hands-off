//! Compares models on the real suggester and planner prompts using the golden images.
//!
//! Usage: `OPENROUTER_API_KEY=... cargo run -p hands-off-llm --example benchmark -- model1 model2 ...`
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hands_off_core::NormalizedPoint;
use hands_off_core::model::{Property, WidgetKind};
use hands_off_llm::contracts::{
    PLANNER_SYSTEM, SUGGESTER_SYSTEM, SUGGESTIONS_SCHEMA, WIDGET_SCHEMA, parse_suggestions,
    parse_widget, planner_user_text, suggester_user_text, suggestion_schema, widget_schema,
};
use hands_off_llm::openrouter::OpenRouterProvider;
use hands_off_llm::{ImageInput, JsonRequest, KeyProvider, LlmProvider, Stage};
use image::imageops::FilterType;
use secrecy::SecretString;

struct EnvKey;

#[async_trait]
impl KeyProvider for EnvKey {
    async fn api_key(&self) -> hands_off_core::Result<Option<SecretString>> {
        Ok(std::env::var("OPENROUTER_API_KEY")
            .ok()
            .map(SecretString::from))
    }
}

struct Case {
    image: &'static str,
    label: &'static str,
    bbox: [f32; 4],
    property: (&'static str, &'static str, WidgetKind),
}

const CASES: [Case; 5] = [
    Case {
        image: "couch",
        label: "couch",
        bbox: [263.0, 333.0, 870.0, 698.0],
        property: ("length", "Couch length", WidgetKind::Slider),
    },
    Case {
        image: "bottle",
        label: "vase",
        bbox: [58.0, 22.0, 659.0, 809.0],
        property: ("shape", "Bottle shape", WidgetKind::ControlPoints),
    },
    Case {
        image: "person_pose",
        label: "person",
        bbox: [249.0, 86.0, 1027.0, 1402.0],
        property: ("arm_position", "Arm position", WidgetKind::MultiHandle),
    },
    Case {
        image: "burger",
        label: "sandwich",
        bbox: [28.0, 44.0, 1256.0, 1055.0],
        property: ("height", "Burger height", WidgetKind::Slider),
    },
    Case {
        image: "person_cup",
        label: "bowl",
        bbox: [371.0, 800.0, 681.0, 986.0],
        property: ("fill_level", "Drink level", WidgetKind::Slider),
    },
];

fn crop(case: &Case) -> Vec<u8> {
    let image = image::open(format!("eval/golden/{}.jpg", case.image)).unwrap();
    let (w, h) = (image.width() as f32, image.height() as f32);
    let [x0, y0, x1, y1] = case.bbox;
    let (pad_x, pad_y) = ((x1 - x0) * 0.1, (y1 - y0) * 0.1);
    let (cx0, cy0) = ((x0 - pad_x).max(0.0), (y0 - pad_y).max(0.0));
    let (cx1, cy1) = ((x1 + pad_x).min(w), (y1 + pad_y).min(h));
    let cropped = image
        .crop_imm(
            cx0 as u32,
            cy0 as u32,
            (cx1 - cx0) as u32,
            (cy1 - cy0) as u32,
        )
        .resize(768, 768, FilterType::Triangle);
    let mut out = Cursor::new(Vec::new());
    cropped
        .to_rgb8()
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .unwrap();
    out.into_inner()
}

#[derive(Default)]
struct Tally {
    ok: u32,
    total: u32,
    seconds: f64,
    cost: f64,
    failures: Vec<String>,
}

fn place(x: f32, y: f32) -> NormalizedPoint {
    NormalizedPoint::new(x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)).unwrap()
}

async fn run_model(provider: Arc<OpenRouterProvider>, model: String) -> (String, Tally) {
    let mut tally = Tally::default();
    for case in &CASES {
        let image = ImageInput {
            mime: "image/jpeg",
            bytes: crop(case),
        };
        let property = Property {
            key: case.property.0.into(),
            label: case.property.1.into(),
            widget: case.property.2,
        };
        let requests = [
            (
                Stage::Suggester,
                SUGGESTER_SYSTEM,
                suggester_user_text(case.label),
                SUGGESTIONS_SCHEMA,
                suggestion_schema(),
            ),
            (
                Stage::UiPlanner,
                PLANNER_SYSTEM,
                planner_user_text(case.label, &property, [0.09, 0.09, 0.91, 0.91]),
                WIDGET_SCHEMA,
                widget_schema(),
            ),
        ];
        for (stage, system, user_text, schema_name, schema) in requests {
            tally.total += 1;
            let request = JsonRequest {
                stage,
                models: vec![model.clone()],
                system: system.into(),
                user_text,
                images: vec![image.clone()],
                schema_name,
                schema,
                repair: None,
            };
            match provider.complete_json(request).await {
                Ok(response) => {
                    tally.seconds += response.latency.as_secs_f64();
                    tally.cost += response.usage.cost_usd.unwrap_or(0.0);
                    let verdict = if stage == Stage::Suggester {
                        parse_suggestions(&response.content).map(|_| ())
                    } else {
                        parse_widget(&response.content, property.widget, &place, &|p| p).map(|_| ())
                    };
                    match verdict {
                        Ok(()) => tally.ok += 1,
                        Err(problem) => tally.failures.push(format!(
                            "{} {}: {problem}",
                            case.image,
                            stage.name()
                        )),
                    }
                }
                Err(error) => {
                    tally
                        .failures
                        .push(format!("{} {}: {error}", case.image, stage.name()));
                }
            }
        }
    }
    (model, tally)
}

#[tokio::main]
async fn main() {
    let models: Vec<String> = std::env::args().skip(1).collect();
    let provider = Arc::new(
        OpenRouterProvider::new(Arc::new(EnvKey)).with_backoff(Duration::from_millis(500)),
    );
    let results = futures::future::join_all(
        models
            .into_iter()
            .map(|m| run_model(Arc::clone(&provider), m)),
    )
    .await;
    println!(
        "{:<34} {:>7} {:>10} {:>9}",
        "model", "valid", "avg secs", "cost $"
    );
    for (model, t) in &results {
        let answered = f64::from(t.total).max(1.0);
        println!(
            "{model:<34} {:>3}/{:<3} {:>10.1} {:>9.4}",
            t.ok,
            t.total,
            t.seconds / answered,
            t.cost
        );
    }
    for (model, t) in &results {
        for failure in t.failures.iter().take(4) {
            println!(
                "  [{model}] {}",
                failure.chars().take(170).collect::<String>()
            );
        }
    }
}

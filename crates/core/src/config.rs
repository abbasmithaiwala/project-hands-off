//! Runtime configuration. Everything has a default; only the OpenRouter key is needed to
//! generate edits, and it is read lazily so the app can start without it.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use secrecy::SecretString;

use crate::error::{Error, Result};

const DEFAULT_BIND: &str = "127.0.0.1:8080";
const DEFAULT_MAX_UPLOAD_MB: usize = 25;
const DEFAULT_MAX_DIMENSION: u32 = 8192;
const DEFAULT_JOB_TIMEOUT_SECS: u64 = 180;
const DEFAULT_CORS_ORIGINS: [&str; 2] = ["http://localhost:5173", "http://127.0.0.1:5173"];
const BYTES_PER_MB: usize = 1024 * 1024;

/// Upload limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum size of an uploaded file in bytes.
    pub max_upload_bytes: usize,
    /// Maximum width or height in pixels.
    pub max_dimension: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_upload_bytes: DEFAULT_MAX_UPLOAD_MB * BYTES_PER_MB,
            max_dimension: DEFAULT_MAX_DIMENSION,
        }
    }
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to.
    pub bind: SocketAddr,
    /// Directory holding the database and image blobs.
    pub data_dir: PathBuf,
    /// Directory holding downloaded model weights.
    pub model_dir: PathBuf,
    /// Where to download the RF-DETR-Seg ONNX model from, if hosted.
    pub rfdetr_seg_url: Option<String>,
    /// Expected SHA-256 of the RF-DETR-Seg model, if known.
    pub rfdetr_seg_sha256: Option<String>,
    /// Answer AI requests from built-in rules instead of calling models (no key needed).
    pub demo_ai: bool,
    /// OpenRouter API key, if provided.
    pub openrouter_api_key: Option<SecretString>,
    /// Upload limits.
    pub limits: Limits,
    /// Maximum run time of one background job.
    pub job_timeout: Duration,
    /// Origins allowed by CORS.
    pub cors_origins: Vec<String>,
}

impl Config {
    /// Build a configuration from the process environment.
    ///
    /// # Errors
    /// Returns [`Error::Validation`] naming the offending variable.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Build a configuration from an arbitrary variable lookup (used by tests).
    ///
    /// # Errors
    /// Returns [`Error::Validation`] naming the offending variable.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };

        let bind = get("HANDS_OFF_BIND").unwrap_or_else(|| DEFAULT_BIND.to_owned());
        let bind = bind.parse::<SocketAddr>().map_err(|_| {
            invalid(
                "HANDS_OFF_BIND",
                "expected host:port, for example 127.0.0.1:8080",
            )
        })?;

        let max_upload_mb = parse_positive(
            "HANDS_OFF_MAX_UPLOAD_MB",
            get("HANDS_OFF_MAX_UPLOAD_MB"),
            DEFAULT_MAX_UPLOAD_MB,
        )?;
        let max_dimension = parse_positive(
            "HANDS_OFF_MAX_DIMENSION",
            get("HANDS_OFF_MAX_DIMENSION"),
            DEFAULT_MAX_DIMENSION,
        )?;
        let job_timeout_secs = parse_positive(
            "HANDS_OFF_JOB_TIMEOUT_SECS",
            get("HANDS_OFF_JOB_TIMEOUT_SECS"),
            DEFAULT_JOB_TIMEOUT_SECS,
        )?;

        let cors_origins = get("HANDS_OFF_CORS_ORIGINS").map_or_else(
            || {
                DEFAULT_CORS_ORIGINS
                    .iter()
                    .map(|o| (*o).to_owned())
                    .collect()
            },
            |list| {
                list.split(',')
                    .map(|o| o.trim().to_owned())
                    .filter(|o| !o.is_empty())
                    .collect()
            },
        );

        Ok(Self {
            bind,
            data_dir: get("HANDS_OFF_DATA_DIR")
                .map_or_else(|| PathBuf::from("data"), PathBuf::from),
            model_dir: get("HANDS_OFF_MODEL_DIR")
                .map_or_else(|| PathBuf::from("models"), PathBuf::from),
            rfdetr_seg_url: get("HANDS_OFF_RFDETR_SEG_URL"),
            rfdetr_seg_sha256: get("HANDS_OFF_RFDETR_SEG_SHA256"),
            demo_ai: get("HANDS_OFF_DEMO_AI")
                .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes")),
            openrouter_api_key: get("OPENROUTER_API_KEY").map(SecretString::from),
            limits: Limits {
                max_upload_bytes: max_upload_mb * BYTES_PER_MB,
                max_dimension,
            },
            job_timeout: Duration::from_secs(job_timeout_secs),
            cors_origins,
        })
    }
}

fn invalid(name: &str, reason: &str) -> Error {
    Error::Validation(format!("{name}: {reason}"))
}

fn parse_positive<T>(name: &str, raw: Option<String>, default: T) -> Result<T>
where
    T: std::str::FromStr + PartialOrd + From<u8>,
{
    let Some(raw) = raw else { return Ok(default) };
    let value = raw
        .parse::<T>()
        .map_err(|_| invalid(name, "expected a positive whole number"))?;
    if value > T::from(0) {
        Ok(value)
    } else {
        Err(invalid(name, "must be greater than zero"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use secrecy::ExposeSecret;

    use super::*;

    fn config(vars: &[(&str, &str)]) -> Result<Config> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|name| map.get(name).cloned())
    }

    #[test]
    fn empty_environment_yields_working_defaults() {
        let cfg = config(&[]).unwrap();
        assert_eq!(cfg.bind.to_string(), DEFAULT_BIND);
        assert_eq!(cfg.limits, Limits::default());
        assert!(cfg.openrouter_api_key.is_none());
        assert!(cfg.rfdetr_seg_url.is_none());
        assert!(!cfg.demo_ai);
        assert_eq!(cfg.cors_origins.len(), 2);
    }

    #[test]
    fn overrides_are_applied() {
        let cfg = config(&[
            ("HANDS_OFF_BIND", "0.0.0.0:9000"),
            ("HANDS_OFF_MAX_UPLOAD_MB", "5"),
            ("HANDS_OFF_DATA_DIR", "/var/lib/hands-off"),
            (
                "HANDS_OFF_CORS_ORIGINS",
                "https://a.example, https://b.example",
            ),
            ("OPENROUTER_API_KEY", "sk-test"),
        ])
        .unwrap();
        assert_eq!(cfg.bind.port(), 9000);
        assert_eq!(cfg.limits.max_upload_bytes, 5 * BYTES_PER_MB);
        assert_eq!(cfg.data_dir, PathBuf::from("/var/lib/hands-off"));
        assert_eq!(cfg.cors_origins, ["https://a.example", "https://b.example"]);
        assert_eq!(cfg.openrouter_api_key.unwrap().expose_secret(), "sk-test");
    }

    #[test]
    fn demo_ai_accepts_common_truthy_spellings_only() {
        for (value, expected) in [
            ("1", true),
            ("TRUE", true),
            ("yes", true),
            ("0", false),
            ("no", false),
            ("maybe", false),
        ] {
            assert_eq!(
                config(&[("HANDS_OFF_DEMO_AI", value)]).unwrap().demo_ai,
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn blank_values_fall_back_to_defaults() {
        let cfg = config(&[("HANDS_OFF_BIND", "  "), ("OPENROUTER_API_KEY", "")]).unwrap();
        assert_eq!(cfg.bind.to_string(), DEFAULT_BIND);
        assert!(cfg.openrouter_api_key.is_none());
    }

    #[test]
    fn invalid_values_name_the_variable() {
        for (name, value) in [
            ("HANDS_OFF_BIND", "nonsense"),
            ("HANDS_OFF_MAX_UPLOAD_MB", "abc"),
            ("HANDS_OFF_MAX_UPLOAD_MB", "0"),
            ("HANDS_OFF_MAX_DIMENSION", "-4"),
            ("HANDS_OFF_JOB_TIMEOUT_SECS", "0"),
        ] {
            let message = config(&[(name, value)]).unwrap_err().to_string();
            assert!(message.contains(name), "{message}");
        }
    }

    #[test]
    fn debug_output_never_contains_the_api_key() {
        let cfg = config(&[("OPENROUTER_API_KEY", "sk-super-secret")]).unwrap();
        assert!(!format!("{cfg:?}").contains("sk-super-secret"));
    }
}

//! Tracing setup.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

const DEFAULT_FILTER: &str = "info,hands_off_api=debug,tower_http=info";

/// Install the global tracing subscriber. Set `RUST_LOG` to change verbosity and
/// `HANDS_OFF_LOG_FORMAT=json` for structured output.
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE);
    if std::env::var("HANDS_OFF_LOG_FORMAT").is_ok_and(|v| v.eq_ignore_ascii_case("json")) {
        builder.json().init();
    } else {
        builder.init();
    }
}

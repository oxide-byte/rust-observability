use axum::http::StatusCode;
use axum::response::IntoResponse;
use opentelemetry::global;
use opentelemetry::KeyValue;
use tokio::time::{sleep, Duration};
use std::time::Instant;
use rand::Rng;
use tracing::{info, instrument};

// Create/get a histogram once per process. The underlying meter provider is set up in server::observability.
fn histogram() -> opentelemetry::metrics::Histogram<f64> {
    static INIT: std::sync::OnceLock<opentelemetry::metrics::Histogram<f64>> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        let meter = global::meter("rust_observability.api");
        meter
            .f64_histogram("api_demo_duration_ms")
            .with_description("Duration of /api/demo handler in milliseconds")
            .build()
    }).clone()
}

#[instrument(name = "api_demo_handler", skip_all)]
pub async fn handler() -> impl IntoResponse {
    let start = Instant::now();

    // random delay between 300ms and 800ms
    let delay_ms: u64 = rand::thread_rng().gen_range(300..=800);
    sleep(Duration::from_millis(delay_ms)).await;

    // coin flip for status
    let forbidden = rand::thread_rng().gen_bool(0.5);
    let status = if forbidden { StatusCode::FORBIDDEN } else { StatusCode::OK };

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let hist = histogram();
    let status_label = if forbidden { "forbidden" } else { "ok" };
    hist.record(
        elapsed_ms,
        &[KeyValue::new("endpoint", "/api/demo"), KeyValue::new("status", status_label)],
    );

    info!(delay_ms, status = status.as_u16(), "Handled /api/demo");

    (status, format!("demo: {delay_ms} ms\n"))
}
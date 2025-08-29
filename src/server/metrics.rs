use crate::server::observability::{render_otel_metrics, render_process_metrics};
use axum::response::IntoResponse;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn get_metrics() -> impl IntoResponse {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    println!("[scrape] /metrics requested at unix_ts={ts:.3}");

    let process_metrics = render_process_metrics();
    let otel_metrics = render_otel_metrics();

    // Concatenate all non-empty groups
    let mut parts = Vec::new();
    if !process_metrics.is_empty() {
        parts.push(process_metrics);
    }
    if !otel_metrics.is_empty() {
        parts.push(otel_metrics);
    }
    parts.join("\n")
}
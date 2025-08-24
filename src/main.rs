use axum::Router;
use rust_observability::server::observability::{render_process_metrics, render_otel_metrics, setup_observability, http_metrics_middleware};
use rust_observability::api;
use std::time::{SystemTime, UNIX_EPOCH};
use rust_observability::server::graceful_shutdown::graceful_shutdown;
use rust_observability::server::tracing::init_tracing_logging;

#[tokio::main]
async fn main() {
    // Initialize structured logging for tracing (logs are NOT sent to Prometheus; they go to stdout)
    init_tracing_logging();

    setup_observability();
    
    let app = Router::new()
        .merge(api::router())
        .route(
            "/metrics",
            axum::routing::get(move || async move {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                println!("[scrape] /metrics requested at unix_ts={ts:.3}");

                let process_metrics = render_process_metrics();
                let otel_metrics = render_otel_metrics();

                // Concatenate all non-empty groups
                let mut parts = Vec::new();
                if !process_metrics.is_empty() { parts.push(process_metrics); }
                if !otel_metrics.is_empty() { parts.push(otel_metrics); }
                parts.join("\n")
            }),
        )
        .layer(axum::middleware::from_fn(http_metrics_middleware));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    println!("Metrics server listening on {}", listener.local_addr().unwrap());
    println!("Server can be stopped by CTRL-C");
    axum::serve(listener, app)
        .with_graceful_shutdown(graceful_shutdown())
        .await
        .unwrap();
}
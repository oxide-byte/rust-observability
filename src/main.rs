use axum::Router;
use rust_observability::server::observability::{render_process_metrics, render_otel_metrics, setup_observability};
use rust_observability::api;
use std::time::{SystemTime, UNIX_EPOCH};
use rust_observability::server::graceful_shutdown::graceful_shutdown;

#[tokio::main]
async fn main() {
    let (prometheus_layer, metric_handle) = setup_observability();
    
    let app = Router::new()
        .merge(api::router())
        .route(
            "/metrics",
            axum::routing::get(move || async move {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                println!("[scrape] /metrics requested at unix_ts={:.3}", ts);

                let http_metrics = metric_handle.0.render();
                let process_metrics = render_process_metrics();
                let otel_metrics = render_otel_metrics();

                // Concatenate all non-empty groups
                let mut parts = Vec::new();
                if !http_metrics.is_empty() { parts.push(http_metrics); }
                if !process_metrics.is_empty() { parts.push(process_metrics); }
                if !otel_metrics.is_empty() { parts.push(otel_metrics); }
                parts.join("\n")
            }),
        )
        .layer(prometheus_layer);

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
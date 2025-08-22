use axum::Router;
use rust_observability::server::observability::{render_process_metrics, setup_observability};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    let (prometheus_layer, metric_handle) = setup_observability();
    
    let app = Router::new()
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
                if http_metrics.is_empty() {
                    process_metrics
                } else if process_metrics.is_empty() {
                    http_metrics
                } else {
                    format!("{}\n{}", http_metrics, process_metrics)
                }
            }),
        )
        .layer(prometheus_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    println!(
        "Metrics server listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}
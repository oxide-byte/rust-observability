use axum::Router;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use rust_observability::server::graceful_shutdown::graceful_shutdown;
use rust_observability::server::observability::{
    http_metrics_middleware, setup_observability,
};
use rust_observability::server::tracing::init_tracing_logging;
use rust_observability::{api, server};

#[tokio::main]
async fn main() {
    // Initialize structured logging for tracing (logs are NOT sent to Prometheus; they go to stdout)
    init_tracing_logging();

    setup_observability();

    let app = Router::new()
        .layer(OtelInResponseLayer::default()) // Contains no Spawn:
        .merge(server::router())
        .layer(OtelAxumLayer::default()) // Contains Spawn: "_spans":["api_demo_handler"]
        .merge(api::router())
        .layer(axum::middleware::from_fn(http_metrics_middleware));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    println!(
        "Metrics server listening on {}",
        listener.local_addr().unwrap()
    );
    println!("Server can be stopped by CTRL-C");
    axum::serve(listener, app)
        .with_graceful_shutdown(graceful_shutdown())
        .await
        .unwrap();
}
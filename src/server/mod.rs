use axum::routing::get;
use axum::Router;

pub mod graceful_shutdown;
pub mod observability;
pub mod tracing;
pub mod health;
pub mod metrics;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health::get_health))
        .route("/metrics", get(metrics::get_metrics))
}
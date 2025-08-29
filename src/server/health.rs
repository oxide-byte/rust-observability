use axum::response::IntoResponse;
use serde_json::json;
use tracing::log::{log, Level};

pub async fn get_health() -> impl IntoResponse {
    log!(Level::Info, "health check");
    axum::Json(json!({ "status" : "UP" }))
}
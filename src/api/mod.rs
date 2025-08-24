use axum::{routing::get, Router};

mod demo;

pub fn router() -> Router {
    Router::new().route("/api/demo", get(demo::handler))
}

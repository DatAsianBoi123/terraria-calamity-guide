use axum::{routing::get, Router};
use tower_http::services::ServeDir;

pub mod route;

pub fn app() -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("static/public"))
        .route("/invite", get(route::invite))
}


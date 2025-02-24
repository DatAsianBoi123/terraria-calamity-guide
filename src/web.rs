use std::sync::Arc;

use axum::{routing::get, Router};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::loadout_data::LoadoutData;

pub mod route;

pub fn app(loadouts: Arc<RwLock<LoadoutData>>) -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("static/public"))
        .route("/invite", get(route::invite))
        .route("/api/loadout/:class/:stage", get(route::loadout))
        .with_state(loadouts)
}


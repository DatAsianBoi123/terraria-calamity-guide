use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, response::{IntoResponse, Redirect, Response}, Json};
use poise::serenity_prelude::UserId;
use tokio::sync::RwLock;

use crate::{loadout_data::{CalamityClass, FullLoadout, LoadoutData, Stage}, playthrough_data::PlaythroughData};

pub async fn invite() -> Redirect {
    Redirect::to("https://discord.com/api/oauth2/authorize?client_id=1128716845365596273&permissions=274878171136&scope=bot%20applications.commands")
}

pub async fn loadout(Path((class, stage)): Path<(CalamityClass, Stage)>, State(loadouts): State<Arc<RwLock<LoadoutData>>>) -> Response {
    let loadouts = loadouts.read().await;
    loadouts.get_stage(stage)
        .map(|stage| Json(FullLoadout::new(stage, class)).into_response())
        .unwrap_or(StatusCode::NOT_FOUND.into_response())
}


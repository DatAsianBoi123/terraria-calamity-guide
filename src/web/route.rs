use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, response::{IntoResponse, Redirect, Response}, Json};
use linked_hash_map::LinkedHashMap;
use poise::serenity_prelude::UserId;
use reqwest::Url;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::{loadout_data::{CalamityClass, Loadout, LoadoutData, PotionType, Powerup, Stage, StageData}, playthrough_data::PlaythroughData};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ApiLoadout<'a> {
    pub class: String,
    pub stage: String,
    pub stage_img: Url,

    pub potion: PotionType,
    pub powerups: Option<&'a Vec<Powerup>>,
    pub armor: &'a str,
    pub weapons: &'a [String; 4],
    pub equipment: &'a Vec<String>,
    pub extra: &'a LinkedHashMap<String, Vec<String>>,
}

impl<'a> ApiLoadout<'a> {
    pub fn new(StageData { potion, powerups, loadouts }: &'a StageData, class: CalamityClass, stage: Stage) -> Option<Self> {
        let Loadout { armor, weapons, equipment, extra, .. } = loadouts.get(&class)?;
        Some(Self {
            class: class.to_string(),
            stage: stage.to_string(),
            stage_img: stage.img(),

            potion: *potion,
            powerups: powerups.as_ref(),
            armor,
            weapons,
            equipment,
            extra,
        })
    }
}

pub async fn invite() -> Redirect {
    Redirect::to("https://discord.com/api/oauth2/authorize?client_id=1128716845365596273&permissions=274878171136&scope=bot%20applications.commands")
}

pub async fn loadout(Path((class, stage)): Path<(CalamityClass, Stage)>, State(loadouts): State<Arc<RwLock<LoadoutData>>>) -> Response {
    let loadouts = loadouts.read().await;
    loadouts.get_stage(stage)
        .map(|stage_data| Json(ApiLoadout::new(stage_data, class, stage)).into_response())
        .unwrap_or(StatusCode::NOT_FOUND.into_response())
}

pub async fn playthrough(Path(id): Path<UserId>, State(playthroughs): State<Arc<RwLock<PlaythroughData>>>) -> Response {
    let playthroughs = playthroughs.read().await;
    playthroughs.active_playthroughs.get(&id)
        .map(|playthrough| Json(playthrough).into_response())
        .unwrap_or(StatusCode::NOT_FOUND.into_response())
}


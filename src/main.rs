#![deny(unused_crate_dependencies)]

use std::{path::PathBuf, fs};

use commands::db::db;
use loadout_data::LoadoutData;
use poise::{samples::register_globally, FrameworkOptions, serenity_prelude::{Activity, OnlineStatus}};
use serenity::prelude::{GatewayIntents, TypeMapKey};

use shuttle_poise::ShuttlePoise;
use shuttle_runtime::CustomError;
use shuttle_secrets::SecretStore;

use sqlx::{PgPool, Executor};
use tracing::info;

use crate::{commands::{ping::ping, help::help, send_rules::send_rules, view_loadout::view_loadout, view_item::view_item, playthrough::playthrough}, playthrough_data::PlaythroughData};

mod loadout_data;
mod playthrough_data;
mod commands;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result = std::result::Result<(), Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    loadouts: LoadoutData,
    pool: PgPool,
}

impl TypeMapKey for Data {
    type Value = PlaythroughData;
}

#[macro_export]
macro_rules! str {
    ($s: literal) => {
        $s.to_string()
    };
}

#[shuttle_runtime::main]
async fn poise(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_static_folder::StaticFolder] static_folder: PathBuf,
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://DatAsianBoi123:{secrets.NEON_PASS}@ep-rough-star-70439200.us-east-2.aws.neon.tech/neondb"
    )] pool: PgPool,
) -> ShuttlePoise<Data, Error> {
    let token = secret_store.get("TOKEN").expect("TOKEN not found");

    let mut schema = static_folder.clone();
    schema.push("schema.sql");
    let schema = fs::read_to_string(schema).expect("file exists");
    pool.execute(&schema[..]).await.map_err(CustomError::new)?;

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                ping(),
                view_loadout(),
                help(),
                send_rules(),
                playthrough(),
                db(),
            ],
            ..Default::default()
        })
        .token(token)
        .intents(GatewayIntents::GUILDS)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                ctx.set_presence(Some(Activity::playing("TModLoader")), OnlineStatus::Online).await;
                {
                    ctx.data.write().await.insert::<Data>(PlaythroughData::load(&pool).await);
                }
                info!("ready! logged in as {}", ready.user.tag());
                info!("loaded {} playthroughs", ctx.data.read().await.get::<Data>().expect("contains data").active_playthroughs.len());
                Ok(Data {
                    loadouts: loadout_data::load_data(static_folder.clone()),
                    pool
                })
            })
        }).build().await.map_err(CustomError::new)?;

    Ok(framework.into())
}

pub fn bulleted<S>(vec: &Vec<S>) -> String
where
    S: ToString,
{
    if vec.len() == 1 { return vec[0].to_string() }
    vec.iter()
        .map(|e| str!("- ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}

pub fn bulleted_iter<S, I>(iter: I) -> String
where
    S: ToString,
    I: Iterator<Item = S>,
{
    iter
        .map(|e| str!("- ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}

pub fn bulleted_array<S>(array: &[S]) -> String
where
    S: ToString,
{
    if array.len() == 1 { return array[0].to_string() }
    array.iter()
        .map(|e| str!("- ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}


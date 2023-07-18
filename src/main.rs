use std::path::PathBuf;

use data::LoadoutData;
use poise::{samples::register_globally, FrameworkOptions, serenity_prelude::{Activity, OnlineStatus}};
use serenity::prelude::GatewayIntents;

use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

use tracing::info;

use crate::commands::{ping::ping, help::help, send_rules::send_rules, view_loadout::view_loadout};

mod data;
mod commands;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result = std::result::Result<(), Error>;
pub type Context<'a> = poise::Context<'a, LoadoutData, Error>;

#[macro_export]
macro_rules! str {
    ($s: literal) => {
        $s.to_string()
    };
}

#[macro_export]
macro_rules! str_slice {
    [$($s: literal),*] => {
        [
            $($s.to_string(),)*
        ]
    }
}

#[macro_export]
macro_rules! str_vec {
    [$($s: literal),*] => {
        vec![$($s.to_string(),)*]
    }
}

#[shuttle_runtime::main]
async fn poise(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_static_folder::StaticFolder] static_folder: PathBuf,
) -> ShuttlePoise<LoadoutData, Error> {
    let token = secret_store.get("TOKEN").expect("TOKEN not found");

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                ping(),
                view_loadout(),
                help(),
                send_rules(),
            ],
            ..Default::default()
        })
        .token(token)
        .intents(GatewayIntents::GUILDS)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                ctx.set_presence(Some(Activity::playing("TModLoader")), OnlineStatus::Online).await;
                info!("ready! logged in as {}", ready.user.tag());
                Ok(data::load_data(static_folder))
            })
        }).build().await.map_err(shuttle_runtime::CustomError::new)?;

    Ok(framework.into())
}


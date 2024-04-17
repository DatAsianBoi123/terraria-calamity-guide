use std::{sync::Arc, time::Instant, fs::File};

use poise::command;
use tokio::sync::RwLock;

use crate::{Context, PoiseResult, playthrough_data::PlaythroughData, issue::Issues, Playthroughs, IssueData, loadout_data::LoadoutData, Loadouts};

#[command(slash_command, subcommands("sync", "reset_loadouts"), default_member_permissions = "MANAGE_GUILD", owners_only)]
pub async fn db(_: Context<'_>) -> PoiseResult {
    Ok(())
}

#[command(slash_command)]
async fn sync(ctx: Context<'_>) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let before = Instant::now();
    {
        let pool = &ctx.data().pool;
        let loadouts = LoadoutData::load(pool);
        let playthroughs = PlaythroughData::load(pool);
        let issues = Issues::load(ctx.http(), pool);

        let (loadouts, playthroughs, issues) = tokio::join!(loadouts, playthroughs, issues);

        let mut data_lock = ctx.serenity_context().data.write().await;

        data_lock.insert::<Loadouts>(Arc::new(RwLock::new(loadouts)));
        data_lock.insert::<Playthroughs>(Arc::new(RwLock::new(playthroughs)));
        data_lock.insert::<IssueData>(Arc::new(RwLock::new(issues)));
    }

    ctx.say(format!("Successfully synced with database ({}ms)", (Instant::now() - before).as_millis())).await?;
    Ok(())
}

#[command(slash_command, rename = "resetloadouts")]
async fn reset_loadouts(ctx: Context<'_>) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    {
        let mut lock = ctx.serenity_context().data.write().await;
        let pool = &ctx.data().pool;
        LoadoutData::reset(pool).await;
        let loadouts = LoadoutData::from_file(File::open("static/loadout_data.json").expect("file exists")).expect("valid json");
        loadouts.save(pool).await;
        // HACK: this is the only easy way IDs are able to get updated bc json file doesn't contain
        // loadout ids
        let loadouts = LoadoutData::load(pool).await;
        lock.insert::<Loadouts>(Arc::new(RwLock::new(loadouts)));
    }

    ctx.say("Rolled back loadouts").await?;
    Ok(())
}


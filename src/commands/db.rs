use std::{time::Instant, fs::File};

use poise::command;

use crate::{Context, PoiseResult, playthrough_data::PlaythroughData, issue::Issues, loadout_data::LoadoutData};

#[command(slash_command, subcommands("sync", "reset_loadouts"), default_member_permissions = "MANAGE_GUILD", owners_only)]
pub async fn db(_: Context<'_>) -> PoiseResult {
    Ok(())
}

#[command(slash_command)]
async fn sync(ctx: Context<'_>) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let data = &ctx.data();
    let pool = &ctx.data().pool;
    let loadouts = LoadoutData::load(pool);
    let playthroughs = PlaythroughData::load(pool);
    let issues = Issues::load(ctx.http(), pool);

    let before = Instant::now();

    let (loadouts, playthroughs, issues) = tokio::join!(loadouts, playthroughs, issues);

    {
        let mut loadouts_write = data.loadouts.write().await;
        *loadouts_write = loadouts;
    };
    {
        let mut playthroughs_write = data.playthroughs.write().await;
        *playthroughs_write = playthroughs;
    };
    {
        let mut issues_write = data.issues.write().await;
        *issues_write = issues;
    };

    ctx.say(format!("Successfully synced with database ({}ms)", (Instant::now() - before).as_millis())).await?;
    Ok(())
}

#[command(slash_command, rename = "resetloadouts")]
async fn reset_loadouts(ctx: Context<'_>) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let pool = &ctx.data().pool;
    LoadoutData::reset(pool).await;
    let loadouts = LoadoutData::from_file(File::open("static/loadout_data.json").expect("file exists")).expect("valid json");
    loadouts.save(pool).await;
    // HACK: this is the only easy way IDs are able to get updated bc json file doesn't contain
    // loadout ids
    let loadouts = LoadoutData::load(pool).await;
    {
        let mut loadout_write = ctx.data().loadouts.write().await;
        *loadout_write = loadouts;
    }

    ctx.say("Rolled back loadouts").await?;
    Ok(())
}


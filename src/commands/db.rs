use std::sync::Arc;

use poise::command;
use tokio::sync::RwLock;

use crate::{Context, Result, playthrough_data::PlaythroughData, issue::Issues, Playthroughs, IssueData};

#[command(slash_command, subcommands("sync"), default_member_permissions = "MANAGE_GUILD", owners_only)]
pub async fn db(_: Context<'_>) -> Result {
    Ok(())
}

#[command(slash_command, default_member_permissions = "MANAGE_GUILD")]
async fn sync(ctx: Context<'_>) -> Result {
    ctx.defer_ephemeral().await?;

    let mut data_lock = ctx.serenity_context().data.write().await;
    let pool = &ctx.data().pool;
    let playthroughs = PlaythroughData::load(pool).await;
    let issues = Issues::load(ctx.http(), pool).await;
    data_lock.insert::<Playthroughs>(Arc::new(RwLock::new(playthroughs)));
    data_lock.insert::<IssueData>(Arc::new(RwLock::new(issues)));

    let playthrough_data = data_lock.get::<Playthroughs>().expect("has playthroughs").clone();
    let playthrough_data = &playthrough_data.read().await;
    ctx.say(format!("Successfully synced with database\ntotal playthroughs is now {}", playthrough_data.active_playthroughs.len())).await?;
    Ok(())
}


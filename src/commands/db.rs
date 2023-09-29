use poise::command;

use crate::{Context, Result, Data, playthrough_data::PlaythroughData, MutableData, issue::Issues};

#[command(slash_command, subcommands("sync"), default_member_permissions = "MANAGE_GUILD", owners_only)]
pub async fn db(_: Context<'_>) -> Result {
    Ok(())
}

#[command(slash_command, default_member_permissions = "MANAGE_GUILD")]
async fn sync(ctx: Context<'_>) -> Result {
    ctx.defer_ephemeral().await?;
    let mut data_lock = ctx.serenity_context().data.write().await;
    let playthroughs = PlaythroughData::load(&ctx.data().pool).await;
    let issues = Issues::load(ctx.http(), &ctx.data().pool).await;
    data_lock.insert::<Data>(MutableData {
        playthroughs,
        issues,
    });
    let playthrough_data = &data_lock.get::<Data>().expect("has playthroughs").playthroughs;
    ctx.say(format!("Successfully synced with database\ntotal playthroughs is now {}", playthrough_data.active_playthroughs.len())).await?;
    Ok(())
}


use poise::command;

use crate::{Context, Result, Data, playthrough_data::PlaythroughData};

#[command(slash_command, subcommands("sync"), default_member_permissions = "MANAGE_GUILD")]
pub async fn db(_: Context<'_>) -> Result {
    Ok(())
}

#[command(slash_command, default_member_permissions = "MANAGE_GUILD")]
async fn sync(ctx: Context<'_>) -> Result {
    ctx.defer_ephemeral().await?;
    let mut data_lock = ctx.serenity_context().data.write().await;
    data_lock.insert::<Data>(PlaythroughData::load(&ctx.data().pool).await);
    let playthrough_data = data_lock.get::<Data>().expect("has playthroughs");
    ctx.say(format!("Successfully synced with database\ntotal playthroughs is now {}", playthrough_data.active_playthroughs.len())).await?;
    Ok(())
}


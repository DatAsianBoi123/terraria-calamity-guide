use poise::{command, serenity_prelude::CreateMessage};

use crate::{Context, Result, loadout_data::{CalamityClass, Stage}, IssueData};

#[command(slash_command, description_localized("en-US", "Reports a problem with a loadout"), ephemeral)]
pub async fn report(
    ctx: Context<'_>,
    #[description = "The class that the issue is in"] class: CalamityClass,
    #[description = "The stage that the issue is in"] stage: Stage,
    #[description = "The incorrect phrase"] incorrect: String,
    #[description = "The phrase that should replace the incorrect one"] correct: String,
) -> Result {
    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<IssueData>().expect("data exists").clone();
    let mut issues = data_lock.write().await;
    let issue = issues.create(ctx.author(), class, stage, incorrect, correct, &ctx.data().pool).await;

    ctx.data().issue_channel.send_message(ctx, CreateMessage::new()
        .embed(issue.create_embed())
        .components(issue.create_components()))
        .await?;

    ctx.say("Successfully reported your issue!").await?;

    Ok(())
}


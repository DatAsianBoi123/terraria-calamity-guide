use poise::command;

use crate::{Context, Result, loadout_data::{CalamityClass, Stage}, Data};

#[command(slash_command, description_localized("en-US", "Reports a problem with a loadout"), ephemeral)]
pub async fn report(
    ctx: Context<'_>,
    #[description = "The class that the issue is in"] class: CalamityClass,
    #[description = "The stage that the issue is in"] stage: Stage,
    #[description = "The incorrect phrase"] incorrect: String,
    #[description = "The phrase that should replace the incorrect one"] correct: String,
) -> Result {
    let mut issues = ctx.serenity_context().data.write().await;
    let issues = &mut issues.get_mut::<Data>().expect("data exists").issues;
    let issue = issues.create(ctx.author(), class, stage, incorrect, correct, &ctx.data().pool).await;

    ctx.data().issue_channel.send_message(ctx, |c| c.set_embed(issue.create_embed()).set_components(issue.create_components())).await?;

    ctx.say("Successfully reported your issue!").await?;

    Ok(())
}


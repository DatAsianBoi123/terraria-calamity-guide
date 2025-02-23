use poise::{command, CreateReply};

use crate::{Context, PoiseResult, loadout_data::{CalamityClass, Stage}};

#[command(slash_command, subcommands("view"), description_localized("en-US", "Loadout commands"))]
pub async fn loadout(_: Context<'_>) -> PoiseResult {
    Ok(())
}

#[command(slash_command, description_localized("en-US", "Views the recommended loadout during a specific stage of progression"))]
async fn view(
    ctx: Context<'_>,
    #[description = "The class"] class: CalamityClass,
    #[description = "The stage of the game"] stage: Option<Stage>,
) -> PoiseResult {
    let stage = stage.unwrap_or(Stage::PreBoss);
    let loadout_data = ctx.data().loadouts.read().await;
    if let Some(stage_data) = loadout_data.get_stage(stage) {
        ctx.send(CreateReply::default().embed(stage_data.create_embed(ctx.author(), class, stage))).await?;
    } else {
        ctx.say("No loadout found! Please report this!").await?;
    }
    Ok(())
}


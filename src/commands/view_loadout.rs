use poise::command;

use crate::{Context, Result, loadout_data::{CalamityClass, Stage}};

#[command(
    slash_command,
    description_localized("en-US", "Views the recommended loadout during a specific stage of progression"),
    rename = "viewloadout"
)]
pub async fn view_loadout(
    ctx: Context<'_>,
    #[description = "The class"] class: CalamityClass,
    #[description = "The stage of the game"] stage: Option<Stage>,
) -> Result {
    let stage = stage.unwrap_or(Stage::PreBoss);
    if let Context::Application(ctx) = ctx {
        let stage_data = ctx.data().loadouts.get(&stage).expect("stage exists");
        ctx.send(|f| f.embed(|e| stage_data.format_embed(e, ctx.author(), class, stage))).await?;
    }
    Ok(())
}


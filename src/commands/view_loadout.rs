use poise::{command, CreateReply};

use crate::{Context, PoiseResult, loadout_data::{CalamityClass, Stage}};

#[command(
    slash_command,
    description_localized("en-US", "Views the recommended loadout during a specific stage of progression"),
    rename = "viewloadout"
)]
pub async fn view_loadout(
    ctx: Context<'_>,
    #[description = "The class"] class: CalamityClass,
    #[description = "The stage of the game"] stage: Option<Stage>,
) -> PoiseResult {
    let stage = stage.unwrap_or(Stage::PreBoss);
    if let Context::Application(ctx) = ctx {
        let stage_data = ctx.data().loadouts.get(&stage).expect("stage exists");
        ctx.send(CreateReply::default().embed(stage_data.create_embed(ctx.author(), class, stage))).await?;
    }
    Ok(())
}


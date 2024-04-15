use poise::{command, CreateReply};

use crate::{Context, PoiseResult, loadout_data::{CalamityClass, Stage}, Loadouts};

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
        let data_lock = ctx.serenity_context().data.read().await;
        let loadouts = data_lock.get::<Loadouts>().expect("loadout data exists").read().await;
        if let Some(stage_data) = loadouts.get_stage(stage) {
            ctx.send(CreateReply::default().embed(stage_data.create_embed(ctx.author(), class, stage))).await?;
        } else {
            ctx.say("No loadout found! Please report this!").await?;
        }
    }
    Ok(())
}

